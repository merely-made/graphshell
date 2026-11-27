// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! A personal-sync projection, and a stored legacy translation receipt.

use super::*;

/// The personal graph a replica's store holds projects into incipit-graph
/// through [`IncipitFold`] exactly as it projected into the kernel's through
/// `KernelFold`, and a session journal the kernel began from that projection
/// reopens over incipit at the kernel's digest.
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

    fn hex(bytes: &[u8; 32]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    pollster::block_on(async {
        let kernel = expected("sync");
        // The replica's identity, roster and selection, as the kernel's had.
        let identity = InMemoryProvider::from_seed([0x5d; 32]);
        let roster = SyncRoster::new([identity.master_public_key().to_bytes()]);
        let selection = || {
            SyncSelection::default()
                .with_facets(["cross.note"])
                .with_access_records(true)
                .with_saved_scenes(true)
        };
        let replica = PersonalGraphReplica::for_identity(
            store("sync/replica.bin").await,
            GRAPH,
            &identity,
            roster,
            selection(),
        )
        .unwrap();

        let ours = replica.projection_with(&IncipitFold).await.unwrap();
        assert_eq!(
            unclocked(incipit_whole(&ours.graph)),
            unclocked(kernel["projection"].clone())
        );
        assert_eq!(ours.graph.nodes().count(), 2);
        assert_eq!(json!(ours.access_records.len()), kernel["access_records"]);
        let writers: Vec<Value> = ours
            .writers
            .iter()
            .map(|w| json!([hex(&w.operation), hex(&w.signer), hex(&w.stable_subject)]))
            .collect();
        assert_eq!(json!(writers), kernel["writers"]);
        let projected = incipit_session::personal_sync::materialize(
            &replica.sync_store(),
            GRAPH,
            replica.roster(),
            None,
            &selection(),
        )
        .await
        .unwrap();
        assert_eq!(
            unclocked(incipit_whole(&projected.graph)),
            unclocked(kernel["projection"].clone())
        );
        // Control: a fold that drops one arm projects another graph.
        let dropped = replica.projection_with(&DropsTags).await.unwrap();
        assert_ne!(
            unclocked(incipit_whole(&dropped.graph)),
            unclocked(kernel["projection"].clone())
        );

        // The session the kernel began from its projection reopens over
        // incipit at the same digest and graph.
        let reopened = Sessions::new(store("sync/session.bin").await)
            .open(session_id(&kernel["session"]))
            .await
            .unwrap();
        assert_eq!(incipit_whole(reopened.graph()), kernel["graph"]);
        assert_eq!(json!(live_digest(&reopened)), kernel["live_digest"]);
    });
}

/// A session holding a stored `LegacySurfaceV1` translation receipt, which
/// only the kernel's sessions wrote, opens over incipit through
/// [`INCIPIT_LEGACY`](incipit_session::INCIPIT_LEGACY) as the kernel read it;
/// without the hook, or with its receipt tampered, it is refused.
#[test]
fn a_stored_legacy_receipt_opens_through_incipits_hook() {
    pollster::block_on(async {
        let backend = store("legacy/session.bin").await;
        let kernel = expected("legacy");
        let session_id = session_id(&kernel["session"]);
        let receipt = format!("sessions/{}/legacy-translation.json", session_id.as_uuid());
        assert!(
            backend.get(&receipt).await.unwrap().is_some(),
            "a stored receipt"
        );

        let ours = Sessions::new(backend.clone())
            .open(session_id)
            .await
            .unwrap();
        assert_eq!(incipit_whole(ours.graph()), kernel["graph"]);
        assert_eq!(json!(live_digest(&ours)), kernel["live_digest"]);
        assert_eq!(
            ours.placement().map(|placement| placement.baseline),
            Some(PlacementProfile::LegacySurfaceV1)
        );
        assert_eq!(
            serde_json::to_value(ours.placement()).unwrap(),
            kernel["placement"]
        );
        assert_eq!(
            json!(
                ours.graph_at(muniment::Seq(0))
                    .map(|graph| incipit_whole(&graph))
            ),
            kernel["graph_at_0"]
        );

        // Control: sessions with no legacy hook refuse the receipt.
        let bare = pandect::MereSessions::<MemoryBackend, Graph>::new_with(backend.clone())
            .open_with(session_id)
            .await;
        assert!(
            matches!(bare, Err(SessionError::Corrupt(_))),
            "{:?}",
            bare.err()
        );
        // Control: a tampered receipt is refused through incipit's hook too.
        tamper(&backend, &receipt, "Legacy", "Legaci").await;
        let forged = Sessions::new(backend.clone()).open(session_id).await;
        assert!(forged.is_err(), "a tampered receipt is refused");
    });
}
