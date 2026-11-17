// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

use super::*;
use layout_dom_api::{LayoutDom, LocalName, Namespace};
use scenograph::presentation::{Body, Extent, Material, NodeMark, Paint, Silhouette};
use scenograph::{
    Arrangement, Channel, Encoding, Interaction, ProjectionDefinition, Provenance, Reading,
    SourceBinding,
};
use scenomise::projection::{
    ItemSizes, ProjectionCompiler, ProjectionDataset, ProjectionFieldType, ProjectionOccurrence,
    ProjectionValue,
};
use std::collections::BTreeMap;

fn compiled(canvas: &Canvas, key: NodeKey) -> CompiledProjection {
    let id = canvas.graph().get_node(key).unwrap().id;
    let source = SourceBinding {
        authority: "fixture".into(),
        domain: "graph".into(),
        resource: "marks".into(),
    };
    let dataset = ProjectionDataset {
        source: source.clone(),
        revision: "one".into(),
        fields: BTreeMap::from([
            ("id".into(), ProjectionFieldType::Text),
            ("label".into(), ProjectionFieldType::Text),
            ("x".into(), ProjectionFieldType::Number),
            ("y".into(), ProjectionFieldType::Number),
        ]),
        occurrences: vec![ProjectionOccurrence::new(
            "one",
            sceno::SourceRef::new(crate::MERE_GRAPH_ADAPTER, id.to_string()),
            BTreeMap::from([
                ("id".into(), ProjectionValue::Text("one".into())),
                ("label".into(), ProjectionValue::Text("Event".into())),
                ("x".into(), ProjectionValue::Number(0.0)),
                ("y".into(), ProjectionValue::Number(0.0)),
            ]),
        )],
    };
    let recipe = ProjectionDefinition {
        id: "marks".into(),
        label: "Marks".into(),
        source,
        reading: Reading {
            key: "id".into(),
            ..Default::default()
        },
        encoding: Encoding {
            x: Channel::Field("x".into()),
            y: Channel::Field("y".into()),
            label: Some(Channel::Field("label".into())),
            color: None,
        },
        arrangement: Arrangement {
            direction: "coordinates".into(),
            ..Default::default()
        },
        interaction: Interaction {
            pan: false,
            zoom: false,
            ..Default::default()
        },
        appearance: scenograph::Appearance {
            realization: "canvas".into(),
            title: "Marks".into(),
            theme: "slate".into(),
        },
        provenance: Provenance {
            author: "fixture".into(),
            source_revision: Some("one".into()),
            ..Default::default()
        },
        marks: scenograph::MarkSlots {
            node: Some(NodeMark {
                body: Body::Silhouette(Silhouette::Diamond),
                paint: Paint::Bare,
                material: Some(Material {
                    restitution: 0.2,
                    friction: 0.4,
                    density: 1.5,
                }),
                ..NodeMark::new(
                    sceno::Representation::Glyph,
                    [Extent::Measured, Extent::Assigned],
                )
            }),
            ..Default::default()
        },
        ..Default::default()
    };
    ProjectionCompiler::new(ItemSizes {
        card: sceno::Size2::new(40.0, 40.0),
    })
    .compile(&recipe, &dataset)
    .unwrap()
}

#[test]
fn compiled_recipe_marks_reach_paint_picking_colliders_and_material() {
    let mut canvas = Canvas::new();
    let key = canvas.visit("https://event.example");
    let projection = compiled(&canvas, key);
    canvas.set_compiled_projection(&projection).unwrap();
    assert_eq!(canvas.node_shape(key), Silhouette::Diamond);
    assert_eq!(canvas.node_face(key), Paint::Bare);
    assert_eq!(canvas.node_material(key).density, 1.5);
    assert!(
        matches!(canvas.node_collider(key), seiche::NodeCollider::Hull { points, .. } if points.len() == 4)
    );
    canvas.frame(800, 600);
    let body = canvas.node_billboard(key).unwrap();
    let side = body.local_side * body.zoom;
    let corner = body.center + euclid::default::Vector2D::new(0.44 * side, 0.44 * side);
    assert!(!body.contains_screen(corner));
    assert!(body.contains_screen(body.center));
    let node = canvas.gnode_of[&key];
    let classes = canvas
        .node_document
        .dom()
        .attribute(node, &Namespace::from(""), &LocalName::from("class"))
        .unwrap();
    assert!(classes.contains("diamond"));
    canvas.set_projection_score(None);
    assert!(canvas.projection_mark(key).is_none());
    assert_eq!(canvas.node_shape(key), Silhouette::Square);
    assert_eq!(canvas.node_material(key), seiche::NodeMaterial::default());
}

