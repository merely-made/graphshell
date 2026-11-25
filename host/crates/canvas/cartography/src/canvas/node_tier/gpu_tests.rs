// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The node pass rendered into a texture on a real adapter and read back
//! once. Skips where there is no adapter.

use std::collections::HashMap;

use super::{GpuNodeTarget, gpu_node_device_for};
use crate::canvas::{Canvas, NodeShape, NodeState};

/// RGBA8 pixels of `target`, row-major.
fn read_back(handles: &netrender::WgpuHandles, canvas: &Canvas) -> (Vec<u8>, [u32; 2]) {
    let gpu = canvas.node_tier.gpu.as_ref().unwrap();
    let texture = gpu.texture().unwrap();
    let GpuNodeTarget { size, .. } = canvas.gpu_node_target().unwrap();
    let row = (size[0] * 4).next_multiple_of(256);
    let buffer = handles.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("node tier readback"),
        size: u64::from(row * size[1]),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = handles.device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(size[1]),
            },
        },
        wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
    );
    let submission = handles.queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    handles
        .device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(std::time::Duration::from_secs(10)),
        })
        .unwrap();
    rx.recv().unwrap().unwrap();
    let mapped = buffer.slice(..).get_mapped_range().expect("mapped range");
    let mut pixels = Vec::with_capacity((size[0] * size[1] * 4) as usize);
    for y in 0..size[1] {
        let start = (y * row) as usize;
        pixels.extend_from_slice(&mapped[start..start + (size[0] * 4) as usize]);
    }
    (pixels, size)
}

#[test]
fn the_node_pass_draws_bodies_links_and_leaves_the_background_clear() {
    let Ok(handles) = netrender::boot() else {
        eprintln!("no wgpu adapter: skipping the node pass render test");
        return;
    };
    let (w, h) = (800u32, 600u32);
    let mut canvas = Canvas::with_graph(crate::canvas::build::sample_graph());
    canvas.resize(w, h);
    canvas.reseed();
    canvas.fit_visible();
    let keys: Vec<_> = canvas.graph.nodes().map(|(key, _)| key).collect();
    let id = |i: usize| canvas.graph.get_node(keys[i]).unwrap().id;
    let shapes = [
        (3, NodeShape::RoundedSquare),
        (4, NodeShape::Circle),
        (5, NodeShape::Diamond),
        (6, NodeShape::Hexagon),
        (7, NodeShape::Square),
    ];
    let shapes: HashMap<_, _> = shapes.iter().map(|&(i, s)| (id(i), s)).collect();
    let states = HashMap::from([(id(1), NodeState::Open), (id(2), NodeState::Closed)]);
    canvas.set_node_shapes(shapes);
    canvas.set_node_states(states);
    canvas.set_gpu_node_device(Some(gpu_node_device_for(&handles)));
    canvas.set_gpu_node_tier(true);
    canvas.set_gpu_node_target(9, 1.0);
    // One gnode only: the selected node, which the GPU leaves to the DOM.
    canvas.set_max_dom_gnodes(1);
    canvas.select_only(keys[0]);
    // Let the layout open up until no two bodies overlap.
    for _ in 0..600 {
        canvas.frame(w, h);
        let centers: Vec<_> = keys
            .iter()
            .filter_map(|&k| canvas.node_billboard(k))
            .map(|b| b.center)
            .collect();
        let apart = centers.iter().enumerate().all(|(i, a)| {
            centers[i + 1..]
                .iter()
                .all(|b| (a.x - b.x).hypot(a.y - b.y) > 60.0)
        });
        if apart {
            break;
        }
    }
    assert!(canvas.gpu_node_tier_active());
    assert!(canvas.node_tier.assigner.holds(keys[0]));
    let target = canvas.gpu_node_target().unwrap();
    assert_eq!((target.key, target.size), (9, [w, h]));
    let (pixels, size) = read_back(&handles, &canvas);
    let at = |x: f32, y: f32| {
        let i = ((y.round() as u32).min(size[1] - 1) * size[0]
            + (x.round() as u32).min(size[0] - 1)) as usize
            * 4;
        [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
    };
    let near = |got: [u8; 4], want: [u8; 3]| {
        got[3] == 255 && (0..3).all(|c| (i32::from(got[c]) - i32::from(want[c])).abs() <= 3)
    };
    let body = |i: usize| canvas.node_billboard(keys[i]).unwrap();
    let center = |i: usize| (body(i).center.x, body(i).center.y);

    // State colours at the centres of GPU-drawn bodies.
    assert!(
        near(at(center(1).0, center(1).1), [58, 140, 94]),
        "open {:?} at {:?} tex {:?} stats {:?} tier {}",
        at(center(1).0, center(1).1),
        center(1),
        size,
        canvas.gpu_node_stats(),
        canvas.gpu_node_tier_active()
    );
    assert!(
        near(at(center(2).0, center(2).1), [166, 72, 72]),
        "closed {:?}",
        at(center(2).0, center(2).1)
    );
    assert!(
        near(at(center(7).0, center(7).1), [54, 92, 156]),
        "idle {:?}",
        at(center(7).0, center(7).1)
    );
    // The selected node is the DOM's: no amber body under it.
    assert!(
        !near(at(center(0).0, center(0).1), [232, 150, 40]),
        "selected is skipped"
    );
    // A link midpoint is painted, by the link and not a body.
    let (a, b) = (center(8), center(9));
    let mid = at((a.0 + b.0) * 0.5, (a.1 + b.1) * 0.5);
    assert!(mid[3] > 60, "link midpoint {mid:?}");
    // Premultiplied: colour never exceeds alpha.
    assert!(
        pixels
            .chunks(4)
            .all(|p| p[0] <= p[3] && p[1] <= p[3] && p[2] <= p[3])
    );
    // Background corner.
    assert_eq!(at(1.0, 1.0)[3], 0);

    // Silhouettes: how many of the four diagonal points at 0.38 of the side
    // are inside. A link may graze one.
    let corners = |i: usize| {
        let (cx, cy) = center(i);
        let d = body(i).local_side * body(i).zoom * 0.38;
        [(d, d), (-d, d), (d, -d), (-d, -d)]
            .iter()
            .filter(|(dx, dy)| at(cx + dx, cy + dy)[3] > 128)
            .count()
    };
    assert_eq!(corners(7), 4, "square");
    assert!(corners(3) >= 3, "rounded square");
    assert!(corners(4) <= 1, "circle");
    assert!(corners(5) <= 1, "diamond");
    assert!(corners(6) <= 1, "hexagon");
}
