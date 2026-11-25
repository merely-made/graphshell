// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

use super::*;
use crate::{
    PresentationBinding, PresentationKey, PresentationManifest, ProtocolVersion, SceneSnapshot,
};
use sceno::{
    Footprint, InstanceId, ProjectedItem, Representation, Scene, Size2, SourceRef, Transform2,
};

fn engine(cursor: u64) -> EngineRevision {
    EngineRevision {
        session: "session:e3".into(),
        cursor,
        digest: format!("{cursor:064x}"),
    }
}

fn session() -> ProjectionSession {
    ProjectionSession("fixture:e3".into())
}

/// A dense build of `count` items, each bound to a key, the first moved by
/// `shift`: each build differs from the last by a small delta.
fn built(revision: u64, count: usize, shift: f32) -> ProjectionSnapshot {
    build(revision, count, shift, false)
}

/// A dense build with every item moved by `shift`: a relayout.
fn relaid(revision: u64, count: usize, shift: f32) -> ProjectionSnapshot {
    build(revision, count, shift, true)
}

fn build(revision: u64, count: usize, shift: f32, all: bool) -> ProjectionSnapshot {
    let mut scene = Scene::new();
    let mut presentation = PresentationManifest::default();
    for i in 0..count {
        let source = scene.intern_source(SourceRef::new("fixture", format!("n{i}")));
        scene.items.push(ProjectedItem {
            source,
            space: Scene::WORLD,
            transform: Transform2::translation(i as f32 + if all || i == 0 { shift } else { 0.0 }, 0.0),
            footprint: Footprint::Rect {
                size: Size2::new(8.0, 8.0),
            },
            representation: Representation::Glyph,
            layer: 0,
            visible: true,
            hit: None,
            channels: Vec::new(),
        });
        presentation.bindings.push(PresentationBinding {
            instance: InstanceId(i as u32),
            key: PresentationKey(format!("k{i}")),
        });
    }
    scene.generation = revision;
    ProjectionSnapshot {
        version: ProtocolVersion::V1,
        session: session(),
        scene: SceneSnapshot::from_dense(SceneEpoch(1), Revision(revision), scene)
            .unwrap()
            .with_engine_revision(engine(revision)),
        presentation,
        cache_policy: Default::default(),
    }
}

fn request(revision: u64, engine_revision: Option<EngineRevision>) -> ResumeRequest {
    ResumeRequest::new(session(), SceneEpoch(1), Revision(revision)).with_engine(engine_revision)
}

#[test]
fn the_held_scene_advances_by_the_retained_delta() {
    let mut retention = DeltaRetention::default();
    assert_eq!(retention.advance(built(1, 3, 0.0)), Advance::Restarted);
    assert_eq!(retention.advance(built(1, 3, 0.0)), Advance::Unchanged);
    assert_eq!(retention.advance(built(2, 4, 1.0)), Advance::Retained);
    assert_eq!(retention.keyframe(), Some(&built(2, 4, 1.0)));
    // A shorter build leaves tombstones in the held scene, and the unbound
    // instance is unbound there.
    assert_eq!(retention.advance(built(3, 2, 1.0)), Advance::Retained);
    let held = retention.keyframe().unwrap();
    assert_eq!(held.scene.tables.items.len(), 4);
    assert_eq!(held.scene.active_item_count(), 2);
    assert_eq!(held.presentation.bindings.len(), 2);
    // Filling a tombstone again is no delta: restart from the build.
    assert_eq!(retention.advance(built(4, 4, 1.0)), Advance::Restarted);
    assert!(retention.is_empty());
}

#[test]
fn a_resume_gets_the_verified_suffix_current_or_a_keyframe() {
    let mut retention = DeltaRetention::default();
    for revision in 1..=4 {
        retention.advance(built(revision, 40, revision as f32));
    }
    assert_eq!(retention.len(), 3);
    let Some(ResumeReply::Diffs(diffs)) = retention.resume(&request(2, Some(engine(2)))) else {
        panic!("a retained base gets its suffix");
    };
    assert_eq!(
        diffs.iter().map(|d| d.scene.revision.0).collect::<Vec<_>>(),
        [3, 4]
    );
    assert!(matches!(
        retention.resume(&request(4, Some(engine(4)))),
        Some(ResumeReply::Current(_))
    ));
    // The same counter under another engine revision is a divergent base.
    for divergent in [request(2, Some(engine(9))), request(2, None), request(4, None)] {
        assert!(matches!(
            retention.resume(&divergent),
            Some(ResumeReply::Snapshot(_))
        ));
    }
    let mut other = request(2, Some(engine(2)));
    other.session = ProjectionSession("fixture:other".into());
    assert!(retention.resume(&other).is_none());
}

#[test]
fn the_count_bound_keeps_the_last_deltas() {
    // Counted by number alone, so only the count bound applies.
    let mut retention = DeltaRetention::new(
        RETAINED_DELTAS,
        WireSize {
            keyframe: |_| usize::MAX,
            delta: |_| 1,
        },
    );
    for revision in 1..=(RETAINED_DELTAS as u64 + 6) {
        retention.advance(built(revision, 2, revision as f32));
    }
    assert_eq!(retention.len(), RETAINED_DELTAS);
    let oldest = retention.retained().next().unwrap().scene.base.0;
    assert_eq!(oldest, 6);
    assert!(matches!(
        retention.resume(&request(5, Some(engine(5)))),
        Some(ResumeReply::Snapshot(_))
    ));
    assert!(matches!(
        retention.resume(&request(6, Some(engine(6)))),
        Some(ResumeReply::Diffs(diffs)) if diffs.len() == RETAINED_DELTAS
    ));
}

