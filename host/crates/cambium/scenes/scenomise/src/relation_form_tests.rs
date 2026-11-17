// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

use super::*;
use sceno::{InstanceId, RelationForm, Vec2};
use scenograph::presentation::{Condition, Effect, PrimitiveKind, Property, Rule, RuleSet, Target};

fn fixture(kind: &str, form: RelationForm) -> (HostDatasetV1, ProjectionDefinition) {
    let data = parse(&envelope()).unwrap();
    let mut recipe = definition(
        &data,
        kind,
        if kind == "grid" { "x" } else { "occurrence_id" },
    );
    recipe.encoding.y = Channel::Field("y".into());
    if kind == "partition" || kind == "mosaic" {
        recipe.encoding.x = Channel::Field("label".into());
        recipe.encoding.y = Channel::Field("x".into());
    }
    recipe.marks.relation = Some(form);
    (data, recipe)
}

fn request(data: &HostDatasetV1, id: &str, from: &str, to: &str) -> DisclosedRelationship {
    let mut relationship = data.relationships[0].clone();
    relationship.id = id.into();
    relationship.from_occurrence = from.into();
    relationship.to_occurrence = to.into();
    relationship
}

#[test]
fn same_relations_switch_from_strokes_to_directional_cells_without_new_occurrences() {
    let (mut data, mut recipe) = fixture("grid", RelationForm::Stroke);
    data.relationships.push(request(&data, "reverse", "b", "a"));
    let stroke = data.compile(&compiler(), &recipe).unwrap();
    recipe.marks.relation = Some(RelationForm::MatrixCell);
    let matrix = data.compile(&compiler(), &recipe).unwrap();
    assert_eq!(stroke.relationships, matrix.relationships);
    assert_eq!(stroke.projection.scene.items, matrix.projection.scene.items);
    assert_eq!(
        stroke.projection.scene.sources,
        matrix.projection.scene.sources
    );
    let cells = &matrix.projection.scene.relations;
    let forward = cells[0].cell_bounds().unwrap();
    let reverse = cells[1].cell_bounds().unwrap();
    assert!(forward.origin.x > reverse.origin.x);
    assert!(forward.origin.y < reverse.origin.y);
    assert!(cells.iter().all(|r| r.points.len() == 4));
    assert!(
        stroke
            .projection
            .scene
            .relations
            .iter()
            .all(|r| r.points.len() == 2)
    );
    let scene = matrix.projection.scene.clone();
    assert_eq!(crate::relation_forms::matrix_headings(&scene).len(), 3);
    for (_, row, column) in crate::relation_forms::matrix_headings(&scene) {
        for p in [row, column] {
            assert!(p.x >= scene.bounds.origin.x);
            assert!(p.y >= scene.bounds.origin.y);
            assert!(p.x < scene.bounds.origin.x + scene.bounds.size.w);
            assert!(p.y < scene.bounds.origin.y + scene.bounds.size.h);
        }
    }
    let mut refreshed = matrix.projection;
    for _ in 0..3 {
        crate::relation_forms::refresh_scene(&mut refreshed.scene);
    }
    assert_eq!(
        refreshed.scene, scene,
        "refresh never grows its own matrix bounds"
    );
    let refreshed = compiler()
        .refresh(&refreshed, &definition(&data, "grid", "x"), &data.dataset)
        .unwrap();
    let mut restored = refreshed;
    route_relationships(&mut restored, &data.dataset, &data.relationships).unwrap();
    assert_eq!(
        restored.scene, stroke.projection.scene,
        "changing form restores unconstrained geometry"
    );
}

