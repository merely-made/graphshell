// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The instance slot a node keeps within an epoch (engine binding D6).
//!
//! A build places nodes in the served layout's order, and that order moves
//! when a node arrives. The slot does not: a node keeps its slot for the
//! epoch, a new node appends one, and a node that leaves tombstones its slot
//! for good. Visual order is the item order, kept where it still agrees with
//! the layout, so an edit changes only the slots it touches.

use std::collections::HashMap;

use sceno::{Footprint, ProjectedItem, Representation, Scene, SourceIx, SourceRef, Transform2};
use scenotime::{SceneSnapshot, SnapshotError};
use uuid::Uuid;

/// Room left between neighbours when the order is numbered afresh.
const ORDER_SPACING: i32 = 1024;

#[derive(Debug, Default)]
pub(super) struct InstanceSlots {
    epoch: Option<u64>,
    slots: Vec<Slot>,
    of: HashMap<Uuid, u32>,
}

#[derive(Debug)]
struct Slot {
    /// The node holding the slot; `None` once it left.
    node: Option<Uuid>,
    /// Interned again for a slot whose node left, so the sources after it
    /// keep their indices.
    source: SourceRef,
    order: Option<i32>,
}

/// One build's placement, slot by slot.
#[derive(Debug)]
pub(super) struct Placed {
    /// Each slot's index in the layout's order; `None` for a node that left.
    pub rank: Vec<Option<usize>>,
    pub source: Vec<SourceRef>,
    pub order: Vec<Option<i32>>,
}

impl InstanceSlots {
    /// Place the layout's nodes, in its order, into their slots for `epoch`.
    /// Another epoch starts the slots afresh.
    pub fn place(&mut self, epoch: u64, layout: &[(Uuid, SourceRef)]) -> Placed {
        if self.epoch != Some(epoch) {
            *self = Self {
                epoch: Some(epoch),
                ..Self::default()
            };
        }
        let mut rank = vec![None; self.slots.len()];
        let mut by_rank = Vec::with_capacity(layout.len());
        for (index, (id, source)) in layout.iter().enumerate() {
            let slot = match self.of.get(id) {
                Some(&slot) => slot as usize,
                None => {
                    self.of.insert(*id, self.slots.len() as u32);
                    self.slots.push(Slot {
                        node: Some(*id),
                        source: source.clone(),
                        order: None,
                    });
                    rank.push(None);
                    self.slots.len() - 1
                },
            };
            rank[slot] = Some(index);
            by_rank.push(slot);
        }
        for (slot, placed) in self.slots.iter_mut().zip(&rank) {
            if placed.is_none() {
                if let Some(id) = slot.node.take() {
                    self.of.remove(&id);
                }
                slot.order = None;
            }
        }
        self.order(&by_rank);
        Placed {
            rank,
            source: self.slots.iter().map(|slot| slot.source.clone()).collect(),
            order: self.slots.iter().map(|slot| slot.order).collect(),
        }
    }

    /// Give every placed slot an order that rises along the layout's order,
    /// keeping the orders already held where they still rise and fitting new
    /// slots into the gaps; numbered afresh when they do not fit.
    fn order(&mut self, by_rank: &[usize]) {
        let held: Vec<i32> = by_rank
            .iter()
            .filter_map(|&slot| self.slots[slot].order)
            .collect();
        if held.windows(2).all(|pair| pair[0] < pair[1]) && self.fill_gaps(by_rank) {
            return;
        }
        for (index, &slot) in by_rank.iter().enumerate() {
            self.slots[slot].order = Some((index as i32).saturating_mul(ORDER_SPACING));
        }
    }

    /// Fit each run of unordered slots between its ordered neighbours.
    /// `false` when a run does not fit.
    fn fill_gaps(&mut self, by_rank: &[usize]) -> bool {
        let mut fills = Vec::new();
        let mut index = 0;
        while index < by_rank.len() {
            if self.slots[by_rank[index]].order.is_some() {
                index += 1;
                continue;
            }
            let start = index;
            while index < by_rank.len() && self.slots[by_rank[index]].order.is_none() {
                index += 1;
            }
            let run = (index - start) as i64;
            let before = start
                .checked_sub(1)
                .and_then(|at| self.slots[by_rank[at]].order)
                .map(i64::from);
            let after = by_rank
                .get(index)
                .and_then(|&slot| self.slots[slot].order)
                .map(i64::from);
            let first = match (before, after) {
                // Nothing ordered yet: number afresh, with room.
                (None, None) => return false,
                (Some(low), None) => low + 1,
                (None, Some(high)) => high - run,
                (Some(low), Some(high)) if high - low > run => low + 1,
                (Some(_), Some(_)) => return false,
            };
            if first < i64::from(i32::MIN) + 1 || first + run > i64::from(i32::MAX) {
                return false;
            }
            fills.extend((start..index).zip(first..));
        }
        for (at, order) in fills {
            self.slots[by_rank[at]].order = Some(order as i32);
        }
        true
    }
}

