// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! A personal-sync projection, and a stored legacy translation receipt.

use super::*;

/// The personal graph a replica holds projects into incipit-graph through
/// [`IncipitFold`] exactly as it projects into the kernel's through
/// `KernelFold`, and a session journal begun from it reopens over incipit at
/// the kernel's digest.
#[cfg(all(feature = "personal-sync", not(target_arch = "wasm32")))]
#[test]
fn a_personal_sync_projection_and_its_session_journal_read_over_incipit() {
    use identity::{IdentityProvider, InMemoryProvider};
    use incipit_session::personal_sync::{IncipitFold, PersonalFold};
    use pandect::personal_sync::{
        PersonalGraphError, PersonalGraphEvent, PersonalGraphReplica, SyncRoster, SyncSelection,
    };
    const GRAPH: [u8; 32] = [0x5c; 32];

    /// The incipit fold with one arm dropped, for the comparison's control.
    struct DropsTags;
    impl PersonalFold<Graph> for DropsTags {
        fn apply(
            &self,
            graph: &mut Graph,
            asserter_iri: &str,
            fallback_id: &str,
            event: &PersonalGraphEvent,
        ) -> Result<(), PersonalGraphError> {
            match event {
                PersonalGraphEvent::AddTag { .. } => Ok(()),
                other => IncipitFold.apply(graph, asserter_iri, fallback_id, other),
            }
        }
        fn set_access_history(
            &self,
            graph: &mut Graph,
            node: Uuid,
            history: pandect::access::AccessHistory,
        ) {
            IncipitFold.set_access_history(graph, node, history);
        }
    }

    pollster::block_on(async {
        let identity = InMemoryProvider::from_seed([0x5d; 32]);
        let roster = SyncRoster::new([identity.master_public_key().to_bytes()]);
        let selection = SyncSelection::default()
            .with_facets(["cross.note"])
            .with_access_records(true)
            .with_saved_scenes(true);
        let mut replica = PersonalGraphReplica::for_identity(
            MemoryBackend::new(),
            GRAPH,
            &identity,
            roster,
            selection,
        )
        .unwrap();
        let add = |n, title: &str| PersonalGraphEvent::AddNode {
            id: id(n),
            address: format!("https://sync.test/{n}"),
            title: title.into(),
        };
        let relation = EdgeAssertion::Semantic {
            sub_kind: SemanticSubKind::Cites,
            label: None,
        };
        replica
            .author(vec![add(1, "One"), add(2, ""), add(3, "Three")])
            .await
            .unwrap();
        replica
            .author(vec![
                PersonalGraphEvent::SetTitle {
                    node: id(2),
                    title: "Two".into(),
                },
                PersonalGraphEvent::AddTag {
                    node: id(1),
                    tag: "kept".into(),
                },
                PersonalGraphEvent::AddTag {
                    node: id(2),
                    tag: "dropped".into(),
                },
                PersonalGraphEvent::AssertRelation {
                    from: id(1),
                    to: id(2),
                    assertion: relation.clone(),
                    assertion_version: None,
                    statement_id: None,
                    asserted_at_ms: None,
                    basis: None,
                },
                PersonalGraphEvent::SetFacet {
                    node: id(1),
                    facet: "cross.note".into(),
                    value: json!({"n": 1}),
                },
                PersonalGraphEvent::AppendAccess {
                    record: pandect::access::AccessRecord {
                        record_id: Uuid::from_u128(0xacc),
                        container_id: id(1),
                        address: "https://sync.test/1".into(),
                        action: pandect::access::AccessAction::Open,
                        persona: "personae://profile/cross".into(),
                        device: "cross".into(),
                        application: "graphshell".into(),
                        at_ms: 5,
                        handler: "graphshell.open".into(),
                        dwell_ms: None,
                        referring_container_id: None,
                        referring_address: None,
                        transition: pandect::access::AccessTransition::Unknown,
                        capture_source: "cross.read".into(),
                        source_event_id: None,
                        privacy: eidetic::PrivacyClass::TrustedPeersOnly,
                    },
                },
            ])
            .await
            .unwrap();
        replica
            .author(vec![
                PersonalGraphEvent::RemoveTag {
                    node: id(2),
                    tag: "dropped".into(),
                },
                PersonalGraphEvent::RemoveFacet {
                    node: id(1),
                    facet: "cross.note".into(),
                },
                PersonalGraphEvent::RemoveNode { id: id(3) },
            ])
            .await
            .unwrap();

        let kernel = replica.projection().await.unwrap();
        let ours = replica.projection_with(&IncipitFold).await.unwrap();
        assert_eq!(
            unclocked(incipit_whole(&ours.graph)),
            unclocked(kernel_whole(&kernel.graph))
        );
        assert_eq!(ours.graph.nodes().count(), 2);
        assert_eq!(ours.access_records.len(), kernel.access_records.len());
        assert_eq!(ours.writers, kernel.writers);
        let projected = incipit_session::personal_sync::materialize(
            &replica.sync_store(),
            GRAPH,
            replica.roster(),
            None,
            &SyncSelection::default()
                .with_facets(["cross.note"])
                .with_access_records(true)
                .with_saved_scenes(true),
        )
        .await
        .unwrap();
        assert_eq!(
            unclocked(incipit_whole(&projected.graph)),
            unclocked(kernel_whole(&kernel.graph))
        );
        // Control: a fold that drops one arm projects another graph.
        let dropped = replica.projection_with(&DropsTags).await.unwrap();
        assert_ne!(
            unclocked(incipit_whole(&dropped.graph)),
            unclocked(kernel_whole(&kernel.graph))
        );

        // The kernel's projection begins a session; its journal reopens over
        // incipit at the same digest and graph.
        let backend = MemoryBackend::new();
        let sessions = KernelSessions::new(backend.clone());
        let mut session = sessions.begin_recorded(author(), Some(kernel.graph.clone()));
        session.flush(AT).await.unwrap();
        session
            .apply(
                author(),
                vec![mere_kernel::graph::CapturedDelta::ReplaySetNodeTitleById {
                    node_id: id(1).to_string(),
                    title: "Once".into(),
                }],
            )
            .await
            .unwrap();
        let reopened = Sessions::new(backend.clone())
            .open(session.id())
            .await
            .unwrap();
        assert_eq!(
            incipit_whole(reopened.graph()),
            kernel_whole(session.graph())
        );
        assert_eq!(live_digest(&reopened), live_digest(&session));
    });
}

