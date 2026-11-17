// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

use super::*;
use sceno::Representation;
use scenograph::presentation::{
    Body, Condition, Effect, Extent, NodeMark, Paint, PrimitiveKind, Property, RelationForm, Rule,
    RuleSet, Silhouette, Target,
};
use scenograph::{FieldForm, MarkSlots};

fn mark(shape: Silhouette) -> NodeMark {
    NodeMark {
        body: Body::Silhouette(shape),
        paint: Paint::Bare,
        ..NodeMark::new(Representation::Glyph, [Extent::Assigned, Extent::Measured])
    }
}

fn rules(effects: Vec<Effect>) -> RuleSet {
    RuleSet {
        version: 1,
        rules: vec![Rule {
            id: "one-occurrence".into(),
            label: "One occurrence".into(),
            enabled: true,
            priority: 1,
            target: Target {
                kind: PrimitiveKind::Node,
                ids: ["a".into()].into(),
            },
            condition: Condition::Always,
            effects,
        }],
    }
}

#[test]
fn recipe_marks_reach_the_scene_and_rules_override_one_occurrence() {
    let mut recipe = definition("grid", "x");
    let default = mark(Silhouette::Diamond);
    let mut replacement = mark(Silhouette::Hexagon);
    replacement.detail = Representation::Snapshot;
    recipe.marks = MarkSlots {
        node: Some(default.clone()),
        relation: Some(RelationForm::Stroke),
        field: Some(FieldForm::Contour),
    };
    let rules = rules(vec![Effect::NodeMark {
        value: replacement.clone(),
    }]);
    let compiler = ProjectionCompiler::new(sizes(40.0, 20.0)).with_presentation(rules);
    let compiled = compiler.compile(&recipe, &dataset()).unwrap();
    assert_eq!(compiled.marks, recipe.marks);
    for id in ["a", "b", "c"] {
        let instance = compiled.instance_by_occurrence[id];
        let resolved = &compiled.node_marks[&instance];
        let expected = if id == "a" { &replacement } else { &default };
        assert_eq!(resolved.defaults.node.as_ref(), Some(&default));
        assert_eq!(resolved.marks.node.as_ref(), Some(expected));
        assert_eq!(
            compiled.score.items[instance.0 as usize].representation,
            expected.detail
        );
        assert_eq!(
            compiled.scene.items[instance.0 as usize].representation,
            expected.detail
        );
        assert_eq!(resolved.resolution.traces[0].matched, id == "a");
        if id == "a" {
            assert_eq!(
                resolved.resolution.winners[&Property::Representation].rule_id,
                "one-occurrence"
            );
        }
    }
    let json = recipe.to_json_bytes().unwrap();
    assert_eq!(
        serde_json::from_slice::<ProjectionDefinition>(&json).unwrap(),
        recipe
    );
}

#[test]
fn legacy_recipe_is_unchanged_and_paint_only_refresh_reuses_geometry() {
    let mut recipe = definition("grid", "x");
    let legacy_json = recipe.to_json_bytes().unwrap();
    assert!(
        !String::from_utf8(legacy_json.clone())
            .unwrap()
            .contains("marks")
    );
    let legacy: ProjectionDefinition = serde_json::from_slice(&legacy_json).unwrap();
    let compiler = ProjectionCompiler::new(sizes(40.0, 20.0));
    let before = compiler.compile(&recipe, &dataset()).unwrap();
    let read_back = compiler.compile(&legacy, &dataset()).unwrap();
    assert_eq!(before.score, read_back.score);
    assert_eq!(before.scene, read_back.scene);
    recipe.marks.node = Some(mark(Silhouette::Diamond));
    let first = compiler.compile(&recipe, &dataset()).unwrap();
    recipe.marks.node.as_mut().unwrap().paint = Paint::Derived;
    let refreshed = compiler.refresh(&first, &recipe, &dataset()).unwrap();
    assert!(refreshed.placement_reused);
    assert_eq!(first.scene, refreshed.scene);
    assert!(
        refreshed
            .node_marks
            .values()
            .all(|item| { item.marks.node.as_ref().unwrap().paint == Paint::Derived })
    );
}

#[test]
fn legacy_detail_effect_and_whole_mark_share_rule_precedence() {
    let mut recipe = definition("grid", "x");
    recipe.marks.node = Some(mark(Silhouette::Diamond));
    let mut presentation = rules(vec![Effect::NodeMark {
        value: mark(Silhouette::Hexagon),
    }]);
    let mut later = presentation.rules[0].clone();
    later.id = "later-detail".into();
    later.effects = vec![Effect::Representation {
        value: Representation::Card,
    }];
    presentation.rules.push(later);
    let compiled = ProjectionCompiler::new(sizes(40.0, 20.0))
        .with_presentation(presentation)
        .compile(&recipe, &dataset())
        .unwrap();
    let resolved = &compiled.node_marks[&compiled.instance_by_occurrence["a"]];
    let chosen = resolved.marks.node.as_ref().unwrap();
    assert_eq!(chosen.body, Body::Silhouette(Silhouette::Diamond));
    assert_eq!(chosen.detail, Representation::Card);
    assert!(resolved.resolution.traces[0].winning_properties.is_empty());
    assert_eq!(
        resolved.resolution.traces[1].winning_properties,
        [Property::Representation]
    );
}

#[test]
fn rule_marks_are_checked_against_the_arrangements_extent() {
    let mut invalid = mark(Silhouette::Hexagon);
    invalid.accepts = [Extent::Encoded].into();
    let compiler = ProjectionCompiler::new(sizes(40.0, 20.0))
        .with_presentation(rules(vec![Effect::NodeMark { value: invalid }]));
    let issues = compiler
        .compile(&definition("grid", "x"), &dataset())
        .unwrap_err();
    assert!(
        issues
            .iter()
            .any(|issue| issue.field == "occurrences[a].mark")
    );
}