#[test]
fn order_repositions_slots_and_preserves_occurrence_identity() {
    for kind in ["grid", "spiral", "mosaic"] {
        let (mut data, recipe) = fixture(kind, RelationForm::Order);
        data.relationships = vec![request(&data, "reverse", "c", "a")];
        let baseline = compiler().compile(&recipe, &data.dataset).unwrap();
        let compiled = data.compile(&compiler(), &recipe).unwrap().projection;
        let a = compiled.instance_by_occurrence["a"];
        let c = compiled.instance_by_occurrence["c"];
        assert_eq!(
            compiled.instance_by_occurrence,
            baseline.instance_by_occurrence
        );
        assert_eq!(
            compiled.occurrence_by_instance,
            baseline.occurrence_by_instance
        );
        assert_ne!(
            compiled.scene.items[a.0 as usize].transform,
            baseline.scene.items[a.0 as usize].transform,
            "{kind}"
        );
        assert_eq!(
            compiled.scene.items[a.0 as usize].source,
            baseline.scene.items[a.0 as usize].source
        );
        assert!(
            compiled
                .scene
                .arrangement_order
                .iter()
                .position(|i| *i == c)
                < compiled
                    .scene
                    .arrangement_order
                    .iter()
                    .position(|i| *i == a)
        );
        assert!(compiled.scene.relation_constraints[0].honored, "{kind}");
        assert!(compiled.scene.relations[0].points.is_empty());
    }
}

#[test]
fn conflicting_order_is_reported_and_pins_keep_their_exact_positions() {
    let (mut data, recipe) = fixture("grid", RelationForm::Order);
    data.relationships = vec![
        request(&data, "first", "c", "a"),
        request(&data, "second", "a", "c"),
    ];
    let compiled = data.compile(&compiler(), &recipe).unwrap().projection;
    assert!(compiled.scene.relation_constraints[0].honored);
    assert!(!compiled.scene.relation_constraints[1].honored);
    assert!(!compiled.scene.relation_constraints[1].reason.is_empty());

    let mut pinned = compiler().compile(&recipe, &data.dataset).unwrap();
    pinned.score.holds.push(sceno::HeldPlacement {
        source: pinned.score.items[0].source.clone(),
        at: Vec2::new(-100.0, -100.0),
        hold: sceno::Hold::Pinned,
    });
    pinned.scene = crate::solve_via(&pinned.score, &pinned.relation_registry).unwrap();
    let at = pinned.scene.items[0].transform.translate;
    route_relationships(&mut pinned, &data.dataset, &data.relationships).unwrap();
    assert_eq!(pinned.scene.items[0].transform.translate, at);
    assert_eq!(pinned.scene.honored_holds.len(), 1);
    assert!(pinned.scene.relation_constraints.iter().any(|r| !r.honored));
}

#[test]
fn partition_contains_children_with_weighted_extents_and_reports_competing_parents() {
    let (mut data, mut recipe) = fixture("partition", RelationForm::Containment);
    // A common source group permits nesting without crossing a categorical boundary.
    for (i, occurrence) in data.dataset.occurrences.iter_mut().enumerate() {
        occurrence.values.insert(
            "label".into(),
            crate::projection::ProjectionValue::Text("group".into()),
        );
        occurrence.values.insert(
            "x".into(),
            crate::projection::ProjectionValue::Number([8.0, 1.0, 3.0][i]),
        );
    }
    data.relationships = vec![
        request(&data, "one", "a", "b"),
        request(&data, "two", "a", "c"),
    ];
    recipe.arrangement.options.insert("gap".into(), "0".into());
    let mut compiled = compiler().compile(&recipe, &data.dataset).unwrap();
    // Scores can disclose weights directly; recipe weight-channel parity is B4.
    for (item, weight) in compiled.score.items.iter_mut().zip([8.0, 1.0, 3.0]) {
        item.weight = Some(weight);
    }
    compiled.scene = crate::solve_via(&compiled.score, &compiled.relation_registry).unwrap();
    route_relationships(&mut compiled, &data.dataset, &data.relationships).unwrap();
    assert!(
        compiled
            .scene
            .relation_constraints
            .iter()
            .all(|r| r.honored)
    );
    let a = compiled.instance_by_occurrence["a"];
    let b = compiled.instance_by_occurrence["b"];
    let c = compiled.instance_by_occurrence["c"];
    let area = |id: InstanceId| {
        let r = compiled.scene.items[id.0 as usize]
            .footprint
            .bounds()
            .unwrap();
        r.size.w * r.size.h
    };
    assert!((area(c) / area(b) - 3.0).abs() < 0.001);
    assert_eq!(
        compiled.scene.items[b.0 as usize].layer,
        compiled.scene.items[a.0 as usize].layer + 1
    );
    data.relationships
        .push(request(&data, "z-competing", "c", "b"));
    let competing = data.compile(&compiler(), &recipe).unwrap().projection;
    assert!(!competing.scene.relation_constraints[2].honored);
    assert!(
        competing.scene.relation_constraints[2]
            .reason
            .contains("competing parents")
    );
    data.relationships[2] = request(&data, "z-cycle", "b", "a");
    let cycle = data.compile(&compiler(), &recipe).unwrap().projection;
    assert!(!cycle.scene.relation_constraints[2].honored);
    assert!(cycle.scene.relation_constraints[2].reason.contains("cycle"));

    let mut pinned = compiler().compile(&recipe, &data.dataset).unwrap();
    pinned.score.holds.push(sceno::HeldPlacement {
        source: pinned.score.items[1].source.clone(),
        at: Vec2::new(10_000.0, 10_000.0),
        hold: sceno::Hold::Pinned,
    });
    pinned.scene = crate::solve_via(&pinned.score, &pinned.relation_registry).unwrap();
    route_relationships(&mut pinned, &data.dataset, &data.relationships).unwrap();
    assert_eq!(
        pinned.scene.items[1].transform.translate,
        Vec2::new(10_000.0, 10_000.0)
    );
    assert!(!pinned.scene.relation_constraints[0].honored);
    assert_eq!(pinned.scene.honored_holds.len(), 1);
}

