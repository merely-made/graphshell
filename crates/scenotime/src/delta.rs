// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The diff between two snapshots of one epoch, for a producer that builds
//! whole scenes and retains the deltas between them (engine binding E3).
//!
//! Slots are compared by index. A slot present in both and changed is an
//! update; a slot past the older table is an append; a slot the newer table no
//! longer reaches is a tombstone. What a diff cannot say comes back as `None`:
//! a tombstoned slot filled again, a tombstone past the older table, or a
//! change to the held placements, which no operation carries.

use sceno::InstanceId;

use crate::{
    BackdropId, FoldId, RegionId, RelationId, SceneDiff, SceneOp, SceneSnapshot, SceneTables,
};

impl SceneSnapshot {
    /// The diff that takes this snapshot to `next`, naming both engine
    /// revisions when both snapshots carry one. `None` when the two are not
    /// one epoch, `next` is not later, only one names an engine revision, or
    /// the change is one a diff cannot express.
    pub fn diff_to(&self, next: &SceneSnapshot) -> Option<SceneDiff> {
        if next.epoch != self.epoch
            || next.revision <= self.revision
            || next.engine_revision.is_some() != self.engine_revision.is_some()
        {
            return None;
        }
        let diff = SceneDiff::new(
            self.epoch,
            self.revision,
            next.revision,
            self.operations_to(next)?,
        );
        Some(match (&self.engine_revision, &next.engine_revision) {
            (Some(base), Some(target)) => diff.with_engine(base.clone(), target.clone()),
            _ => diff,
        })
    }

    /// The operations that take this snapshot's tables to `next`'s, whatever
    /// the revisions. Empty when the tables already agree.
    pub fn operations_to(&self, next: &SceneSnapshot) -> Option<Vec<SceneOp>> {
        operations(&self.tables, &next.tables)
    }
}

fn operations(prev: &SceneTables, next: &SceneTables) -> Option<Vec<SceneOp>> {
    if prev.unmet_holds != next.unmet_holds || prev.honored_holds != next.honored_holds {
        return None;
    }
    let mut ops = Vec::new();
    if prev.frame != next.frame {
        ops.push(SceneOp::SetFrame {
            frame: next.frame.clone(),
        });
    }
    slots(
        &prev.sources,
        &next.sources,
        &mut ops,
        |i, value| SceneOp::AddSource {
            index: sceno::SourceIx(i),
            value,
        },
        |i, value| SceneOp::UpdateSource {
            index: sceno::SourceIx(i),
            value,
        },
        |i| SceneOp::TombstoneSource {
            index: sceno::SourceIx(i),
        },
    )?;
    slots(
        &prev.spaces,
        &next.spaces,
        &mut ops,
        |i, value| SceneOp::AddSpace {
            index: sceno::SpaceId(i),
            value,
        },
        |i, value| SceneOp::UpdateSpace {
            index: sceno::SpaceId(i),
            value,
        },
        |i| SceneOp::TombstoneSpace {
            index: sceno::SpaceId(i),
        },
    )?;
    slots(
        &prev.backdrops,
        &next.backdrops,
        &mut ops,
        |i, value| SceneOp::AddBackdrop {
            index: BackdropId(i),
            value,
        },
        |i, value| SceneOp::UpdateBackdrop {
            index: BackdropId(i),
            value,
        },
        |i| SceneOp::TombstoneBackdrop {
            index: BackdropId(i),
        },
    )?;
    items(prev, next, &mut ops)?;
    figures(prev, next, &mut ops)?;
    scalars(prev, next, &mut ops);
    Some(ops)
}