/// The item a slot whose node left holds until [`settle`] tombstones it.
pub(super) fn left(source: SourceIx) -> ProjectedItem {
    ProjectedItem {
        source,
        space: Scene::WORLD,
        transform: Transform2::translation(0.0, 0.0),
        footprint: Footprint::Point,
        representation: Representation::Glyph,
        layer: 0,
        visible: false,
        hit: None,
        channels: Vec::new(),
    }
}

/// Give a dense build its slots' item orders and tombstone the slots whose
/// nodes left. Node slots start at `offset`; the session item, before them,
/// is drawn first.
pub(super) fn settle(
    snapshot: &mut SceneSnapshot,
    placed: &Placed,
    offset: u32,
    session_item: bool,
) -> Result<(), SnapshotError> {
    let tables = &mut snapshot.tables;
    if session_item {
        tables.item_order[0] = Some(i32::MIN);
    }
    for (slot, order) in placed.order.iter().enumerate() {
        let at = slot + offset as usize;
        tables.item_order[at] = *order;
        if order.is_none() {
            tables.items[at] = None;
        }
    }
    snapshot.validate()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout(ids: &[u128]) -> Vec<(Uuid, SourceRef)> {
        ids.iter()
            .map(|&n| {
                let id = Uuid::from_u128(n);
                (id, SourceRef::new("mere.graph", id.to_string()))
            })
            .collect()
    }

    /// Each node's order, in the layout's order.
    fn orders(placed: &Placed, ids: &[u128], slots: &InstanceSlots) -> Vec<i32> {
        ids.iter()
            .map(|&n| placed.order[slots.of[&Uuid::from_u128(n)] as usize].unwrap())
            .collect()
    }

    #[test]
    fn a_node_keeps_its_slot_and_a_new_one_appends() {
        let mut slots = InstanceSlots::default();
        let first = slots.place(1, &layout(&[3, 2, 1]));
        assert_eq!(first.rank, [Some(0), Some(1), Some(2)]);
        // The newest first: the layout moves, the slots do not.
        let second = slots.place(1, &layout(&[4, 3, 2, 1]));
        assert_eq!(second.rank, [Some(1), Some(2), Some(3), Some(0)]);
        let ids = [4, 3, 2, 1];
        let held = orders(&second, &ids, &slots);
        assert!(held.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(second.order[..3], first.order[..], "the others kept their order");
    }

    #[test]
    fn a_node_that_leaves_tombstones_its_slot_for_good() {
        let mut slots = InstanceSlots::default();
        slots.place(1, &layout(&[1, 2, 3]));
        let placed = slots.place(1, &layout(&[1, 3]));
        assert_eq!(placed.rank, [Some(0), None, Some(1)]);
        assert_eq!(placed.order[1], None);
        assert_eq!(placed.source.len(), 3, "the slot keeps its source");
        let back = slots.place(1, &layout(&[1, 2, 3]));
        assert_eq!(back.rank, [Some(0), None, Some(2), Some(1)], "a new slot");
        let ids = [1, 2, 3];
        assert!(orders(&back, &ids, &slots).windows(2).all(|p| p[0] < p[1]));
    }

    #[test]
    fn a_reordered_layout_is_numbered_afresh_and_a_new_epoch_starts_over() {
        let mut slots = InstanceSlots::default();
        slots.place(1, &layout(&[1, 2, 3]));
        let swapped = slots.place(1, &layout(&[3, 1, 2]));
        let ids = [3, 1, 2];
        assert!(orders(&swapped, &ids, &slots).windows(2).all(|p| p[0] < p[1]));
        let fresh = slots.place(2, &layout(&[3, 1, 2]));
        assert_eq!(fresh.rank, [Some(0), Some(1), Some(2)]);
    }
}
