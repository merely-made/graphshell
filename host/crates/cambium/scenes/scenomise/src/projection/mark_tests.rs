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
        ..Default::default()
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

#[test]
fn assigned_marks_use_the_same_cells_as_tiles() {
    let compiler = ProjectionCompiler::new(sizes(40.0, 20.0));
    let dataset = dataset();
    for (kind, form) in [
        ("mosaic", None),
        ("partition", Some("tiles")),
        ("partition", Some("rings")),
    ] {
        let mut recipe = definition(kind, "lane");
        if let Some(form) = form {
            recipe
                .arrangement
                .options
                .insert("form".into(), form.into());
        }
        let legacy = compiler.compile(&recipe, &dataset).unwrap();
        let tile = NodeMark {
            body: Body::Silhouette(Silhouette::Square),
            paint: Paint::Bare,
            ..NodeMark::new(Representation::Glyph, [Extent::Assigned])
        };
        let mut reference = legacy.scene;
        for (index, mark) in [
            tile,
            NodeMark::card(),
            NodeMark::new(Representation::Snapshot, [Extent::Assigned]),
            NodeMark::new(
                Representation::Open {
                    kind: "host portrait".into(),
                },
                [Extent::Assigned],
            ),
        ]
        .into_iter()
        .enumerate()
        {
            recipe.marks.node = Some(mark.clone());
            let compiled = compiler.compile(&recipe, &dataset).unwrap();
            assert_eq!(compiled.score.items.len(), reference.items.len());
            for item in &mut reference.items {
                item.representation = mark.detail.clone();
            }
            // Only detail changes. In particular, the representation cannot
            // change the subdivision, grouping, contours or occurrence order.
            assert_eq!(compiled.scene, reference, "{kind} {form:?} mark {index}");
            assert!(
                compiled
                    .node_marks
                    .values()
                    .all(|resolved| resolved.marks.node.as_ref() == Some(&mark))
            );
        }
    }
}

#[test]
fn a_winning_assigned_mark_can_replace_a_measured_default() {
    let mut recipe = definition("partition", "lane");
    recipe.marks.node = Some(NodeMark::new(Representation::Glyph, [Extent::Measured]));
    let card = NodeMark::card();
    let mut presentation = rules(vec![Effect::NodeMark {
        value: card.clone(),
    }]);
    let mut data = dataset();
    let compiler =
        ProjectionCompiler::new(sizes(40.0, 20.0)).with_presentation(presentation.clone());
    let issues = compiler.compile(&recipe, &data).unwrap_err();
    assert!(
        issues
            .iter()
            .all(|issue| issue.field != "occurrences[a].mark")
    );
    assert_eq!(issues.len(), 2);
    presentation.rules[0].target.ids.clear();
    let compiler = ProjectionCompiler::new(sizes(40.0, 20.0)).with_presentation(presentation);
    let compiled = compiler.compile(&recipe, &data).unwrap();
    assert!(
        compiled
            .scene
            .items
            .iter()
            .all(|item| item.representation == Representation::Card)
    );
    assert!(
        compiled
            .node_marks
            .values()
            .all(|resolved| resolved.marks.node.as_ref() == Some(&card))
    );
    data.occurrences.clear();
    assert!(
        compiler
            .compile(&recipe, &data)
            .unwrap_err()
            .iter()
            .any(|issue| issue.field == "marks.node")
    );
}

#[test]
fn switching_subdivision_detail_preserves_an_adjusted_placement() {
    let compiler = ProjectionCompiler::new(sizes(40.0, 20.0));
    let data = dataset();
    for kind in ["mosaic", "partition"] {
        let mut recipe = definition(kind, "lane");
        recipe.marks.node = Some(mark(Silhouette::Square));
        let mut previous = compiler.compile(&recipe, &data).unwrap();
        previous.scene.items[0].transform.translate.x += 12.0;
        let card = NodeMark::card();
        recipe.marks.node = Some(card.clone());
        let refreshed = compiler.refresh(&previous, &recipe, &data).unwrap();
        assert!(refreshed.placement_reused, "{kind}");
        let mut expected = previous.scene;
        for item in &mut expected.items {
            item.representation = Representation::Card;
        }
        assert_eq!(refreshed.scene, expected, "{kind}");
    }
}

#[test]
fn context_rule_replaces_only_the_matching_occurrence() {
    use scenograph::contexts::{ContextPlacement, ContextScene, ContextTrigger, SubjectKind};

    let mut recipe = definition("grid", "x");
    let default = ContextScene {
        id: "linked".into(),
        subject: SubjectKind::Node,
        trigger: ContextTrigger::Hover,
        placement: ContextPlacement::Beside,
        input: "linked".into(),
        recipe: Box::new(recipe.clone()),
        commands: None,
    };
    let override_scene = ContextScene {
        id: "resource".into(),
        input: "resource".into(),
        trigger: ContextTrigger::Request,
        ..default.clone()
    };
    recipe.marks.contexts = vec![default.clone()];
    let rules = rules(vec![Effect::Contexts {
        value: vec![override_scene.clone()],
    }]);
    assert!(rules.validate().is_ok());
    let compiler = ProjectionCompiler::new(sizes(40.0, 20.0)).with_presentation(rules);
    let data = dataset();
    let compiled = compiler.compile(&recipe, &data).unwrap();
    assert_eq!(
        compiled.node_marks[&compiled.instance_by_occurrence["a"]]
            .marks
            .contexts,
        [override_scene]
    );
    assert_eq!(
        compiled.node_marks[&compiled.instance_by_occurrence["b"]]
            .marks
            .contexts,
        [default.clone()]
    );
    let child = crate::contexts::compile(&compiler, &default, &data).unwrap();
    assert_eq!(child.scene.items.len(), data.occurrences.len());
}
