// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The bounded gnode tier and the tier's instance data over a real canvas
//! frame. The pass itself needs an adapter and is checked in `gpu_tests`.

use std::collections::{HashMap, HashSet};

use kernel::geometry::PortablePoint;
use kernel::graph::{Graph, NodeKey, apply};

use super::data::flags;
use crate::canvas::build::hyperlink;
use crate::canvas::{Canvas, NodeShape};

const CAP: usize = 40;

/// 500 nodes on a screen-filling grid with a few links, tier forced on.
fn grid_canvas() -> (Canvas, Vec<NodeKey>) {
    let mut graph = Graph::new();
    let keys: Vec<NodeKey> = (0..500)
        .map(|i| {
            let at =
                PortablePoint::new(20.0 + (i % 25) as f32 * 38.0, 20.0 + (i / 25) as f32 * 38.0);
            apply::add_node(&mut graph, None, format!("mere://n/{i}"), at)
        })
        .collect();
    for pair in [(0, 1), (1, 2), (30, 31)] {
        let _ = apply::assert_relation(&mut graph, keys[pair.0], keys[pair.1], hyperlink());
    }
    let mut canvas = Canvas::with_graph(graph);
    canvas.resize(1000, 800);
    canvas.reseed();
    canvas.node_tier.forced = true;
    canvas.set_max_dom_gnodes(CAP);
    (canvas, keys)
}

fn holders(canvas: &Canvas) -> HashMap<NodeKey, usize> {
    canvas.node_tier.assigner.holders().collect()
}

/// The on-screen node farthest from the viewport centre, which distance alone
/// would never choose.
fn farthest_on_screen(canvas: &Canvas, keys: &[NodeKey]) -> NodeKey {
    keys.iter()
        .copied()
        .filter_map(|key| Some((key, canvas.node_billboard(key)?.center)))
        .filter(|(_, c)| c.x > 0.0 && c.y > 0.0 && c.x < 1000.0 && c.y < 800.0)
        .max_by(|a, b| {
            let d =
                |c: euclid::default::Point2D<f32>| (c.x - 500.0).powi(2) + (c.y - 400.0).powi(2);
            d(a.1).total_cmp(&d(b.1))
        })
        .map(|(key, _)| key)
        .expect("a node on screen")
}

#[test]
fn the_pool_is_bounded_and_every_other_node_is_gpu_only() {
    let (mut canvas, keys) = grid_canvas();
    canvas.frame(1000, 800);
    assert_eq!(canvas.node_tier.dom.len(), CAP);
    assert_eq!(canvas.gnode_of.len(), CAP);
    assert_eq!(canvas.node_tier.assigner.len(), CAP);
    let shown = canvas
        .node_tier
        .dom
        .iter()
        .filter(|slot| !slot.hidden)
        .count();
    assert_eq!(shown, CAP);
    let held: HashSet<NodeKey> = holders(&canvas).into_keys().collect();
    assert!(keys.iter().filter(|key| !held.contains(key)).count() >= 400);
    // The held nodes carry their caption; the pool's elements are recycled.
    assert!(
        canvas
            .node_tier
            .dom
            .iter()
            .all(|slot| !slot.label.is_empty())
    );
}

#[test]
fn the_selected_node_holds_a_gnode_and_assignment_is_stable_across_frames() {
    let (mut canvas, keys) = grid_canvas();
    canvas.frame(1000, 800);
    let far = farthest_on_screen(&canvas, &keys);
    canvas.select_only(far);
    canvas.frame(1000, 800);
    assert!(canvas.node_tier.assigner.holds(far));
    let before = holders(&canvas);
    canvas.frame(1000, 800);
    assert_eq!(holders(&canvas), before, "a still scene moves no gnode");
}

#[test]
fn tiny_nodes_lose_their_gnodes_except_the_selected() {
    let (mut canvas, keys) = grid_canvas();
    canvas.frame(1000, 800);
    let far = farthest_on_screen(&canvas, &keys);
    canvas.select_only(far);
    canvas.camera.zoom = 0.15;
    canvas.camera.offset = (400.0, 300.0);
    canvas.frame(1000, 800);
    let held: Vec<NodeKey> = holders(&canvas).into_keys().collect();
    assert_eq!(held, vec![far]);
    assert_eq!(
        canvas
            .node_tier
            .dom
            .iter()
            .filter(|slot| !slot.hidden)
            .count(),
        1
    );
}