#[test]
fn the_byte_cap_evicts_before_the_count_bound() {
    // Every build moves every item, so each delta is most of a keyframe.
    let mut retention = DeltaRetention::default();
    retention.advance(relaid(1, 40, 0.0));
    retention.advance(relaid(2, 40, 1.0));
    let (delta_bytes, keyframe_bytes) = retention.bytes();
    assert_eq!(retention.len(), 1);
    assert!(delta_bytes * 2 > keyframe_bytes, "the instrument: deltas are large");
    for revision in 3..=8 {
        retention.advance(relaid(revision, 40, revision as f32));
        let (bytes, keyframe) = retention.bytes();
        assert!(bytes <= keyframe, "never more than a keyframe retained");
    }
    assert!(retention.len() < 7, "evicted early: {} retained", retention.len());
    // The same builds' small deltas are all retained.
    let mut small = DeltaRetention::default();
    for revision in 1..=8 {
        small.advance(built(revision, 40, revision as f32));
    }
    assert_eq!(small.len(), 7, "the control keeps every delta");
    assert!(matches!(
        retention.resume(&request(2, Some(engine(2)))),
        Some(ResumeReply::Snapshot(_))
    ));
    // A delta larger than the keyframe is not retained at all.
    let mut tiny = DeltaRetention::new(
        RETAINED_DELTAS,
        WireSize {
            keyframe: |_| 1,
            delta: |_| 2,
        },
    );
    tiny.advance(built(1, 2, 0.0));
    assert_eq!(tiny.advance(built(2, 2, 1.0)), Advance::Retained);
    assert!(tiny.is_empty());
}

#[test]
fn a_request_names_its_engine_revision_only_when_set() {
    let wire = crate::wire::encode(&request(2, None)).unwrap();
    assert!(!format!("{:?}", cbor_core::Value::decode(&wire).unwrap()).contains("engine"));
    let named = request(2, Some(engine(2)));
    let back: ResumeRequest =
        crate::wire::decode(&crate::wire::encode(&named).unwrap()).unwrap();
    assert_eq!(back, named);
}

#[test]
fn a_batch_of_presentation_changes_lands_as_one_by_one_would() {
    // The reference: each change applied alone, as clients did before.
    fn one_by_one(manifest: &mut PresentationManifest, change: &crate::PresentationChange) {
        use crate::PresentationChange::*;
        match change {
            Bind(binding) => {
                manifest.bindings.retain(|b| b.instance != binding.instance);
                manifest.bindings.push(binding.clone());
            },
            Unbind { instance } => manifest.bindings.retain(|b| b.instance != *instance),
            ReplaceOffers { key, offers } => {
                manifest.offers.insert(key.clone(), offers.clone());
            },
            RemoveOffers { key } => {
                manifest.offers.remove(key);
            },
            InvalidateResource { .. } => {},
        }
    }
    let bind = |i: u32, k: &str| {
        crate::PresentationChange::Bind(PresentationBinding {
            instance: InstanceId(i),
            key: PresentationKey(k.into()),
        })
    };
    let start = built(1, 5, 0.0).presentation;
    let changes = vec![
        bind(3, "a"),
        crate::PresentationChange::Unbind {
            instance: InstanceId(1),
        },
        bind(0, "b"),
        bind(3, "c"),
        bind(1, "d"),
        crate::PresentationChange::Unbind {
            instance: InstanceId(0),
        },
        bind(7, "e"),
        crate::PresentationChange::RemoveOffers {
            key: PresentationKey("k2".into()),
        },
    ];
    let mut reference = start.clone();
    for change in &changes {
        one_by_one(&mut reference, change);
    }
    let mut batch = start.clone();
    batch.apply_changes(&changes);
    assert_eq!(batch, reference);
    let mut moved = start.clone();
    moved.apply_changes(&start.changes_to(&built(2, 3, 1.0).presentation));
    assert_eq!(
        moved.bindings.iter().map(|b| b.instance.0).collect::<Vec<_>>(),
        [0, 1, 2]
    );
}

#[test]
fn with_nothing_retained_every_gap_gets_the_keyframe() {
    let keyframe = built(4, 3, 0.0);
    assert!(matches!(
        keyframe_reply(&request(4, Some(engine(4))), keyframe.clone()),
        ResumeReply::Current(ack) if ack.revision == Revision(4)
    ));
    for behind in [
        request(3, Some(engine(3))),
        request(4, Some(engine(9))),
        request(4, None),
        ResumeRequest::new(session(), SceneEpoch(2), Revision(4)).with_engine(Some(engine(4))),
    ] {
        assert!(matches!(
            keyframe_reply(&behind, keyframe.clone()),
            ResumeReply::Snapshot(sent) if *sent == keyframe
        ));
    }
}
