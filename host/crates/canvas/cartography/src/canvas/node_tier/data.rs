// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The tier's instance data as flat words, built on the CPU and uploaded as is.
//!
//! Positions are `[slots, 4]` f32 (`x, y, 0, 0`), the layout Seiche's resident
//! graph publishes, so a bound GPU buffer can stand in for the upload. Node
//! attributes are four words a node; links four words a link.

use std::collections::HashMap;

use kernel::graph::{GraphInputKey, NodeKey};

/// Per-node flag bits, in the attribute's `meta` word and the shader.
pub(crate) mod flags {
    /// Alive and in scope: links to it are drawn.
    pub const VISIBLE: u32 = 1;
    /// The GPU draws the body (a DOM gnode draws it otherwise).
    pub const BODY: u32 = 1 << 1;
    pub const SELECTED: u32 = 1 << 2;
    pub const HOVERED: u32 = 1 << 3;
    pub const FOCUSED: u32 = 1 << 4;
    pub const PINNED: u32 = 1 << 5;
}

/// Shape ids in the attribute's `meta` word.
pub(crate) mod shape {
    /// Square, rounded square or circle: by corner radius.
    pub const ROUNDED: u32 = 0;
    pub const DIAMOND: u32 = 1;
    pub const HEXAGON: u32 = 2;
}

/// One node's attribute words: half side (world px), lift (world px),
/// colour (`r | g << 8 | b << 16 | a << 24`), and `meta`:
/// bits 0..2 shape, 2..10 flags, 16..24 corner radius as a fraction of the
/// half side in 1/255ths.
pub(crate) fn pack_node(
    half: f32,
    lift: f32,
    rgb: [u8; 3],
    shape: u32,
    radius_fraction: f32,
    flag_bits: u32,
) -> [u32; 4] {
    let radius = (radius_fraction.clamp(0.0, 1.0) * 255.0).round() as u32;
    [
        half.to_bits(),
        lift.to_bits(),
        u32::from(rgb[0]) | u32::from(rgb[1]) << 8 | u32::from(rgb[2]) << 16 | 0xff << 24,
        (shape & 0x3) | (flag_bits & 0xff) << 2 | radius << 16,
    ]
}

/// A link's words: from slot, to slot, colour `rgba8`, width px (bits).
pub(crate) fn pack_link(from: u32, to: u32, rgba: [u8; 4], width: f32) -> [u32; 4] {
    [
        from,
        to,
        u32::from(rgba[0])
            | u32::from(rgba[1]) << 8
            | u32::from(rgba[2]) << 16
            | u32::from(rgba[3]) << 24,
        width.to_bits(),
    ]
}

/// The tier's CPU-side buffers and the slot map they are indexed by.
#[derive(Default)]
pub(crate) struct NodeCpuData {
    /// Slot to node.
    pub keys: Vec<NodeKey>,
    pub slot_of: HashMap<NodeKey, u32>,
    /// The graph structure the slots were built from.
    pub structure: Option<(GraphInputKey, usize)>,
    pub positions: Vec<f32>,
    pub attrs: Vec<u32>,
    /// Body slots, back to front.
    pub order: Vec<u32>,
    pub links: Vec<u32>,
    /// What the links were built from.
    pub links_key: Option<(GraphInputKey, [usize; 3])>,
    sort_scratch: Vec<u64>,
}

impl NodeCpuData {
    /// Re-slot when the graph's structure changed. Returns whether it did.
    pub fn sync_slots(
        &mut self,
        structure: (GraphInputKey, usize),
        nodes: impl Iterator<Item = NodeKey>,
    ) -> bool {
        if self.structure == Some(structure) {
            return false;
        }
        self.keys.clear();
        self.keys.extend(nodes);
        self.slot_of.clear();
        self.slot_of
            .extend(self.keys.iter().enumerate().map(|(i, &k)| (k, i as u32)));
        let n = self.keys.len();
        self.positions.clear();
        self.positions.resize(n * 4, 0.0);
        self.attrs.clear();
        self.attrs.resize(n * 4, 0);
        self.structure = Some(structure);
        self.links_key = None;
        true
    }

