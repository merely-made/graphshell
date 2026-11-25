// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Each refusal beside its positive control: the same construction, left
//! intact, opens.

use sceno::{Footprint, ProjectedItem, Representation, Scene, SourceRef, Transform2};
use scenotime::{SceneDiff, SceneOp};

use super::*;
use crate::{CachePolicy, PresentationManifest, ProjectionSession, ProtocolVersion};

fn engine(cursor: u64) -> EngineRevision {
    EngineRevision {
        session: "engine-session".into(),
        cursor,
        digest: format!("v2:{cursor:064x}"),
    }
}

fn keyframe() -> ProjectionSnapshot {
    let mut scene = Scene::new();
    let source = scene.intern_source(SourceRef::new("mere.graph", "node:a"));
    scene.items.push(ProjectedItem {
        source,
        space: Scene::WORLD,
        transform: Transform2::translation(1.5, -2.0),
        footprint: Footprint::Circle { radius: 4.0 },
        representation: Representation::Glyph,
        layer: 0,
        visible: true,
        hit: None,
        channels: vec![("weight".into(), 0.25)],
    });
    ProjectionSnapshot {
        version: ProtocolVersion::V1,
        session: ProjectionSession("projection-session".into()),
        scene: crate::SceneSnapshot::from_dense(SceneEpoch(3), Revision(7), scene)
            .unwrap()
            .with_engine_revision(engine(41)),
        presentation: PresentationManifest::default(),
        cache_policy: CachePolicy::default(),
    }
}

fn diff() -> ProjectionDiff {
    ProjectionDiff {
        version: ProtocolVersion::V1,
        session: ProjectionSession("projection-session".into()),
        scene: SceneDiff::new(
            SceneEpoch(3),
            Revision(7),
            Revision(8),
            vec![SceneOp::SetItemLayer {
                index: sceno::InstanceId(0),
                layer: 2,
            }],
        )
        .with_engine(engine(41), engine(42)),
        presentation: Vec::new(),
        status: None,
    }
}

