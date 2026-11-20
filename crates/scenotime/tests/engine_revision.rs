// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The snapshot's optional engine revision (engine split D189): absent from
//! the wire when unset, ignored by a reader that predates it, and carried
//! whole when set.

use sceno::Scene;
use scenotime::{EngineRevision, Revision, SceneEpoch, SceneSnapshot};
use serde_json::Value;

fn snapshot() -> SceneSnapshot {
    SceneSnapshot::from_dense(SceneEpoch(4), Revision(9), Scene::new()).unwrap()
}

fn revision() -> EngineRevision {
    EngineRevision {
        session: "00000000-0000-0000-0000-000000000801".into(),
        cursor: 12,
        digest: "v2:abc".into(),
    }
}

#[test]
fn an_unstamped_snapshot_serializes_as_before() {
    let wire = serde_json::to_value(snapshot()).unwrap();
    let keys: Vec<&str> = wire
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(keys, ["epoch", "revision", "tables"]);
}

#[test]
fn a_snapshot_written_without_the_field_reads_as_unstamped() {
    let mut old = serde_json::to_value(snapshot()).unwrap();
    old.as_object_mut().unwrap().remove("engine_revision");
    let read: SceneSnapshot = serde_json::from_value(old).unwrap();
    assert_eq!(read.engine_revision, None);
    assert_eq!(read, snapshot());
}

#[test]
fn a_stamped_snapshot_carries_its_engine_revision_whole() {
    let stamped = snapshot().with_engine_revision(revision());
    let wire = serde_json::to_string(&stamped).unwrap();
    let back: SceneSnapshot = serde_json::from_str(&wire).unwrap();
    assert_eq!(back.engine_revision, Some(revision()));
    assert_ne!(back, snapshot(), "the stamp is part of the snapshot");

    // A reader shaped like the snapshot before the field skips it.
    #[derive(serde::Deserialize)]
    struct OldSnapshot {
        epoch: SceneEpoch,
        revision: Revision,
        #[allow(dead_code)]
        tables: Value,
    }
    let old: OldSnapshot = serde_json::from_str(&wire).unwrap();
    assert_eq!((old.epoch, old.revision), (SceneEpoch(4), Revision(9)));
}
