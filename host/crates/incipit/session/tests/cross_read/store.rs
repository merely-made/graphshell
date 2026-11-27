// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! A store and session journal written by the kernel: a varied graph
//! published through mere-kernel's addressable store (titled, tagged and
//! pinned nodes, a facet, an access record, a semantic relation, a visit and
//! a removal).

use super::*;

/// The addressable store and the session a kernel host wrote open over
/// incipit-graph at the generation, digest and graph the kernel read, and a
/// re-save through incipit writes the bytes the kernel's re-save wrote.
#[test]
fn an_addressable_store_and_its_session_open_over_incipit() {
    pollster::block_on(async {
        let backend = store("store/written.bin").await;
        let kernel = expected("store");
        let session_id = session_id(&kernel["session"]);
        let container = uuid_of(&kernel["container"]);

        // The resident graph: same generation, same records read.
        let view = AddressableGraphStore::new(backend.clone(), container)
            .open(ResidencyRoots::default())
            .await
            .unwrap();
        assert_eq!(json!(view.generation()), kernel["generation"]);
        assert_eq!(incipit_whole(view.graph()), kernel["view"]);
        let roots = ResidencyRoots::default();
        let resident = Sessions::new(backend.clone())
            .open_resident_graph(session_id, roots)
            .await
            .unwrap();
        assert_eq!(json!(resident.generation()), kernel["generation"]);

        // The session: same graph, journal and digest.
        let session = Sessions::new(backend.clone())
            .open(session_id)
            .await
            .unwrap();
        assert_eq!(incipit_whole(session.graph()), kernel["graph"]);
        assert_eq!(
            json!(session.journal().entries().len()),
            kernel["journal_entries"]
        );
        assert_eq!(json!(live_digest(&session)), kernel["live_digest"]);
        // The access history the kernel's writer left reads back through
        // incipit's reader.
        let key = session.graph().get_node_key_by_id(id(1)).unwrap();
        let history = incipit_session::access::access_history(session.graph(), key).unwrap();
        assert_eq!(
            serde_json::to_value(&history).unwrap(),
            kernel["access_history"]
        );
        assert_eq!(history.records.len(), 1);
        // incipit's publication of the reopened graph names the same generation.
        assert_eq!(
            json!(session.operation_revision().unwrap()),
            kernel["operation_revision"]
        );

        // A re-save from the same bytes: a checkpoint and its publication,
        // then one more published edit, as the kernel's re-save did.
        let incipit_store = copy_of(&backend).await;
        let mut session = Sessions::new(incipit_store.clone())
            .open(session_id)
            .await
            .unwrap();
        assert_eq!(
            json!(session.publish_resident_checkpoint().await.unwrap()),
            kernel["checkpoint"]
        );
        let incipit_edit = session
            .transact_deltas_published(
                author(),
                vec![kernel::graph::apply::GraphDelta::ReplaySetNodeTitleById {
                    node_id: id(2),
                    title: "Two".into(),
                }],
                AT,
            )
            .await
            .unwrap()
            .1;
        assert_eq!(json!(incipit_edit), kernel["edit_generation"]);
        // The checkpoint's snapshot is dated by the wall clock, so the re-save
        // matches the kernel's up to that second: every other key byte for
        // byte, the snapshot once dated alike, the checkpoint but for its digest
        // over the snapshot, and the kernel's re-save reopens to this graph.
        let ours = bytes(&incipit_store).await;
        let theirs = held("store/resaved.bin");
        assert_eq!(
            ours.keys().collect::<Vec<_>>(),
            theirs.keys().collect::<Vec<_>>()
        );
        let at = |name: &str| format!("sessions/{}/{name}", session_id.as_uuid());
        let (snapshot, checkpoint) = (at("graph.json"), at("checkpoint.json"));
        for (key, value) in &ours {
            if *key != snapshot && *key != checkpoint {
                assert_eq!(value, &theirs[key], "{key}");
            }
        }
        let json_at = |held: &BTreeMap<String, Vec<u8>>, key: &str| -> Value {
            serde_json::from_slice(&held[key]).unwrap()
        };
        let kernel_snapshot = json_at(&theirs, &snapshot);
        assert_eq!(
            dated_as_kernel(json_at(&ours, &snapshot), &kernel_snapshot, "/timestamp_secs"),
            kernel_snapshot
        );
        let undigested = |mut checkpoint: Value| {
            checkpoint.as_object_mut().unwrap().remove("contents_digest");
            checkpoint
        };
        assert_eq!(
            undigested(json_at(&ours, &checkpoint)),
            undigested(json_at(&theirs, &checkpoint))
        );
        let kernel_resaved = Sessions::new(store("store/resaved.bin").await)
            .open(session_id)
            .await
            .unwrap();
        assert_eq!(
            incipit_whole(session.graph()),
            incipit_whole(kernel_resaved.graph())
        );
        assert_eq!(live_digest(&session), live_digest(&kernel_resaved));
        // Control: the comparison sees one edit's difference.
        assert_ne!(incipit_whole(session.graph()), incipit_whole(view.graph()));
        assert_ne!(incipit_whole(session.graph()), kernel["graph"]);

        // Control: a tampered record refuses the reader.
        let record = bytes(&backend)
            .await
            .into_keys()
            .find(|key| {
                key.starts_with(&format!("graphs/{container}/resident/v1/nodes/surface/"))
                    && key.contains(&id(1).to_string())
            })
            .expect("node 1's record");
        tamper(&backend, &record, "One", "Onf").await;
        assert!(
            AddressableGraphStore::new(backend.clone(), container)
                .open(ResidencyRoots::default())
                .await
                .is_err(),
            "a tampered record is refused"
        );
    });
}

/// A session journal reopens over incipit at the kernel's digest, and a
/// tampered journal line names another digest or is refused.
#[test]
fn a_session_journal_reopens_at_the_kernels_digest() {
    pollster::block_on(async {
        let backend = store("store/written.bin").await;
        let kernel = expected("store");
        let session_id = session_id(&kernel["session"]);
        let digest = kernel["live_digest"].as_str().unwrap();
        let session = Sessions::new(backend.clone())
            .open(session_id)
            .await
            .unwrap();
        assert_eq!(live_digest(&session), digest);
        // Every journal prefix pins to the digest the kernel pinned it to.
        let prefixes = kernel["prefix_digests"].as_array().unwrap();
        assert_eq!(
            prefixes.len() as u64,
            session.journal().live_cursor().0 + 1
        );
        for (cursor, kernel_digest) in prefixes.iter().enumerate() {
            let cursor = muniment::Seq(cursor as u64);
            assert_eq!(json!(session.pin(cursor).unwrap().digest()), *kernel_digest);
        }

        // Each journal entry is its own key under the journal's name.
        let prefix = format!("sessions/{}/journal.jsonl", session_id.as_uuid());
        let entry = bytes(&backend)
            .await
            .into_iter()
            .find(|(key, value)| {
                key.starts_with(&prefix) && String::from_utf8_lossy(value).contains("\"One\"")
            })
            .map(|(key, _)| key)
            .expect("the entry that titles node 1");
        tamper(&backend, &entry, "\"One\"", "\"Onf\"").await;
        match Sessions::new(backend.clone()).open(session_id).await {
            Ok(forged) => {
                println!("tampered journal opens at another digest");
                assert_ne!(live_digest(&forged), digest, "a tampered line moves it")
            },
            Err(error) => {
                println!("tampered journal refused: {error}");
                assert!(matches!(error, SessionError::Corrupt(_)), "{error}")
            },
        }
    });
}