#[test]
fn failed_rebinding_is_atomic_and_graph_replacement_clears_marks() {
    let mut canvas = Canvas::new();
    let key = canvas.visit("https://event.example");
    let mut projection = compiled(&canvas, key);
    canvas.set_compiled_projection(&projection).unwrap();
    projection.node_marks.clear();
    assert!(canvas.set_compiled_projection(&projection).is_err());
    assert_eq!(canvas.node_shape(key), Silhouette::Diamond);
    let id = canvas.graph().get_node(key).unwrap().id;
    canvas.set_node_face(id, Paint::Derived);
    assert_eq!(canvas.node_face(key), Paint::Derived);
    canvas.clear_node_face(id);
    assert_eq!(canvas.node_face(key), Paint::Bare);
    canvas.set_graph(Graph::new());
    assert!(canvas.projection_marks.is_empty());
}

#[test]
fn live_rule_marks_override_recipe_marks_and_clear_atomically() {
    let mut canvas = Canvas::new();
    let key = canvas.visit("https://event.example");
    let id = canvas.graph().get_node(key).unwrap().id;
    let projection = compiled(&canvas, key);
    canvas.set_compiled_projection(&projection).unwrap();
    let epoch = canvas.graph_view_epoch();
    let mark = NodeMark {
        body: Body::Silhouette(Silhouette::Hexagon),
        paint: Paint::Derived,
        ..NodeMark::new(sceno::Representation::Glyph, [Extent::Measured])
    };
    assert!(Canvas::supports_node_rule_mark(&NodeMark {
        paint: Paint::Open {
            kind: "portrait".into()
        },
        ..mark.clone()
    }));
    let marks = HashMap::from([(id, mark.clone())]);
    assert!(
        canvas
            .set_node_rule_presentation(epoch, HashMap::new(), marks.clone())
            .unwrap()
    );
    assert_eq!(canvas.node_shape(key), Silhouette::Hexagon);
    assert_eq!(canvas.node_face(key), Paint::Derived);
    assert_eq!(canvas.node_material(key), seiche::NodeMaterial::default());
    assert!(
        !canvas
            .set_node_rule_presentation(epoch, HashMap::new(), marks.clone())
            .unwrap()
    );
    assert!(
        canvas
            .set_node_rule_presentation(epoch.wrapping_add(1), HashMap::new(), HashMap::new())
            .is_err()
    );
    let rungs = HashMap::from([(id, sceno::Representation::Card)]);
    assert!(
        canvas
            .set_node_rule_presentation(epoch, rungs, marks)
            .is_err()
    );
    assert_eq!(canvas.node_shape(key), Silhouette::Hexagon);
    canvas
        .set_node_rule_representations(epoch, HashMap::new())
        .unwrap();
    assert_eq!(canvas.node_shape(key), Silhouette::Diamond);
    assert_eq!(canvas.node_face(key), Paint::Bare);
    assert_eq!(canvas.node_material(key).density, 1.5);
    canvas.set_layout_strategy(Some("partition.default".into()));
    assert!(
        canvas
            .set_node_rule_presentation(epoch, HashMap::new(), HashMap::from([(id, mark)]),)
            .is_err()
    );
    assert_eq!(canvas.node_shape(key), Silhouette::Diamond);
}

#[test]
fn explicit_mark_silhouettes_control_rounding_in_both_grammars() {
    let mut canvas = Canvas::new();
    let key = canvas.visit("https://silhouette.example");
    let member = canvas.graph().get_node(key).unwrap().id;
    for grammar in [false, true] {
        canvas.set_graph_grammar(grammar);
        for shape in [
            Silhouette::Square,
            Silhouette::RoundedSquare,
            Silhouette::Circle,
            Silhouette::Diamond,
            Silhouette::Hexagon,
        ] {
            let mark = NodeMark {
                body: Body::Silhouette(shape),
                ..NodeMark::new(sceno::Representation::Glyph, [Extent::Measured])
            };
            canvas
                .set_node_rule_presentation(
                    canvas.graph_view_epoch(),
                    HashMap::new(),
                    HashMap::from([(member, mark)]),
                )
                .unwrap();
            canvas.frame(800, 600);
            let body = canvas.node_billboard(key).unwrap();
            let radius = match shape {
                Silhouette::Square | Silhouette::Diamond | Silhouette::Hexagon => 0.0,
                Silhouette::RoundedSquare => 9.0_f32.min(body.local_side * 0.5),
                Silhouette::Circle => body.local_side * 0.5,
            };
            assert_eq!(body.corner_radius, radius, "{grammar} {shape:?}");
            let node = canvas.gnode_of[&key];
            let style = canvas
                .node_document
                .dom()
                .attribute(node, &Namespace::from(""), &LocalName::from("style"))
                .unwrap();
            assert!(
                style.contains(&format!("border-radius: {radius}px;")),
                "{style}"
            );
            let delta = body.local_side * body.zoom * 0.44;
            let corner = body.center + euclid::default::Vector2D::new(delta, delta);
            assert_eq!(
                body.contains_screen(corner),
                shape == Silhouette::Square,
                "{grammar} {shape:?}"
            );
        }
    }
}
