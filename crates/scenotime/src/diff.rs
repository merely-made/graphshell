// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

use sceno::{
    Backdrop, Fold, InstanceId, ProjectedItem, Rect, Region, RoutedRelation, SourceIx, SourceRef,
    Space, SpaceId, StandIn,
};
use serde::{Deserialize, Serialize};

use crate::{
    BackdropId, EngineRevision, FoldId, RegionId, RelationId, Revision, SceneEpoch, SceneSnapshot,
    SnapshotError,
};

/// One idempotent transition within a scene epoch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneDiff {
    pub epoch: SceneEpoch,
    pub base: Revision,
    pub revision: Revision,
    pub operations: Vec<SceneOp>,
    /// The engine revision of the scene this diff applies to, named with
    /// [`Self::engine_revision`] or not at all (engine binding E3). Absent
    /// from the wire when unset, as on [`SceneSnapshot`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_engine: Option<EngineRevision>,
    /// The engine revision of the scene this diff produces.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine_revision: Option<EngineRevision>,
}

impl SceneDiff {
    /// A diff that names no engine revision.
    pub fn new(
        epoch: SceneEpoch,
        base: Revision,
        revision: Revision,
        operations: Vec<SceneOp>,
    ) -> Self {
        Self {
            epoch,
            base,
            revision,
            operations,
            base_engine: None,
            engine_revision: None,
        }
    }

    /// This diff, naming the engine revisions of its base and target.
    pub fn with_engine(mut self, base: EngineRevision, target: EngineRevision) -> Self {
        self.base_engine = Some(base);
        self.engine_revision = Some(target);
        self
    }
}

