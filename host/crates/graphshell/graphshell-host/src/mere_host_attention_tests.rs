// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Arrangements follow; attention stays (engine split D133, contract C7): the
//! view log carries arrangement only, and the device's attention lives in a
//! slot no other reader of the journal sees.

use super::*;
use attention::decode_view;
use incipit::view::{AttentionState, CameraSnapshot, VIEW_INTENT_SCHEMA_V1};
use kernel::graph::{CapturedDelta, Seq};
use muniment::{Backend, DirectoryBackend, MemoryBackend};
use pandect::access::AccessContext;
use std::path::Path;

const FOCUS: &str = "https://secret.test/focus";

fn persona() -> SelectedPersonaRef {
    SelectedPersonaRef {
        persona: FIXTURE_PERSONA_ADDRESS.to_string(),
        profile: "profile:graphshell-a3".to_string(),
    }
}

fn context() -> AccessContext {
    AccessContext {
        persona: FIXTURE_PERSONA_ADDRESS.to_string(),
        device: FIXTURE_DEVICE_TWO_ADDRESS.to_string(),
        at_ms: 3_000,
    }
}

fn node(n: u128) -> CapturedDelta {
    CapturedDelta::ReplayAddNodeWithIdIfMissing {
        id: uuid::Uuid::from_u128(n).to_string(),
        url: format!("https://attention.test/{n}"),
        position: [0.0, 0.0],
    }
}

fn arrangement(strategy: &str, hidden: &[&str]) -> ViewIntent {
    ViewIntent {
        strategy: Some(strategy.to_string()),
        hidden_assertions: hidden.iter().map(|id| id.to_string()).collect(),
        ..ViewIntent::default()
    }
}

fn attention() -> AttentionState {
    AttentionState {
        camera: Some(CameraSnapshot {
            coefficients: [2.0, 0.0, 0.0, 2.0, 9.0, -9.0],
        }),
        focus: Some(FOCUS.to_string()),
        mirror_tiles: true,
    }
}

async fn open<B: Backend + Clone>(backend: B, id: SessionId) -> MereHost<B> {
    MereHost::open_session(backend, id, persona(), fixture_handlers(), context())
        .await
        .expect("open the session")
}

/// A host on a new session with two nodes and one arrangement, stored.
async fn written(backend: &MemoryBackend) -> (MereHost<MemoryBackend>, SessionId) {
    let mut host = MereHost::empty(backend.clone(), persona(), fixture_handlers(), context());
    host.through("graphshell".into(), |h| h.apply_edits(vec![node(1), node(2)]))
        .unwrap();
    host.set_view("graphshell", "canvas", arrangement("spectral.default", &["a"]))
        .await
        .unwrap();
    host.persist(1_800_000_000).await.unwrap();
    let id = host.graph_session().id();
    (host, id)
}

async fn session_bytes(backend: &MemoryBackend) -> Vec<(String, Vec<u8>)> {
    let mut keys = backend.list("sessions/").await.unwrap();
    keys.sort();
    let mut out = Vec::new();
    for key in keys {
        out.push((key.clone(), backend.get(&key).await.unwrap().unwrap()));
    }
    out
}

fn contains(haystack: &[u8], needle: &str) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle.as_bytes())
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        if name.to_string_lossy().starts_with(".muniment") {
            continue;
        }
        let target = to.join(&name);
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

#[test]
fn a_store_written_before_c7_reopens_and_its_v1_entries_keep_their_arrangement() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pre_c7_views");
    let work = tempfile::tempdir().unwrap();
    copy_dir(&fixture, work.path());
    pollster::block_on(async {
        let backend = DirectoryBackend::open(work.path()).expect("the fixture store opens");
        let mut host = MereHost::open(backend.clone(), persona(), fixture_handlers(), context())
            .await
            .expect("reopen the pre-C7 store");
        assert!(host.was_reopened());
        let canvas = ViewKey::new("graphshell", "canvas").unwrap();

        // Positive control: the stored value really carries attention keys.
        let stored = host.graph_session().view(&canvas).unwrap();
        assert_eq!(stored.schema, VIEW_INTENT_SCHEMA_V1);
        assert!(stored.value.get("camera").is_some_and(|v| !v.is_null()));
        assert!(stored.value.get("focus").is_some_and(|v| !v.is_null()));

        // The arrangement reads unchanged, at the current cursor and at N.
        let latest = arrangement("spectral.default", &["assertion-a", "assertion-b"]);
        assert_eq!(host.view("graphshell", "canvas"), Some(latest));
        let first = arrangement("phyllotaxis.default", &["assertion-a"]);
        let at_n = host.graph_session().view_at(&canvas, Seq(4)).unwrap();
        assert_eq!(at_n.schema, VIEW_INTENT_SCHEMA_V1);
        assert_eq!(decode_view(at_n), Some(first.clone()));
        assert_eq!(host.view("graphshell", "reading"), Some(ViewIntent::default()));

        // Attention found in v1 entries is dropped on read (D133 F1).
        assert_eq!(host.attention("graphshell", "canvas"), None);
        assert_eq!(host.attention("graphshell", "reading"), None);

        // A new write is v2; the old entry stays v1.
        let next = arrangement("timeline.updated", &["assertion-c"]);
        host.set_view("graphshell", "canvas", next.clone())
            .await
            .unwrap();
        let id = host.graph_session().id();
        drop(host);
        let reopened = open(backend, id).await;
        let current = reopened.graph_session().view(&canvas).unwrap();
        assert_eq!(current.schema, VIEW_INTENT_SCHEMA);
        assert_eq!(reopened.view("graphshell", "canvas"), Some(next));
        let old = reopened.graph_session().view_at(&canvas, Seq(4)).unwrap();
        assert_eq!(old.schema, VIEW_INTENT_SCHEMA_V1);
        assert_eq!(decode_view(old), Some(first));
    });
}

