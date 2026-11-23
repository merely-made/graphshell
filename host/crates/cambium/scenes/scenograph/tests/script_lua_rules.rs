// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The Scenograph rule runner on real Lua (Scenograph editor plan, SE92):
//! `Condition::Script` conditions evaluated by [`LuaValueEngine`] through the
//! `ValueEngine` contract, with scenograph linking no engine itself.
//!
//! The rules are Lua as the Lua evaluator plan rules it: a fact that may be
//! missing is guarded and `nil` is no match (D6); maths is Lua's own `math`
//! library, with no host maths registered (D17). `states` is a set, so a state
//! not held reads as `nil` (scenograph-editor D1).

use std::collections::{BTreeMap, BTreeSet};

use scenograph::presentation::{
    Condition, Context, Effect, PRESENTATION_RULE_VERSION, PrimitiveKind, Property, Rule, RuleSet,
    Scalar, Target, ViewState,
};
use script_didge::LuaValueEngine;

fn rule(id: &str, priority: i32, source: &str, details: bool) -> Rule {
    Rule {
        id: id.into(),
        label: id.into(),
        enabled: true,
        priority,
        target: Target {
            kind: PrimitiveKind::Node,
            ids: BTreeSet::new(),
        },
        condition: Condition::Script {
            source: source.into(),
        },
        effects: vec![Effect::Details { visible: details }],
    }
}

fn set(rules: Vec<Rule>) -> RuleSet {
    RuleSet {
        version: PRESENTATION_RULE_VERSION,
        rules,
    }
}

fn node(id: &str) -> Context {
    Context {
        id: id.into(),
        kind: PrimitiveKind::Node,
        facts: BTreeMap::new(),
        states: BTreeSet::new(),
        screen_size: Some(400.0),
        previous_matches: BTreeSet::new(),
    }
}

fn engine() -> LuaValueEngine {
    LuaValueEngine::new()
}

fn winner(set: &RuleSet, context: &Context, engine: &mut LuaValueEngine) -> Option<String> {
    set.resolve_with_scripts(context, engine)
        .unwrap()
        .winners
        .get(&Property::Details)
        .map(|decision| decision.rule_id.clone())
}

