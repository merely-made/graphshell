// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Which on-screen nodes hold a DOM gnode, and which pool element each holds.

use std::collections::{HashMap, HashSet};

use kernel::graph::NodeKey;

/// An on-screen node the tier may give a gnode.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Candidate {
    pub key: NodeKey,
    /// Body side in screen pixels.
    pub side_px: f32,
    /// Squared distance from the viewport centre, px.
    pub dist2: f32,
    /// Selected, hovered, focused or pinned.
    pub priority: bool,
    /// Held a gnode last frame.
    pub assigned: bool,
}

/// The nodes that get a gnode this frame, best first, at most `cap`.
///
/// A node qualifies by size or priority. Priority nodes go first, then those
/// that already hold a gnode (so the set does not thrash at the cap), then the
/// nearest to the viewport centre.
pub(crate) fn select_dom(
    candidates: &mut Vec<Candidate>,
    cap: usize,
    min_side: f32,
) -> Vec<NodeKey> {
    candidates.retain(|c| c.priority || c.side_px >= min_side);
    candidates.sort_by(|a, b| {
        b.priority
            .cmp(&a.priority)
            .then(b.assigned.cmp(&a.assigned))
            .then(a.dist2.total_cmp(&b.dist2))
    });
    candidates.iter().take(cap).map(|c| c.key).collect()
}

/// A bounded set of pool elements and who holds each. Keeps a node on the same
/// element while it stays chosen, so a pan does not rewrite every caption.
#[derive(Default)]
pub(crate) struct SlotAssigner {
    owner: Vec<Option<NodeKey>>,
    slot_of: HashMap<NodeKey, usize>,
}

/// What an [`SlotAssigner::assign`] changed.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Reassigned {
    pub released: Vec<usize>,
    pub taken: Vec<(usize, NodeKey)>,
}

impl SlotAssigner {
    /// Forget every holder and size the pool to `cap`.
    pub fn reset(&mut self, cap: usize) {
        self.owner.clear();
        self.owner.resize(cap, None);
        self.slot_of.clear();
    }

    pub fn capacity(&self) -> usize {
        self.owner.len()
    }

    pub fn slot(&self, key: NodeKey) -> Option<usize> {
        self.slot_of.get(&key).copied()
    }

    pub fn holds(&self, key: NodeKey) -> bool {
        self.slot_of.contains_key(&key)
    }

    pub fn len(&self) -> usize {
        self.slot_of.len()
    }

    /// Release holders not in `chosen`, then seat the rest in free elements.
    /// `chosen` longer than the pool is cut to fit.
    pub fn assign(&mut self, chosen: &[NodeKey]) -> Reassigned {
        let keep: HashSet<NodeKey> = chosen.iter().copied().take(self.owner.len()).collect();
        let mut out = Reassigned::default();
        for (slot, owner) in self.owner.iter_mut().enumerate() {
            if let Some(key) = *owner
                && !keep.contains(&key)
            {
                self.slot_of.remove(&key);
                *owner = None;
                out.released.push(slot);
            }
        }
        let mut free: std::vec::IntoIter<usize> = self
            .owner
            .iter()
            .enumerate()
            .filter(|(_, owner)| owner.is_none())
            .map(|(slot, _)| slot)
            .collect::<Vec<_>>()
            .into_iter();
        for &key in chosen.iter().take(self.owner.len()) {
            if self.slot_of.contains_key(&key) {
                continue;
            }
            let Some(slot) = free.next() else { break };
            self.owner[slot] = Some(key);
            self.slot_of.insert(key, slot);
            out.taken.push((slot, key));
        }
        out
    }

    /// Holders by key, for the canvas's gnode map.
    pub fn holders(&self) -> impl Iterator<Item = (NodeKey, usize)> + '_ {
        self.slot_of.iter().map(|(&key, &slot)| (key, slot))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kernel::geometry::PortablePoint;
    use kernel::graph::{Graph, apply};

    fn keys(n: usize) -> Vec<NodeKey> {
        let mut graph = Graph::new();
        (0..n)
            .map(|i| {
                apply::add_node(
                    &mut graph,
                    None,
                    format!("mere://n/{i}"),
                    PortablePoint::new(0.0, 0.0),
                )
            })
            .collect()
    }

    fn cand(key: NodeKey, side: f32, dist2: f32, priority: bool, assigned: bool) -> Candidate {
        Candidate {
            key,
            side_px: side,
            dist2,
            priority,
            assigned,
        }
    }

    #[test]
    fn the_set_is_bounded_and_priority_first() {
        let k = keys(10);
        let mut all: Vec<Candidate> = (0..10)
            .map(|i| cand(k[i], 20.0, i as f32, i == 9, false))
            .collect();
        let chosen = select_dom(&mut all, 3, 12.0);
        assert_eq!(chosen, vec![k[9], k[0], k[1]]);
    }

    #[test]
    fn small_nodes_qualify_only_by_priority() {
        let k = keys(3);
        let mut all = vec![
            cand(k[0], 4.0, 0.0, false, false),
            cand(k[1], 4.0, 1.0, true, false),
            cand(k[2], 14.0, 9.0, false, false),
        ];
        assert_eq!(select_dom(&mut all, 8, 12.0), vec![k[1], k[2]]);
    }

    #[test]
    fn a_holder_beats_a_nearer_newcomer_at_the_cap() {
        let k = keys(3);
        let mut all = vec![
            cand(k[0], 20.0, 1.0, false, false),
            cand(k[1], 20.0, 50.0, false, true),
            cand(k[2], 20.0, 2.0, false, false),
        ];
        assert_eq!(select_dom(&mut all, 2, 12.0), vec![k[1], k[0]]);
    }

    #[test]
    fn assignment_is_stable_across_frames() {
        let k = keys(6);
        let mut pool = SlotAssigner::default();
        pool.reset(3);
        let first = pool.assign(&[k[0], k[1], k[2]]);
        assert_eq!(first.taken.len(), 3);
        let slots: Vec<_> = k[..3].iter().map(|&key| pool.slot(key).unwrap()).collect();
        // Same set, reordered: nothing moves.
        let same = pool.assign(&[k[2], k[0], k[1]]);
        assert_eq!(same, Reassigned::default());
        // One out, one in: only that slot is rewritten, and the survivors stay.
        let swap = pool.assign(&[k[0], k[3], k[2]]);
        assert_eq!(swap.released, vec![slots[1]]);
        assert_eq!(swap.taken, vec![(slots[1], k[3])]);
        assert_eq!(pool.slot(k[0]), Some(slots[0]));
        assert_eq!(pool.slot(k[2]), Some(slots[2]));
        assert!(!pool.holds(k[1]));
        // Never more holders than elements.
        pool.assign(&k);
        assert_eq!(pool.len(), 3);
    }
}