/// A session holding a stored `LegacySurfaceV1` translation receipt, which
/// only the kernel's sessions write, opens over incipit through
/// [`INCIPIT_LEGACY`](incipit_session::INCIPIT_LEGACY); without the hook,
/// or with its receipt tampered, it is refused.
#[test]
fn a_stored_legacy_receipt_opens_through_incipits_hook() {
    pollster::block_on(async {
        let backend = MemoryBackend::new();
        let sessions = KernelSessions::new(backend.clone());
        let mut graph = KernelGraph::new();
        for n in 1..=2 {
            mere_kernel::graph::apply::add_node(
                &mut graph,
                Some(id(n)),
                format!("https://legacy.test/{n}"),
                Default::default(),
            );
        }
        let mut session = sessions.begin(author(), Some(graph));
        session.flush(AT).await.unwrap();
        session
            .apply(
                author(),
                vec![mere_kernel::graph::CapturedDelta::ReplaySetNodeTitleById {
                    node_id: id(1).to_string(),
                    title: "Legacy".into(),
                }],
            )
            .await
            .unwrap();
        let mut session = sessions.open(session.id()).await.unwrap();
        let cutoff = session.journal().live_cursor();
        session.qualify_legacy(cutoff, vec![]).await.unwrap();
        session.flush(AT).await.unwrap();
        let receipt = format!(
            "sessions/{}/legacy-translation.json",
            session.id().as_uuid()
        );
        assert!(
            backend.get(&receipt).await.unwrap().is_some(),
            "a stored receipt"
        );
        let kernel = sessions.open(session.id()).await.unwrap();
        assert_eq!(
            kernel.placement().map(|placement| placement.baseline),
            Some(PlacementProfile::LegacySurfaceV1)
        );

        let ours = Sessions::new(backend.clone())
            .open(session.id())
            .await
            .unwrap();
        assert_eq!(incipit_whole(ours.graph()), kernel_whole(kernel.graph()));
        assert_eq!(live_digest(&ours), live_digest(&kernel));
        assert_eq!(ours.placement(), kernel.placement());
        assert_eq!(
            ours.graph_at(muniment::Seq(0))
                .map(|graph| incipit_whole(&graph)),
            kernel
                .graph_at(muniment::Seq(0))
                .map(|graph| kernel_whole(&graph))
        );

        // Control: sessions with no legacy hook refuse the receipt.
        let bare = pandect::MereSessions::<MemoryBackend, Graph>::new_with(backend.clone())
            .open_with(session.id())
            .await;
        assert!(
            matches!(bare, Err(SessionError::Corrupt(_))),
            "{:?}",
            bare.err()
        );
        // Control: a tampered receipt is refused through incipit's hook too.
        tamper(&backend, &receipt, "Legacy", "Legaci").await;
        let forged = Sessions::new(backend.clone()).open(session.id()).await;
        assert!(forged.is_err(), "a tampered receipt is refused");
    });
}