#[test]
fn a_rule_reads_facts_and_states() {
    let rules = set(vec![
        rule("heavy", 1, "facts.weight and facts.weight > 2.0", true),
        rule("picked", 2, "states.selected", true),
        rule("image", 3, r#"facts.mime == "image/png""#, true),
    ]);
    let mut item = node("a");
    item.facts.insert("weight".into(), Scalar::Number(2.5));
    assert_eq!(winner(&rules, &item, &mut engine()).as_deref(), Some("heavy"));
    item.facts.insert("weight".into(), Scalar::Number(1.0));
    assert_eq!(winner(&rules, &item, &mut engine()), None);
    item.states.insert(ViewState::Selected);
    assert_eq!(winner(&rules, &item, &mut engine()).as_deref(), Some("picked"));
    item.facts
        .insert("mime".into(), Scalar::Text("image/png".into()));
    assert_eq!(winner(&rules, &item, &mut engine()).as_deref(), Some("image"));
}

#[test]
fn a_guarded_missing_fact_is_a_traced_non_match_and_an_unguarded_one_an_error() {
    let rules = set(vec![
        rule("unguarded", 1, "facts.weight > 2.0", true),
        rule("guarded", 0, "facts.weight and facts.weight > 2.0", true),
    ]);
    let resolved = rules
        .resolve_with_scripts(&node("a"), &mut engine())
        .unwrap();
    assert!(resolved.matched_rules.is_empty());
    // Lua raises comparing nil with a number: an unguarded rule is an error
    // whose trace names the cause (D6).
    assert!(
        resolved.traces[0]
            .reason
            .starts_with("script: script error"),
        "{}",
        resolved.traces[0].reason
    );
    assert!(resolved.traces[0].reason.contains("compare"));
    // Guarded, the missing fact is nil, and nil is a quiet non-match, like the
    // declarative conditions' "fact: missing". Negating it matches, as `Not`
    // does there.
    assert_eq!(resolved.traces[1].reason, "script: no value");
    let negated = set(vec![rule(
        "not-heavy",
        1,
        "not (facts.weight and facts.weight > 2.0)",
        true,
    )]);
    let resolved = negated
        .resolve_with_scripts(&node("a"), &mut engine())
        .unwrap();
    assert!(resolved.matched_rules.contains("not-heavy"));
}

#[test]
fn a_script_runs_the_hysteresis_the_declarative_condition_has() {
    // Compact below 100 pixels; back to detail only above 140.
    let compact = set(vec![rule(
        "compact",
        1,
        "if previously_matched then return screen_size < 140.0 else return screen_size <= 100.0 end",
        false,
    )]);
    let at = |size: f64, was: bool| {
        let mut item = node("a");
        item.screen_size = Some(size);
        if was {
            item.previous_matches.insert("compact".into());
        }
        compact
            .resolve_with_scripts(&item, &mut engine())
            .unwrap()
            .matched_rules
            .contains("compact")
    };
    assert!(at(90.0, false));
    assert!(!at(120.0, false), "does not enter between the thresholds");
    assert!(at(120.0, true), "does not leave between the thresholds");
    assert!(!at(150.0, true));
}

#[test]
fn rule_maths_is_luas_own_and_matches_libm() {
    // exp(-d*d) falls off with distance; the threshold is the libm value.
    let falloff = set(vec![rule(
        "near",
        1,
        "math.exp(-facts.d * facts.d) > 0.5",
        true,
    )]);
    let mut near = node("a");
    near.facts.insert("d".into(), Scalar::Number(0.8));
    assert_eq!(winner(&falloff, &near, &mut engine()).as_deref(), Some("near"));
    near.facts.insert("d".into(), Scalar::Number(0.9));
    assert_eq!(winner(&falloff, &near, &mut engine()), None);
    assert!(libm::exp(-0.8 * 0.8) > 0.5 && libm::exp(-0.9 * 0.9) < 0.5);
}

#[test]
fn a_rule_calling_an_unbound_name_errors_and_the_other_rules_still_resolve() {
    // A shared scene's rules run with fewer bindings (SE36); a rule that calls
    // a host function this engine was not given fails on its own.
    let rules = set(vec![
        rule("host-maths", 9, "sin(1.0) > 0.0", true),
        rule("plain", 0, "true", false),
    ]);
    let resolved = rules
        .resolve_with_scripts(&node("a"), &mut engine())
        .unwrap();
    assert!(
        resolved.traces[0]
            .reason
            .starts_with("script: script error")
    );
    assert_eq!(resolved.winners[&Property::Details].rule_id, "plain");
}

#[test]
fn a_runaway_rule_is_stopped_by_the_budget() {
    let rules = set(vec![
        rule("forever", 9, "while true do end", true),
        rule("plain", 0, "true", false),
    ]);
    let mut engine = engine();
    let resolved = rules.resolve_with_scripts(&node("a"), &mut engine).unwrap();
    assert_eq!(
        resolved.traces[0].reason,
        "script: the script ran past its budget"
    );
    assert_eq!(resolved.winners[&Property::Details].rule_id, "plain");
    // And the engine is still good for the next occurrence.
    assert!(winner(&rules, &node("b"), &mut engine).is_some());
}

#[test]
fn a_script_cannot_carry_state_from_one_occurrence_to_the_next() {
    let rules = set(vec![rule(
        "leak",
        1,
        r#"seen = facts; return id == "first""#,
        true,
    )]);
    let mut engine = engine();
    assert_eq!(
        winner(&rules, &node("first"), &mut engine).as_deref(),
        Some("leak")
    );
    assert_eq!(winner(&rules, &node("second"), &mut engine), None);
    // A global set by an earlier occurrence is gone in the next one.
    let probe = set(vec![rule("probe", 1, "seen == nil", true)]);
    assert_eq!(
        winner(&probe, &node("third"), &mut engine).as_deref(),
        Some("probe")
    );
}

#[test]
fn the_same_occurrence_resolves_the_same_way_twice() {
    let rules = set(vec![rule("a", 1, "math.exp(facts.x) < 3.0", true)]);
    let mut item = node("n");
    item.facts.insert("x".into(), Scalar::Number(1.0));
    let first = rules.resolve_with_scripts(&item, &mut engine()).unwrap();
    let second = rules.resolve_with_scripts(&item, &mut engine()).unwrap();
    assert_eq!(first, second);
}

#[test]
fn composed_script_failures_leave_the_whole_rule_unmatched() {
    for source in ["missing_function()", "while true do end", "42"] {
        let script = Condition::Script {
            source: source.into(),
        };
        let negated = Condition::Not {
            condition: Box::new(script.clone()),
        };
        for condition in [
            script.clone(),
            negated.clone(),
            Condition::Not {
                condition: Box::new(Condition::All {
                    conditions: vec![Condition::Always, script.clone()],
                }),
            },
            Condition::All {
                conditions: vec![Condition::Always, negated],
            },
            Condition::Any {
                conditions: vec![Condition::Always, script],
            },
        ] {
            let mut broken = rule("broken", 9, source, true);
            broken.condition = condition.clone();
            let rules = set(vec![broken, rule("plain", 0, "true", false)]);
            let resolved = rules
                .resolve_with_scripts(&node("a"), &mut engine())
                .unwrap();
            assert!(!resolved.matched_rules.contains("broken"), "{condition:?}");
            assert_eq!(resolved.winners[&Property::Details].rule_id, "plain");
            assert!(resolved.traces[0].reason.contains("script:"));
            assert!(!resolved.traces[0].reason.ends_with("script: false"));
        }
    }
    // A genuine false value remains composable; it is not an evaluation error.
    let mut negated = rule("not-false", 9, "false", true);
    negated.condition = Condition::Not {
        condition: Box::new(negated.condition),
    };
    assert_eq!(
        winner(&set(vec![negated]), &node("a"), &mut engine()).as_deref(),
        Some("not-false")
    );
}