#[test]
fn instances_carry_shape_state_and_flags_and_links_name_slots() {
    let (mut canvas, keys) = grid_canvas();
    canvas.set_node_shapes(HashMap::from([(
        canvas.graph.get_node(keys[5]).unwrap().id,
        NodeShape::Hexagon,
    )]));
    canvas.frame(1000, 800);
    let far = farthest_on_screen(&canvas, &keys);
    canvas.select_only(far);
    canvas.frame(1000, 800);
    let cpu = &canvas.node_tier.cpu;
    let attr = |key: NodeKey| -> [u32; 4] {
        let slot = cpu.slot_of[&key] as usize * 4;
        cpu.attrs[slot..slot + 4].try_into().unwrap()
    };
    let bits = |key: NodeKey| (attr(key)[3] >> 2) & 0xff;
    // Selected and focused (the only selection), held by a gnode so not drawn.
    assert_eq!(
        bits(far) & (flags::SELECTED | flags::FOCUSED | flags::VISIBLE),
        21
    );
    assert_eq!(bits(far) & flags::BODY, 0);
    // A node the GPU draws.
    let drawn = keys
        .iter()
        .copied()
        .find(|key| !canvas.node_tier.assigner.holds(*key))
        .unwrap();
    assert_eq!(bits(drawn) & (flags::BODY | flags::VISIBLE), 3);
    assert_eq!(bits(drawn) & flags::SELECTED, 0);
    // The hexagon rides its shape id; the amber of selection its colour.
    assert_eq!(attr(keys[5])[3] & 3, super::data::shape::HEXAGON);
    assert_eq!(attr(far)[2] & 0xffffff, 232 | 150 << 8 | 40 << 16);
    // Bodies drawn are exactly the slots with the BODY bit.
    let bodies = (0..cpu.slots())
        .filter(|&s| cpu.flags_of(s) & flags::BODY != 0)
        .count();
    assert_eq!(cpu.order.len(), bodies);
    // Three links, each naming the slots of its endpoints.
    assert_eq!(cpu.links.len(), 12);
    let ends: HashSet<(u32, u32)> = cpu
        .links
        .chunks(4)
        .map(|l| (l[0].min(l[1]), l[0].max(l[1])))
        .collect();
    let slot = |i: usize| cpu.slot_of[&keys[i]];
    assert!(ends.contains(&(slot(0).min(slot(1)), slot(0).max(slot(1)))));
}

#[test]
fn node_body_rect_reads_the_dom_for_a_gnode_and_the_billboard_for_the_rest() {
    let (mut canvas, keys) = grid_canvas();
    canvas.frame(1000, 800);
    let held = canvas.node_tier.assigner.holders().next().unwrap().0;
    let plain = keys
        .iter()
        .copied()
        .find(|key| {
            !canvas.node_tier.assigner.holds(*key)
                && canvas.node_billboard(*key).is_some_and(|b| {
                    let r = b.screen_rect();
                    r.min.x > 0.0 && r.min.y > 0.0 && r.max.x < 1000.0 && r.max.y < 800.0
                })
        })
        .unwrap();
    let rect = canvas
        .node_body_rect(plain)
        .expect("a drawn node has a rect");
    let body = canvas.node_billboard(plain).unwrap().screen_rect();
    assert_eq!(rect, (body.min.x, body.min.y, body.max.x, body.max.y));
    let dom = canvas.node_body_rect(held).expect("a gnode has a rect");
    let held_body = canvas.node_billboard(held).unwrap().screen_rect();
    assert!((dom.0 - held_body.min.x).abs() < 1.0 && (dom.2 - held_body.max.x).abs() < 1.0);
    // Off screen reads as nothing, as it did on the DOM path.
    canvas.camera.offset = (5000.0, 5000.0);
    canvas.frame(1000, 800);
    assert_eq!(canvas.node_body_rect(plain), None);
}

#[test]
fn turning_the_tier_off_restores_one_gnode_per_node() {
    let (mut canvas, keys) = grid_canvas();
    canvas.frame(1000, 800);
    canvas.node_tier.forced = false;
    canvas.frame(1000, 800);
    assert_eq!(canvas.gnode_of.len(), keys.len());
    assert!(canvas.node_tier.dom.is_empty());
}