/// The sealed keyframe, decoded, edited, re-digested and re-encoded: an
/// envelope whose digest is honest about whatever the edit did.
fn forge(edit: impl FnOnce(&mut Value<'static>)) -> Vec<u8> {
    let bytes = seal(&keyframe()).unwrap();
    let mut envelope = Value::decode_owned(&bytes).unwrap();
    envelope.remove(DIGEST);
    edit(&mut envelope);
    let digest = digest_of(&envelope);
    envelope.insert(DIGEST, Value::from(digest.to_vec()));
    envelope.encode()
}

#[test]
fn a_keyframe_and_a_diff_round_trip() {
    let snapshot = keyframe();
    assert_eq!(
        open::<ProjectionSnapshot>(&seal(&snapshot).unwrap()).unwrap(),
        snapshot
    );
    let delta = diff();
    let sealed = seal(&delta).unwrap();
    let envelope = Envelope::read(&sealed).unwrap();
    assert_eq!(envelope.kind, EnvelopeKind::Diff);
    assert_eq!(envelope.identity.engine, engine(42));
    assert_eq!(envelope.identity.revision, Revision(8));
    assert_eq!(envelope.payload::<ProjectionDiff>().unwrap(), delta);
}

#[test]
fn sealing_is_deterministic() {
    assert_eq!(seal(&keyframe()).unwrap(), seal(&keyframe()).unwrap());
}

#[test]
fn non_canonical_bytes_are_refused() {
    let bytes = seal(&keyframe()).unwrap();
    assert!(open::<ProjectionSnapshot>(&bytes).is_ok(), "control");
    // `"epoch": 3` with the 3 in a one-byte argument instead of inline.
    let epoch = [0x65, b'e', b'p', b'o', b'c', b'h', 0x03];
    let at = bytes
        .windows(epoch.len())
        .position(|window| window == epoch)
        .unwrap();
    let mut longer = bytes[..at + 6].to_vec();
    longer.extend([0x18, 0x03]);
    longer.extend(&bytes[at + 7..]);
    assert!(matches!(
        open::<ProjectionSnapshot>(&longer),
        Err(EnvelopeError::NonCanonical(_))
    ));
}

#[test]
fn a_non_canonical_payload_is_refused_under_an_honest_digest() {
    let honest = forge(|_| {});
    assert!(open::<ProjectionSnapshot>(&honest).is_ok(), "control");
    let unsorted = forge(|envelope| {
        let payload = envelope.get(PAYLOAD).unwrap().as_bytes().unwrap().to_vec();
        // The payload map's first two entries, swapped, so its keys are out
        // of canonical order.
        let value = Value::decode(&payload).unwrap();
        let map = value.as_map().unwrap();
        let mut entries: Vec<_> = map.iter().collect();
        entries.swap(0, 1);
        let mut out = vec![0xa0 | entries.len() as u8];
        for (key, value) in entries {
            out.extend(key.encode());
            out.extend(value.encode());
        }
        envelope.insert(PAYLOAD, Value::from(out));
    });
    assert!(matches!(
        open::<ProjectionSnapshot>(&unsorted),
        Err(EnvelopeError::NonCanonical(_))
    ));
}

#[test]
fn an_unknown_version_is_refused() {
    assert!(
        open::<ProjectionSnapshot>(&forge(|e| {
            e.insert(VERSION, Value::from(ENVELOPE_VERSION));
        }))
        .is_ok()
    );
    let newer = forge(|e| {
        e.insert(VERSION, Value::from(ENVELOPE_VERSION + 1));
    });
    assert_eq!(
        open::<ProjectionSnapshot>(&newer),
        Err(EnvelopeError::UnknownVersion(ENVELOPE_VERSION + 1))
    );
}

#[test]
fn a_payload_without_an_engine_revision_is_not_sealed() {
    assert!(seal(&keyframe()).is_ok(), "control");
    let mut unbound = keyframe();
    unbound.scene.engine_revision = None;
    assert_eq!(
        seal(&unbound),
        Err(EnvelopeError::MissingIdentity("engine_revision"))
    );
    let mut half = diff();
    half.scene.base_engine = None;
    assert_eq!(
        seal(&half),
        Err(EnvelopeError::MissingIdentity("base_engine"))
    );
}

#[test]
fn a_header_without_its_identity_is_refused() {
    for name in [ENGINE, EPOCH, REVISION] {
        assert!(
            open::<ProjectionSnapshot>(&forge(|_| {})).is_ok(),
            "control"
        );
        let headless = forge(|e| {
            e.remove(name);
        });
        assert_eq!(
            open::<ProjectionSnapshot>(&headless),
            Err(EnvelopeError::MissingIdentity(name)),
            "{name}"
        );
    }
    let sessionless = forge(|e| {
        e.get_mut(ENGINE).unwrap().remove("session");
    });
    assert_eq!(
        open::<ProjectionSnapshot>(&sessionless),
        Err(EnvelopeError::MissingIdentity("session"))
    );
}

#[test]
fn a_payload_without_its_identity_is_refused_under_a_complete_header() {
    let mut unbound = keyframe();
    unbound.scene.engine_revision = None;
    let unbound = payload_bytes(&unbound).unwrap();
    assert!(
        open::<ProjectionSnapshot>(&forge(|_| {})).is_ok(),
        "control"
    );
    let forged = forge(|e| {
        e.insert(PAYLOAD, Value::from(unbound));
    });
    assert_eq!(
        open::<ProjectionSnapshot>(&forged),
        Err(EnvelopeError::MissingIdentity("engine_revision"))
    );
}

#[test]
fn a_digest_mismatch_is_refused() {
    let bytes = seal(&keyframe()).unwrap();
    assert!(open::<ProjectionSnapshot>(&bytes).is_ok(), "control");
    // One letter of the projection session inside the payload, changed: still
    // canonical CBOR and still a snapshot, but not what was digested.
    let session = b"projection-session";
    let at = bytes
        .windows(session.len())
        .position(|window| window == session)
        .unwrap();
    let mut tampered = bytes.clone();
    tampered[at] = b'P';
    assert_eq!(
        open::<ProjectionSnapshot>(&tampered),
        Err(EnvelopeError::DigestMismatch)
    );
    let mut wrong = Value::decode_owned(&bytes).unwrap();
    wrong.insert(DIGEST, Value::from(vec![0u8; 32]));
    assert_eq!(
        open::<ProjectionSnapshot>(&wrong.encode()),
        Err(EnvelopeError::DigestMismatch)
    );
}

#[test]
fn the_digest_is_domain_separated() {
    let bytes = seal(&keyframe()).unwrap();
    let mut envelope = Value::decode_owned(&bytes).unwrap();
    let digest = envelope.remove(DIGEST).unwrap();
    assert_eq!(digest.as_bytes().unwrap(), digest_of(&envelope));
    assert_ne!(
        digest.as_bytes().unwrap(),
        blake3::hash(&envelope.encode()).as_bytes(),
        "a plain hash of the same bytes is not the envelope digest"
    );
}

#[test]
fn a_header_that_is_not_the_payloads_own_is_refused() {
    assert!(
        open::<ProjectionSnapshot>(&forge(|_| {})).is_ok(),
        "control"
    );
    let elsewhere = forge(|e| {
        e.insert(EPOCH, Value::from(4u64));
    });
    assert!(
        Envelope::read(&elsewhere).is_ok(),
        "the header alone verifies"
    );
    assert_eq!(
        open::<ProjectionSnapshot>(&elsewhere),
        Err(EnvelopeError::IdentityMismatch)
    );
    let other_engine = forge(|e| {
        e.insert(ENGINE, Value::serialized(&engine(40)).unwrap());
    });
    assert_eq!(
        open::<ProjectionSnapshot>(&other_engine),
        Err(EnvelopeError::IdentityMismatch)
    );
}

#[test]
fn an_unknown_field_or_kind_is_refused() {
    assert!(
        open::<ProjectionSnapshot>(&forge(|_| {})).is_ok(),
        "control"
    );
    let extra = forge(|e| {
        e.insert("note", Value::from("hello"));
    });
    assert!(matches!(
        open::<ProjectionSnapshot>(&extra),
        Err(EnvelopeError::UnknownField(_))
    ));
    let extra_engine = forge(|e| {
        e.get_mut(ENGINE).unwrap().insert("host", Value::from("x"));
    });
    assert!(matches!(
        open::<ProjectionSnapshot>(&extra_engine),
        Err(EnvelopeError::UnknownField(_))
    ));
    let resume = forge(|e| {
        e.insert(KIND, Value::from("resume"));
    });
    assert_eq!(
        open::<ProjectionSnapshot>(&resume),
        Err(EnvelopeError::Kind("resume".into()))
    );
    assert_eq!(
        open::<ProjectionDiff>(&seal(&keyframe()).unwrap()),
        Err(EnvelopeError::Kind("keyframe".into())),
        "a keyframe is not opened as a diff"
    );
}

#[test]
fn trailing_bytes_are_refused() {
    let mut bytes = seal(&keyframe()).unwrap();
    assert!(open::<ProjectionSnapshot>(&bytes).is_ok(), "control");
    bytes.push(0x00);
    assert!(matches!(
        open::<ProjectionSnapshot>(&bytes),
        Err(EnvelopeError::Malformed(_))
    ));
}