#[test]
fn containment_never_moves_a_child_out_of_its_source_group() {
    let (data, recipe) = fixture("partition", RelationForm::Containment);
    let compiled = data.compile(&compiler(), &recipe).unwrap().projection;
    assert!(!compiled.scene.relation_constraints[0].honored);
    let baseline = compiler().compile(&recipe, &data.dataset).unwrap();
    assert_eq!(compiled.scene.items, baseline.scene.items);
}

#[test]
fn unsupported_defaults_are_refused_before_solving_and_rule_failure_is_atomic() {
    for (kind, form) in [
        ("grid", RelationForm::Containment),
        ("partition", RelationForm::Order),
        ("spiral", RelationForm::Adjacency),
    ] {
        let (data, recipe) = fixture(kind, form);
        let errors = compiler().compile(&recipe, &data.dataset).unwrap_err();
        assert!(errors.iter().any(|e| e.field == "marks.relation"));
    }
    let (data, mut recipe) = fixture("partition", RelationForm::Containment);
    recipe
        .arrangement
        .options
        .insert("form".into(), "rings".into());
    assert!(
        compiler()
            .compile(&recipe, &data.dataset)
            .unwrap_err()
            .iter()
            .any(|e| e.field == "marks.relation")
    );
    let (data, recipe) = fixture("grid", RelationForm::Stroke);
    let mut projection = compiler().compile(&recipe, &data.dataset).unwrap();
    projection.relation_rules = relation_rules(RelationForm::Containment);
    let before = projection.scene.clone();
    let before_marks = projection.relation_marks.clone();
    let issues =
        route_relationships(&mut projection, &data.dataset, &data.relationships).unwrap_err();
    assert!(issues.iter().any(|e| e.field.ends_with(".form")));
    assert_eq!(projection.scene, before);
    assert_eq!(projection.relation_marks, before_marks);
}

fn relation_rules(form: RelationForm) -> RuleSet {
    RuleSet {
        version: 1,
        rules: vec![Rule {
            id: "one-edge".into(),
            label: "One relationship".into(),
            enabled: true,
            priority: 1,
            target: Target {
                kind: PrimitiveKind::Relation,
                ids: ["rel:a-b".into()].into(),
            },
            condition: Condition::Always,
            effects: vec![Effect::RelationForm { value: form }],
        }],
    }
}

#[test]
fn relation_rules_override_recipe_defaults_and_keep_the_resolution_trace() {
    let (data, recipe) = fixture("grid", RelationForm::Stroke);
    let compiled = data
        .compile(
            &compiler().with_presentation(relation_rules(RelationForm::MatrixCell)),
            &recipe,
        )
        .unwrap()
        .projection;
    let resolved = &compiled.relation_marks["rel:a-b"];
    assert_eq!(resolved.defaults.relation, Some(RelationForm::Stroke));
    assert_eq!(resolved.marks.relation, Some(RelationForm::MatrixCell));
    assert_eq!(
        resolved.resolution.winners[&Property::RelationForm].rule_id,
        "one-edge"
    );
    assert!(compiled.scene.relations[0].cell_bounds().is_some());
}

