// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! A store and session journal written by the kernel.

use super::*;

/// The addressable store and the session a kernel host wrote open over
/// incipit-graph at the same generation, digest and graph, and a re-save
/// through incipit writes the bytes the kernel's re-save writes.
#[test]
fn an_addressable_store_and_its_session_open_over_incipit() {
    pollster::block_on(async {
        let backend = MemoryBackend::new();
        let (session_id, container, generation) = kernel_written(&backend).await;
        let kernel = KernelSessions::new(backend.clone())
            .open(session_id)
            .await
            .unwrap();

        // The resident graph: same generation, same records read.
        let kernel_view =
            pandect::addressable_graph::AddressableGraphStore::new(backend.clone(), container)
                .open(pandect::addressable_graph::ResidencyRoots::default())
                .await
                .unwrap();
        let view = AddressableGraphStore::new(backend.clone(), container)
            .open(ResidencyRoots::default())
            .await
            .unwrap();
        assert_eq!(view.generation(), generation);
        assert_eq!(kernel_view.generation(), generation);
        assert_eq!(
            incipit_whole(view.graph()),
            kernel_whole(kernel_view.graph())
        );
        let roots = ResidencyRoots::default();
        let resident = Sessions::new(backend.clone())
            .open_resident_graph(session_id, roots)
            .await
            .unwrap();
        assert_eq!(resident.generation(), generation);

        // The session: same graph, journal and digest.
        let session = Sessions::new(backend.clone())
            .open(session_id)
            .await
            .unwrap();
        assert_eq!(incipit_whole(session.graph()), kernel_whole(kernel.graph()));
        assert_eq!(
            session.journal().entries().len(),
            kernel.journal().entries().len()
        );
        assert_eq!(live_digest(&session), live_digest(&kernel));
        // The access history the kernel's writer left reads back through
        // incipit's reader.
        let key = session.graph().get_node_key_by_id(id(1)).unwrap();
        let history = incipit_session::access::access_history(session.graph(), key).unwrap();
        let kernel_key = kernel.graph().get_node_key_by_id(id(1)).unwrap();
        assert_eq!(
            serde_json::to_value(&history).unwrap(),
            serde_json::to_value(
                pandect::access::access_history(kernel.graph(), kernel_key).unwrap()
            )
            .unwrap()
        );
        assert_eq!(history.records.len(), 1);
        // incipit's publication of the reopened graph names the same generation.
        assert_eq!(
            session.operation_revision().unwrap(),
            kernel.operation_revision().unwrap()
        );

        // A re-save, the same on each side from the same bytes: a checkpoint
        // and its publication, then one more published edit.
        let (kernel_store, incipit_store) = (copy_of(&backend).await, copy_of(&backend).await);
        let mut kernel = KernelSessions::new(kernel_store.clone())
            .open(session_id)
            .await
            .unwrap();
        let mut session = Sessions::new(incipit_store.clone())
            .open(session_id)
            .await
            .unwrap();
        assert_eq!(
            session.publish_resident_checkpoint().await.unwrap(),
            kernel.publish_resident_checkpoint().await.unwrap()
        );
        let title = |node| mere_kernel::graph::apply::GraphDelta::ReplaySetNodeTitleById {
            node_id: node,
            title: "Two".into(),
        };
        let kernel_edit = kernel
            .transact_deltas_published(author(), vec![title(id(2))], AT)
            .await
            .unwrap()
            .1;
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
        assert_eq!(incipit_edit, kernel_edit);
        assert_eq!(bytes(&incipit_store).await, bytes(&kernel_store).await);
        // Control: the comparison sees one edit's difference.
        assert_ne!(incipit_whole(session.graph()), incipit_whole(view.graph()));

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
        let backend = MemoryBackend::new();
        let (session_id, _, _) = kernel_written(&backend).await;
        let kernel = KernelSessions::new(backend.clone())
            .open(session_id)
            .await
            .unwrap();
        let digest = live_digest(&kernel);
        let session = Sessions::new(backend.clone())
            .open(session_id)
            .await
            .unwrap();
        assert_eq!(live_digest(&session), digest);
        // Every journal prefix pins to the same digest on both sides.
        for cursor in 0..=kernel.journal().live_cursor().0 {
            let cursor = muniment::Seq(cursor);
            assert_eq!(
                session.pin(cursor).unwrap().digest(),
                kernel.pin(cursor).unwrap().digest()
            );
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
