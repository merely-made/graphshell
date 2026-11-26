// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The `viewer-guest` test fixture: one region per node of `snapshot.nodes`, labelled
//! `Node <name>`, where the name is the node's `title`, else its `label`, else its place in
//! the list. A node named `TRAP` traps, one named `SPIN` never returns, and a graph that is
//! not JSON is refused. The scene's items are centred on their translate, as block-adapter's
//! `regions()` reads it. The sheet is a column, one row per node: a square and the name.

wit_bindgen::generate!({
    path: "../../wit",
    world: "viewer-guest",
});

use exports::mere::script::viewer::{Claim, Guest, ViewAsk, ViewResult};
use sceno::{
    Footprint, InstanceId, ProjectedItem, Region, Representation, Scene, Size2, SourceIx,
    SourceRef, Transform2, Vec2,
};
use vector_sheet::{Op, Paint, Path, Role, VectorSheet};

struct Component;

fn labels(graph: &[u8]) -> Result<Vec<String>, String> {
    let value: serde_json::Value =
        serde_json::from_slice(graph).map_err(|e| format!("the graph is not JSON: {e}"))?;
    let nodes = value
        .pointer("/snapshot/nodes")
        .and_then(serde_json::Value::as_array)
        .ok_or("the graph has no snapshot.nodes")?;
    Ok(nodes
        .iter()
        .enumerate()
        .map(|(i, node)| {
            ["title", "label"]
                .iter()
                .find_map(|key| node.get(key).and_then(serde_json::Value::as_str))
                .map_or_else(|| (i + 1).to_string(), str::to_string)
        })
        .collect())
}

impl Guest for Component {
    fn describe() -> Claim {
        Claim {
            title: "Fixture nodes".into(),
            graph_format: "mere.graph-snapshot-and-facets/json".into(),
        }
    }

    fn view(graph: Vec<u8>, ask: ViewAsk) -> Result<ViewResult, String> {
        let labels = labels(&graph)?;
        if labels.iter().any(|l| l == "TRAP") {
            panic!("fixture traps");
        }
        if labels.iter().any(|l| l == "SPIN") {
            loop {
                std::hint::black_box(0);
            }
        }
        let mut scene = Scene::new();
        let mut regions = Vec::new();
        let mut ops = Vec::new();
        for (i, label) in labels.iter().enumerate() {
            // The square's top-left is (10, y); its centre is what the item's translate says.
            let y = 10.0 + 30.0 * i as f32;
            let source = scene.sources.len();
            scene.sources.push(SourceRef::new("fixture.node", format!("node:{i}")));
            scene.items.push(ProjectedItem {
                source: SourceIx(source as u32),
                space: Scene::WORLD,
                transform: Transform2 { translate: Vec2::new(20.0, y + 10.0), ..Transform2::IDENTITY },
                footprint: Footprint::Rect { size: Size2::new(20.0, 20.0) },
                representation: Representation::Open { kind: "fixture:node".into() },
                layer: 1,
                visible: true,
                hit: None,
                channels: vec![],
            });
            regions.push(Region {
                members: vec![InstanceId(i as u32)],
                space: Scene::WORLD,
                contour: None,
                label: Some(format!("Node {label}")),
                confidence: 1.0,
            });
            ops.push(Op::Fill {
                path: Path::rect(10.0, y, 20.0, 20.0),
                rule: vector_sheet::FillRule::NonZero,
                paint: Paint::new(Role::Accent),
            });
            ops.push(Op::Text {
                x: 40.0,
                y: y + 2.0,
                size: 14.0,
                text: label.clone(),
                anchor: vector_sheet::Anchor::Start,
                face: vector_sheet::Face::Reading,
                paint: Paint::new(Role::Ink),
            });
        }
        scene.regions = regions;
        let body = cbor_core::Value::serialized(&scene).map_err(|e| e.to_string())?;
        let mut scene_bytes = vec![1u8];
        scene_bytes.extend(body.encode());
        let sheet = VectorSheet {
            width: ask.width.max(100.0),
            height: 10.0 + 30.0 * labels.len().max(1) as f32,
            ops,
        };
        let sheet_bytes = sheet.encode().map_err(|e| e.to_string())?;
        Ok(ViewResult {
            scene: scene_bytes,
            sheet: sheet_bytes,
            width: sheet.width,
            height: sheet.height,
            description: format!("{} nodes", labels.len()),
        })
    }
}

export!(Component);