/// Relations, regions and folds.
fn figures(prev: &SceneTables, next: &SceneTables, ops: &mut Vec<SceneOp>) -> Option<()> {
    slots(
        &prev.relations,
        &next.relations,
        ops,
        |i, value| SceneOp::AddRelation {
            index: RelationId(i),
            value,
        },
        |i, value| SceneOp::UpdateRelation {
            index: RelationId(i),
            value,
        },
        |i| SceneOp::TombstoneRelation {
            index: RelationId(i),
        },
    )?;
    slots(
        &prev.regions,
        &next.regions,
        ops,
        |i, value| SceneOp::AddRegion {
            index: RegionId(i),
            value,
        },
        |i, value| SceneOp::UpdateRegion {
            index: RegionId(i),
            value,
        },
        |i| SceneOp::TombstoneRegion {
            index: RegionId(i),
        },
    )?;
    slots(
        &prev.folds,
        &next.folds,
        ops,
        |i, value| SceneOp::AddFold {
            index: FoldId(i),
            value,
        },
        |i, value| SceneOp::UpdateFold {
            index: FoldId(i),
            value,
        },
        |i| SceneOp::TombstoneFold { index: FoldId(i) },
    )
}

/// The whole-scene values: placed-relation outcomes, bounds, generation.
fn scalars(prev: &SceneTables, next: &SceneTables, ops: &mut Vec<SceneOp>) {
    if prev.relation_constraints != next.relation_constraints
        || prev.arrangement_order != next.arrangement_order
    {
        ops.push(SceneOp::SetRelationConstraints {
            reports: next.relation_constraints.clone(),
            order: next.arrangement_order.clone(),
        });
    }
    if prev.bounds != next.bounds {
        ops.push(SceneOp::SetBounds {
            bounds: next.bounds,
        });
    }
    if prev.generation != next.generation {
        ops.push(SceneOp::SetGeneration {
            generation: next.generation,
        });
    }
}

/// One stable-slot table. `None` when a tombstone would be filled again or
/// a tombstone lies past the older table, neither of which a diff can say.
fn slots<T: Clone + PartialEq>(
    prev: &[Option<T>],
    next: &[Option<T>],
    ops: &mut Vec<SceneOp>,
    add: impl Fn(u32, T) -> SceneOp,
    update: impl Fn(u32, T) -> SceneOp,
    tombstone: impl Fn(u32) -> SceneOp,
) -> Option<()> {
    for (index, slot) in next.iter().enumerate() {
        let i = index as u32;
        match (prev.get(index), slot) {
            (Some(held), slot) if held == slot => {},
            (Some(Some(_)), Some(value)) => ops.push(update(i, value.clone())),
            (Some(Some(_)), None) => ops.push(tombstone(i)),
            (None, Some(value)) => ops.push(add(i, value.clone())),
            (Some(None), Some(_)) | (None, None) | (Some(None), None) => return None,
        }
    }
    for (index, held) in prev.iter().enumerate().skip(next.len()) {
        if held.is_some() {
            ops.push(tombstone(index as u32));
        }
    }
    Some(())
}

/// Items and their order, which share a slot.
fn items(prev: &SceneTables, next: &SceneTables, ops: &mut Vec<SceneOp>) -> Option<()> {
    for (index, slot) in next.items.iter().enumerate() {
        let id = InstanceId(index as u32);
        let order = *next.item_order.get(index)?;
        match (prev.items.get(index), slot) {
            (Some(Some(held)), Some(value)) => {
                if held != value {
                    ops.push(SceneOp::UpdateItem {
                        index: id,
                        value: value.clone(),
                    });
                }
                if prev.item_order.get(index) != Some(&order) {
                    ops.push(SceneOp::SetItemOrder {
                        index: id,
                        order: order?,
                    });
                }
            },
            (Some(Some(_)), None) => ops.push(SceneOp::TombstoneItem { index: id }),
            (Some(None), None) => {},
            (None, Some(value)) => ops.push(SceneOp::AddItem {
                index: id,
                value: value.clone(),
                order: order?,
            }),
            (Some(None), Some(_)) | (None, None) => return None,
        }
    }
    for (index, held) in prev.items.iter().enumerate().skip(next.items.len()) {
        if held.is_some() {
            ops.push(SceneOp::TombstoneItem {
                index: InstanceId(index as u32),
            });
        }
    }
    Some(())
}

#[cfg(test)]
#[path = "delta/tests.rs"]
mod tests;
