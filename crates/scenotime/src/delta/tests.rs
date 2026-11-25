// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

use super::*;
use crate::{ApplyOutcome, DiffError, EngineRevision, Revision, SceneEpoch};
use sceno::{
    Footprint, ProjectedItem, Representation, RoutedRelation, Scene, Size2, SourceRef, Transform2,
};

fn engine(cursor: u64) -> EngineRevision {
    EngineRevision {
        session: "session:e3".into(),
        cursor,
        digest: format!("{cursor:064x}"),
    }
}

/// `count` items in a row, each related to the next, as a dense build.
fn built(revision: u64, count: usize, shift: f32) -> SceneSnapshot {
    let mut scene = Scene::new();
    for i in 0..count {
        let source = scene.intern_source(SourceRef::new("fixture", format!("n{i}")));
        scene.items.push(ProjectedItem {
            source,
            space: Scene::WORLD,
            transform: Transform2::translation(i as f32 * 10.0 + shift, 0.0),
            footprint: Footprint::Rect {
                size: Size2::new(8.0, 8.0),
            },
            representation: Representation::Glyph,
            layer: 0,
            visible: true,
            hit: None,
            channels: Vec::new(),
        });
    }
    for i in 1..count {
        scene.relations.push(RoutedRelation {
            form: Default::default(),
            from: InstanceId(i as u32 - 1),
            to: InstanceId(i as u32),
            space: Scene::WORLD,
            points: vec![sceno::Vec2::new(0.0, 0.0), sceno::Vec2::new(shift, 0.0)],
            kind: None,
            weight: None,
            link_key: None,
            marks: sceno::LinkMarks::default(),
            strength: None,
        });
    }
    scene.generation = revision;
    SceneSnapshot::from_dense(SceneEpoch(1), Revision(revision), scene)
        .unwrap()
        .with_engine_revision(engine(revision))
}

fn applied(prev: &SceneSnapshot, next: &SceneSnapshot) -> SceneSnapshot {
    let diff = prev.diff_to(next).expect("expressible");
    let mut held = prev.clone();
    assert_eq!(held.apply_diff(&diff), Ok(ApplyOutcome::Applied));
    held
}

#[test]
fn a_diff_takes_a_snapshot_to_the_next_build() {
    let prev = built(1, 3, 0.0);
    let next = built(2, 5, 4.0);
    let held = applied(&prev, &next);
    assert_eq!(held, next, "growth and movement land exactly");
    assert_eq!(held.engine_revision, Some(engine(2)), "the diff restamps");
    assert!(prev.operations_to(&prev).unwrap().is_empty());
}

#[test]
fn a_shorter_build_tombstones_the_slots_it_no_longer_reaches() {
    let prev = built(1, 5, 0.0);
    let next = built(2, 3, 0.0);
    let held = applied(&prev, &next);
    assert_eq!(held.tables.items.len(), 5);
    assert!(held.tables.items[3].is_none() && held.tables.items[4].is_none());
    assert_eq!(held.tables.items[..3], next.tables.items[..]);
    assert_eq!(held.active_item_count(), 3);
    // Growing again would fill a tombstone, which a diff cannot say.
    assert!(held.diff_to(&built(3, 5, 0.0)).is_none());
    assert!(held.diff_to(&built(3, 3, 1.0)).is_some());
}

#[test]
fn what_no_operation_carries_is_not_a_diff() {
    let prev = built(1, 2, 0.0);
    let mut next = built(2, 2, 0.0);
    next.tables.unmet_holds.push(sceno::HeldPlacement::pinned(
        SourceRef::new("fixture", "n0"),
        sceno::Vec2::new(1.0, 1.0),
    ));
    assert!(prev.diff_to(&next).is_none());
    let mut older = built(2, 2, 0.0);
    older.revision = Revision(1);
    assert!(built(1, 2, 0.0).diff_to(&older).is_none(), "not later");
    let mut unnamed = built(2, 2, 0.0);
    unnamed.engine_revision = None;
    assert!(prev.diff_to(&unnamed).is_none(), "one end named only");
}

#[test]
fn a_staged_suffix_matches_diff_by_diff_application() {
    let builds = [built(1, 2, 0.0), built(2, 4, 1.0), built(3, 3, 2.0), built(4, 3, 3.0)];
    let mut one_by_one = builds[0].clone();
    let mut staged = builds[0].clone();
    for next in &builds[1..] {
        let diff = one_by_one.diff_to(next).unwrap();
        one_by_one.apply_diff(&diff).unwrap();
        assert_eq!(staged.apply_diff_staged(&diff), Ok(ApplyOutcome::Applied));
    }
    assert_eq!(staged, one_by_one);
    let last = &builds[3];
    assert_eq!(staged.tables.items[..3], last.tables.items[..]);
    assert!(staged.tables.items[3].is_none(), "the shrink left its tombstone");
    assert_eq!(staged.engine_revision, last.engine_revision);
}

#[test]
fn every_base_and_target_identity_is_checked() {
    let prev = built(1, 2, 0.0);
    let next = built(2, 3, 0.0);
    let diff = prev.diff_to(&next).unwrap();

    let mut divergent = prev.clone();
    divergent.engine_revision = Some(engine(9));
    let before = divergent.clone();
    assert!(matches!(
        divergent.apply_diff_staged(&diff),
        Err(DiffError::Diverged { .. })
    ));
    assert_eq!(divergent, before, "refused before anything applied");

    let mut half = diff.clone();
    half.base_engine = None;
    assert!(matches!(
        prev.clone().apply_diff(&half),
        Err(DiffError::InvalidOperation(_))
    ));

    let mut held = prev.clone();
    held.apply_diff(&diff).unwrap();
    assert_eq!(held.apply_diff(&diff), Ok(ApplyOutcome::AlreadyApplied));
    let mut other = diff.clone();
    other.engine_revision = Some(engine(7));
    assert!(matches!(
        held.apply_diff(&other),
        Err(DiffError::Diverged { .. })
    ));
}

#[test]
fn a_diff_without_identities_still_applies_and_round_trips() {
    let mut prev = built(1, 2, 0.0);
    let mut next = built(2, 2, 3.0);
    prev.engine_revision = None;
    next.engine_revision = None;
    let diff = prev.diff_to(&next).unwrap();
    let wire = serde_json::to_string(&diff).unwrap();
    assert!(!wire.contains("engine"), "absent from the wire when unset");
    let back: SceneDiff = serde_json::from_str(&wire).unwrap();
    assert_eq!(back, diff);
    assert_eq!(applied(&prev, &next), next);
}