#[test]
fn matrix_refresh_tracks_motion() {
    let (data, recipe) = fixture("grid", RelationForm::MatrixCell);
    let mut compiled = data.compile(&compiler(), &recipe).unwrap().projection;
    let before = compiled.scene.relations[0].cell_bounds().unwrap();
    compiled.scene.items[2].transform.translate.x += 10_000.0;
    crate::relation_forms::refresh_scene(&mut compiled.scene);
    assert!(compiled.scene.relations[0].cell_bounds().unwrap().origin.x > before.origin.x);
    assert_eq!(compiled.scene.relations[0].points.len(), 4);
}

#[test]
fn custom_arrangements_must_declare_placed_forms_before_the_solver_is_called() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    struct Custom {
        declared: bool,
        calls: Arc<AtomicUsize>,
    }
    impl crate::Solver for Custom {
        fn capability(&self) -> crate::SolverCapability {
            let mut capability = crate::SolverCapability::new("custom-order", "Custom order");
            if self.declared {
                capability.placed_forms.push(RelationForm::Order);
            }
            capability
        }
        fn place(
            &self,
            _: &Value,
            items: &[&sceno::ScoreItem],
        ) -> Result<Vec<Vec2>, crate::SolveError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(items
                .iter()
                .enumerate()
                .map(|(rank, _)| Vec2::new(rank as f32 * 200.0, 0.0))
                .collect())
        }
    }
    for declared in [false, true] {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut registry = crate::SolverRegistry::new();
        registry
            .register(Arc::new(Custom {
                declared,
                calls: calls.clone(),
            }))
            .unwrap();
        let compiler = ProjectionCompiler::with_registry(
            ItemSizes {
                card: Size2::new(40.0, 20.0),
            },
            registry,
        );
        let (mut data, recipe) = fixture("custom-order", RelationForm::Order);
        data.relationships = vec![request(&data, "reverse", "c", "a")];
        let result = data.compile(&compiler, &recipe);
        if declared {
            assert!(result.unwrap().projection.scene.relation_constraints[0].honored);
            assert_eq!(calls.load(Ordering::SeqCst), 2);
        } else {
            assert!(
                result
                    .unwrap_err()
                    .iter()
                    .any(|e| e.field == "marks.relation")
            );
            assert_eq!(calls.load(Ordering::SeqCst), 0);
            let mut default_stroke = recipe.clone();
            default_stroke.marks.relation = Some(RelationForm::Stroke);
            let mut rule = relation_rules(RelationForm::Order);
            rule.rules[0].target.ids = ["reverse".into()].into();
            assert!(
                data.compile(&compiler.with_presentation(rule), &default_stroke)
                    .is_err()
            );
            assert_eq!(
                calls.load(Ordering::SeqCst),
                0,
                "rule capability refusal precedes placement"
            );
        }
    }
}

#[test]
fn moving_a_nested_child_refreshes_its_report_without_moving_it_back() {
    let (mut data, recipe) = fixture("partition", RelationForm::Containment);
    for occurrence in &mut data.dataset.occurrences {
        occurrence.values.insert(
            "label".into(),
            crate::projection::ProjectionValue::Text("group".into()),
        );
    }
    let mut compiled = data.compile(&compiler(), &recipe).unwrap().projection;
    assert!(compiled.scene.relation_constraints[0].honored);
    let child = compiled.instance_by_occurrence["b"].0 as usize;
    compiled.scene.items[child].transform.translate = Vec2::new(10_000.0, 10_000.0);
    crate::relation_forms::refresh_scene(&mut compiled.scene);
    assert!(!compiled.scene.relation_constraints[0].honored);
    assert_eq!(
        compiled.scene.items[child].transform.translate,
        Vec2::new(10_000.0, 10_000.0)
    );
}