/// Stable-slot scene changes. Additions append at the named index; tombstones
/// remain allocated until a new epoch begins.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum SceneOp {
    AddSource {
        index: SourceIx,
        value: SourceRef,
    },
    UpdateSource {
        index: SourceIx,
        value: SourceRef,
    },
    TombstoneSource {
        index: SourceIx,
    },
    AddSpace {
        index: SpaceId,
        value: Space,
    },
    UpdateSpace {
        index: SpaceId,
        value: Space,
    },
    TombstoneSpace {
        index: SpaceId,
    },
    AddBackdrop {
        index: BackdropId,
        value: Backdrop,
    },
    UpdateBackdrop {
        index: BackdropId,
        value: Backdrop,
    },
    TombstoneBackdrop {
        index: BackdropId,
    },
    AddItem {
        index: InstanceId,
        value: ProjectedItem,
        order: i32,
    },
    UpdateItem {
        index: InstanceId,
        value: ProjectedItem,
    },
    TombstoneItem {
        index: InstanceId,
    },
    SetItemLayer {
        index: InstanceId,
        layer: i16,
    },
    SetItemOrder {
        index: InstanceId,
        order: i32,
    },
    AddRelation {
        index: RelationId,
        value: RoutedRelation,
    },
    UpdateRelation {
        index: RelationId,
        value: RoutedRelation,
    },
    TombstoneRelation {
        index: RelationId,
    },
    AddRegion {
        index: RegionId,
        value: Region,
    },
    UpdateRegion {
        index: RegionId,
        value: Region,
    },
    TombstoneRegion {
        index: RegionId,
    },
    /// Fold: the members are hidden behind the stand-in from this revision.
    /// Refolding after an unfold appends a new slot.
    AddFold {
        index: FoldId,
        value: Fold,
    },
    UpdateFold {
        index: FoldId,
        value: Fold,
    },
    /// Change only what is drawn in the fold's place.
    SetFoldStandIn {
        index: FoldId,
        stand_in: StandIn,
    },
    /// Unfold: the fact goes, and the members show as they were.
    TombstoneFold {
        index: FoldId,
    },
    /// Replace placed-relation outcomes and arrangement slot order together.
    SetRelationConstraints {
        reports: Vec<sceno::RelationConstraintReport>,
        order: Vec<InstanceId>,
    },
    SetFrame {
        frame: Option<sceno::SceneFrame>,
    },
    SetBounds {
        bounds: Rect,
    },
    SetGeneration {
        generation: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyOutcome {
    Applied,
    AlreadyApplied,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffError {
    WrongEpoch {
        current: SceneEpoch,
        received: SceneEpoch,
    },
    MissingBase {
        current: Revision,
        required: Revision,
    },
    InvalidRevision {
        base: Revision,
        revision: Revision,
    },
    /// The diff names an engine revision other than the one the scene holds,
    /// at its base or, for a repeat of the held revision, at its target.
    Diverged {
        held: Option<EngineRevision>,
        named: Option<EngineRevision>,
    },
    InvalidOperation(String),
    InvalidSnapshot(SnapshotError),
}

impl SceneSnapshot {
    /// Apply a diff transactionally. A repeated or older target revision is a
    /// successful no-op; a future diff with a missing base requests resync.
    pub fn apply_diff(&mut self, diff: &SceneDiff) -> Result<ApplyOutcome, DiffError> {
        if self.check_diff(diff)? == ApplyOutcome::AlreadyApplied {
            return Ok(ApplyOutcome::AlreadyApplied);
        }
        let mut next = self.clone();
        next.apply_checked(diff)?;
        *self = next;
        Ok(ApplyOutcome::Applied)
    }

    /// Apply a diff in place, to a scene the caller has staged for a whole
    /// suffix, so the suffix costs one clone rather than one per diff
    /// (engine split D201). Checks as [`Self::apply_diff`] does; after an
    /// error the scene may be part changed and the caller discards it.
    ///
    /// # Errors
    ///
    /// As [`Self::apply_diff`], and [`DiffError::Diverged`] when the diff
    /// names another engine revision than the scene holds.
    pub fn apply_diff_staged(&mut self, diff: &SceneDiff) -> Result<ApplyOutcome, DiffError> {
        if self.check_diff(diff)? == ApplyOutcome::AlreadyApplied {
            return Ok(ApplyOutcome::AlreadyApplied);
        }
        self.apply_checked(diff)?;
        Ok(ApplyOutcome::Applied)
    }

    /// Check a diff's epoch, base and target, and the engine revisions it
    /// names, before anything is applied.
    fn check_diff(&self, diff: &SceneDiff) -> Result<ApplyOutcome, DiffError> {
        if self.epoch != diff.epoch {
            return Err(DiffError::WrongEpoch {
                current: self.epoch,
                received: diff.epoch,
            });
        }
        let diverged = |named: &Option<EngineRevision>| DiffError::Diverged {
            held: self.engine_revision.clone(),
            named: named.clone(),
        };
        if diff.revision <= self.revision {
            // A repeat of the held revision must name the identity held.
            if diff.revision == self.revision
                && diff.engine_revision.is_some()
                && diff.engine_revision != self.engine_revision
            {
                return Err(diverged(&diff.engine_revision));
            }
            return Ok(ApplyOutcome::AlreadyApplied);
        }
        if diff.base != self.revision {
            return Err(DiffError::MissingBase {
                current: self.revision,
                required: diff.base,
            });
        }
        if diff.revision <= diff.base {
            return Err(DiffError::InvalidRevision {
                base: diff.base,
                revision: diff.revision,
            });
        }
        if diff.base_engine.is_some() != diff.engine_revision.is_some() {
            return operation_error("a diff names its engine revision at both ends or neither");
        }
        if diff.base_engine.is_some() && diff.base_engine != self.engine_revision {
            return Err(diverged(&diff.base_engine));
        }
        Ok(ApplyOutcome::Applied)
    }

    fn apply_checked(&mut self, diff: &SceneDiff) -> Result<(), DiffError> {
        for operation in &diff.operations {
            self.apply_operation(operation)?;
        }
        self.validate().map_err(DiffError::InvalidSnapshot)?;
        self.revision = diff.revision;
        if let Some(engine) = &diff.engine_revision {
            self.engine_revision = Some(engine.clone());
        }
        Ok(())
    }

    fn apply_operation(&mut self, operation: &SceneOp) -> Result<(), DiffError> {
        let tables = &mut self.tables;
        match operation {
            SceneOp::AddSource { index, value } => {
                append(&mut tables.sources, index.0, value.clone(), "source")
            },
            SceneOp::UpdateSource { index, value } => {
                update(&mut tables.sources, index.0, value.clone(), "source")
            },
            SceneOp::TombstoneSource { index } => tombstone(&mut tables.sources, index.0, "source"),
            SceneOp::AddSpace { index, value } => {
                append(&mut tables.spaces, index.0, value.clone(), "space")
            },
            SceneOp::UpdateSpace { index, value } => {
                update(&mut tables.spaces, index.0, value.clone(), "space")
            },
            SceneOp::TombstoneSpace { index } => tombstone(&mut tables.spaces, index.0, "space"),
            SceneOp::AddBackdrop { index, value } => {
                append(&mut tables.backdrops, index.0, value.clone(), "backdrop")
            },
            SceneOp::UpdateBackdrop { index, value } => {
                update(&mut tables.backdrops, index.0, value.clone(), "backdrop")
            },
            SceneOp::TombstoneBackdrop { index } => {
                tombstone(&mut tables.backdrops, index.0, "backdrop")
            },
            SceneOp::AddItem {
                index,
                value,
                order,
            } => {
                append(&mut tables.items, index.0, value.clone(), "item")?;
                append(&mut tables.item_order, index.0, *order, "item order")
            },
            SceneOp::UpdateItem { index, value } => {
                update(&mut tables.items, index.0, value.clone(), "item")
            },
            SceneOp::TombstoneItem { index } => {
                tombstone(&mut tables.items, index.0, "item")?;
                tombstone(&mut tables.item_order, index.0, "item order")
            },
            SceneOp::SetItemLayer { index, layer } => {
                active_mut(&mut tables.items, index.0, "item")?.layer = *layer;
                Ok(())
            },
            SceneOp::SetItemOrder { index, order } => {
                *active_mut(&mut tables.item_order, index.0, "item order")? = *order;
                Ok(())
            },
            SceneOp::AddRelation { index, value } => {
                append(&mut tables.relations, index.0, value.clone(), "relation")
            },
            SceneOp::UpdateRelation { index, value } => {
                update(&mut tables.relations, index.0, value.clone(), "relation")
            },
            SceneOp::TombstoneRelation { index } => {
                tombstone(&mut tables.relations, index.0, "relation")
            },
            SceneOp::AddRegion { index, value } => {
                append(&mut tables.regions, index.0, value.clone(), "region")
            },
            SceneOp::UpdateRegion { index, value } => {
                update(&mut tables.regions, index.0, value.clone(), "region")
            },
            SceneOp::TombstoneRegion { index } => tombstone(&mut tables.regions, index.0, "region"),
            SceneOp::AddFold { index, value } => {
                append(&mut tables.folds, index.0, value.clone(), "fold")
            },
            SceneOp::UpdateFold { index, value } => {
                update(&mut tables.folds, index.0, value.clone(), "fold")
            },
            SceneOp::SetFoldStandIn { index, stand_in } => {
                active_mut(&mut tables.folds, index.0, "fold")?.stand_in = stand_in.clone();
                Ok(())
            },
            SceneOp::TombstoneFold { index } => tombstone(&mut tables.folds, index.0, "fold"),
            SceneOp::SetRelationConstraints { reports, order } => {
                self.tables.relation_constraints = reports.clone();
                self.tables.arrangement_order = order.clone();
                Ok(())
            },
            SceneOp::SetFrame { frame } => {
                self.tables.frame = frame.clone();
                Ok(())
            },
            SceneOp::SetBounds { bounds } => {
                tables.bounds = *bounds;
                Ok(())
            },
            SceneOp::SetGeneration { generation } => {
                tables.generation = *generation;
                Ok(())
            },
        }
    }
}

fn append<T>(
    slots: &mut Vec<Option<T>>,
    index: u32,
    value: T,
    table: &str,
) -> Result<(), DiffError> {
    if index as usize != slots.len() {
        return operation_error(format!(
            "{table} add index {index} must append at {}; tombstones are not reusable",
            slots.len()
        ));
    }
    slots.push(Some(value));
    Ok(())
}

fn update<T>(slots: &mut [Option<T>], index: u32, value: T, table: &str) -> Result<(), DiffError> {
    *active_mut(slots, index, table)? = value;
    Ok(())
}

fn tombstone<T>(slots: &mut [Option<T>], index: u32, table: &str) -> Result<(), DiffError> {
    let Some(slot) = slots.get_mut(index as usize) else {
        return operation_error(format!("{table} tombstone index {index} is out of range"));
    };
    *slot = None;
    Ok(())
}

fn active_mut<'a, T>(
    slots: &'a mut [Option<T>],
    index: u32,
    table: &str,
) -> Result<&'a mut T, DiffError> {
    slots
        .get_mut(index as usize)
        .and_then(Option::as_mut)
        .ok_or_else(|| {
            DiffError::InvalidOperation(format!("{table} index {index} is absent or tombstoned"))
        })
}

fn operation_error<T>(message: impl Into<String>) -> Result<T, DiffError> {
    Err(DiffError::InvalidOperation(message.into()))
}

#[cfg(test)]
#[path = "diff/tests.rs"]
mod tests;