    pub fn slots(&self) -> usize {
        self.keys.len()
    }

    pub fn set_position(&mut self, slot: u32, x: f32, y: f32) {
        let i = slot as usize * 4;
        self.positions[i] = x;
        self.positions[i + 1] = y;
    }

    pub fn set_attr(&mut self, slot: u32, words: [u32; 4]) {
        let i = slot as usize * 4;
        self.attrs[i..i + 4].copy_from_slice(&words);
    }

    pub fn flags_of(&self, slot: usize) -> u32 {
        (self.attrs[slot * 4 + 3] >> 2) & 0xff
    }

    /// Body slots sorted back to front by projected ground depth
    /// (`x sin yaw + y cos yaw`, rounded as the DOM's z-index is), ties by slot.
    pub fn sort_bodies(&mut self, yaw: f32) {
        let (s, c) = yaw.sin_cos();
        self.sort_scratch.clear();
        for slot in 0..self.slots() {
            if self.flags_of(slot) & flags::BODY == 0 {
                continue;
            }
            let (x, y) = (self.positions[slot * 4], self.positions[slot * 4 + 1]);
            let depth = (x * s + y * c).round().clamp(-2.0e9, 2.0e9) as i64;
            let biased = (depth + (1 << 31)) as u64;
            self.sort_scratch.push(biased << 32 | slot as u64);
        }
        self.sort_scratch.sort_unstable();
        self.order.clear();
        self.order.extend(
            self.sort_scratch
                .iter()
                .map(|word| (word & 0xffff_ffff) as u32),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kernel::geometry::PortablePoint;
    use kernel::graph::{Graph, apply};

    fn data_with(n: usize) -> NodeCpuData {
        let mut graph = Graph::new();
        let keys: Vec<_> = (0..n)
            .map(|i| {
                apply::add_node(
                    &mut graph,
                    None,
                    format!("mere://n/{i}"),
                    PortablePoint::new(0.0, 0.0),
                )
            })
            .collect();
        let mut data = NodeCpuData::default();
        let key = graph.structural_input_key();
        assert!(data.sync_slots((key, n), keys.into_iter()));
        data.structure = Some((key, n));
        data
    }

    #[test]
    fn slots_rebuild_only_when_the_structure_moves() {
        let mut data = data_with(3);
        let same = data.structure.unwrap();
        assert!(!data.sync_slots(same, std::iter::empty()));
        assert_eq!(data.slots(), 3);
        assert_eq!(data.positions.len(), 12);
    }

    #[test]
    fn a_node_packs_shape_state_colour_and_flags() {
        let words = pack_node(
            18.0,
            4.0,
            [10, 20, 30],
            shape::HEXAGON,
            0.5,
            flags::BODY | flags::SELECTED,
        );
        assert_eq!(f32::from_bits(words[0]), 18.0);
        assert_eq!(f32::from_bits(words[1]), 4.0);
        assert_eq!(words[2], 10 | 20 << 8 | 30 << 16 | 0xff << 24);
        assert_eq!(words[3] & 0x3, shape::HEXAGON);
        assert_eq!((words[3] >> 2) & 0xff, flags::BODY | flags::SELECTED);
        assert_eq!(words[3] >> 16, 128);
    }

    #[test]
    fn bodies_sort_back_to_front_and_skip_dom_nodes() {
        let mut data = data_with(4);
        for (slot, y, body) in [
            (0, 30.0, true),
            (1, -5.0, true),
            (2, 10.0, false),
            (3, 10.0, true),
        ] {
            data.set_position(slot, 0.0, y);
            let bits = flags::VISIBLE | if body { flags::BODY } else { 0 };
            data.set_attr(
                slot,
                pack_node(10.0, 0.0, [0; 3], shape::ROUNDED, 0.0, bits),
            );
        }
        data.sort_bodies(0.0);
        assert_eq!(data.order, vec![1, 3, 0]);
    }

    #[test]
    fn a_link_names_both_endpoints_by_slot() {
        let words = pack_link(4, 9, [1, 2, 3, 4], 1.5);
        assert_eq!((words[0], words[1]), (4, 9));
        assert_eq!(f32::from_bits(words[3]), 1.5);
    }
}