#[test]
fn a_v2_view_entry_never_holds_camera_focus_or_mirror_tiles() {
    pollster::block_on(async {
        let backend = MemoryBackend::new();
        let (mut host, _) = written(&backend).await;
        host.set_attention("graphshell", "canvas", attention())
            .await
            .unwrap();
        host.set_view("graphshell", "canvas", arrangement("timeline.updated", &["b"]))
            .await
            .unwrap();
        host.persist(1_800_000_001).await.unwrap();

        let mut views = 0;
        for (key, bytes) in session_bytes(&backend).await {
            if !key.contains("/views/") {
                continue;
            }
            views += 1;
            for word in ["camera", "focus", "mirror_tiles", FOCUS] {
                assert!(!contains(&bytes, word), "{key} holds {word}");
            }
        }
        assert!(views >= 2, "the view and its log were both checked");
        // Positive control: the same words are in the attention slot's bytes.
        let slot = backend.list("graphs/").await.unwrap();
        assert_eq!(slot.len(), 1, "one attention slot: {slot:?}");
        let bytes = backend.get(&slot[0]).await.unwrap().unwrap();
        for word in ["camera", "focus", "mirror_tiles", FOCUS] {
            assert!(contains(&bytes, word));
        }
    });
}

#[test]
fn attention_on_one_device_appears_in_nothing_another_reader_of_the_log_sees() {
    pollster::block_on(async {
        let device_a = MemoryBackend::new();
        let (mut host, id) = written(&device_a).await;
        host.set_attention("graphshell", "canvas", attention())
            .await
            .unwrap();
        host.persist(1_800_000_001).await.unwrap();
        let cursor = host.graph_session().journal().live_cursor();
        let view = host.view("graphshell", "canvas");

        // What replicates is the journal and the view log: everything under
        // `sessions/`. A second device holds exactly that.
        let device_b = MemoryBackend::new();
        for (key, bytes) in session_bytes(&device_a).await {
            assert!(!contains(&bytes, FOCUS), "{key} carries A's focus");
            device_b.put(&key, &bytes).await.unwrap();
        }
        let reader = open(device_b.clone(), id).await;
        assert_eq!(reader.attention("graphshell", "canvas"), None);
        assert_eq!(reader.view("graphshell", "canvas"), view);
        assert_eq!(
            reader.graph_session().graph_at(cursor).map(|g| g.node_count()),
            host.graph_session().graph_at(cursor).map(|g| g.node_count())
        );
        assert!(device_b.list("graphs/").await.unwrap().is_empty());

        // Device A reads its own attention back after a reopen.
        drop(host);
        let again = open(device_a, id).await;
        assert_eq!(again.attention("graphshell", "canvas"), Some(attention()));
    });
}

#[test]
fn attention_is_accepted_on_a_read_only_archive_and_the_log_stays_byte_identical() {
    pollster::block_on(async {
        let backend = MemoryBackend::new();
        let (host, id) = written(&backend).await;
        drop(host);
        // The shape an opened codicil has: a stored session marked read-only.
        let manifest_key = format!("sessions/{}/manifest.json", id.as_uuid());
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&backend.get(&manifest_key).await.unwrap().unwrap()).unwrap();
        manifest["codicil_read_only"] = serde_json::Value::Bool(true);
        backend
            .put(&manifest_key, &serde_json::to_vec_pretty(&manifest).unwrap())
            .await
            .unwrap();
        let before = session_bytes(&backend).await;

        let mut host = open(backend.clone(), id).await;
        assert!(host.graph_session().manifest().codicil_read_only);
        let refused = host
            .set_view("graphshell", "canvas", arrangement("timeline.updated", &[]))
            .await;
        assert!(
            matches!(
                refused,
                Err(MereHostError::Session(SessionError::ReadOnly(_)))
            ),
            "SET_VIEW stays refused: {refused:?}"
        );
        host.set_attention("graphshell", "canvas", attention())
            .await
            .expect("attention needs no write access to the session");
        assert_eq!(host.attention("graphshell", "canvas"), Some(attention()));
        assert_eq!(
            session_bytes(&backend).await,
            before,
            "journal, view log and manifest are untouched"
        );

        drop(host);
        let again = open(backend, id).await;
        assert_eq!(again.attention("graphshell", "canvas"), Some(attention()));
    });
}
