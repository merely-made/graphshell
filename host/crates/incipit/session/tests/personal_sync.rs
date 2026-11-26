// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! pandect's personal-sync tests (Mere 36a13570b, `personal_sync.rs`'s test
//! module) over incipit-graph: every projection folds through
//! [`IncipitFold`]. The protocol under test is pandect's.

#![cfg(all(feature = "personal-sync", not(target_arch = "wasm32")))]

use eidetic::PrivacyClass;
use identity::{IdentityProvider, InMemoryProvider};
use incipit_session::personal_sync::*;
use kernel::graph::{Author, EdgeAssertion, Graph};
use kernel::graph::{EdgeFamily, RelationKind, SemanticSubKind};
use muniment::{MemoryBackend, RedbBackend};
use murm_sync::JoinedSpace;
use p2panda_core::SigningKey;
use pandect::CartographyGeometry;
use pandect::SavedSceneV2;
use pandect::access::AccessRecord;
use pandect::access::{AccessAction, AccessTransition};
use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;
use stickleback::{DataKeyring, ProcessError};
use transport::{P2pandaTransport, PeerID, sync_overlay_topic};
use uuid::Uuid;

const GRAPH: [u8; 32] = [0x77; 32];
const A: Uuid = Uuid::from_u128(0xa1);
const B: Uuid = Uuid::from_u128(0xb2);

fn relation() -> EdgeAssertion {
    EdgeAssertion::Semantic {
        sub_kind: SemanticSubKind::Cites,
        label: None,
    }
}

async fn assertion_fixture() -> (PersonalGraphReplica<MemoryBackend>, String, String) {
    let alice = InMemoryProvider::from_seed([0xa1; 32]);
    let bob = InMemoryProvider::from_seed([0xb2; 32]);
    let roots = [
        alice.master_public_key().to_bytes(),
        bob.master_public_key().to_bytes(),
    ];
    let roster = SyncRoster::new(roots);
    let mut replica = PersonalGraphReplica::for_identity(
        MemoryBackend::new(),
        GRAPH,
        &alice,
        roster.clone(),
        selection(),
    )
    .unwrap();
    let mut peer =
        PersonalGraphReplica::for_identity(MemoryBackend::new(), GRAPH, &bob, roster, selection())
            .unwrap();
    let seed = replica
        .author(vec![
            PersonalGraphEvent::AddNode {
                id: A,
                address: "https://a.test/".into(),
                title: "A".into(),
            },
            PersonalGraphEvent::AddNode {
                id: B,
                address: "https://b.test/".into(),
                title: "B".into(),
            },
        ])
        .await
        .unwrap();
    peer.accept(&seed).await.unwrap();
    replica
        .author(vec![PersonalGraphEvent::AssertRelation {
            from: A,
            to: B,
            assertion: relation(),
            assertion_version: None,
            statement_id: None,
            asserted_at_ms: None,
            basis: None,
        }])
        .await
        .unwrap();
    let assertion = peer
        .author(vec![PersonalGraphEvent::AssertRelation {
            from: A,
            to: B,
            assertion: relation(),
            assertion_version: None,
            statement_id: None,
            asserted_at_ms: None,
            basis: None,
        }])
        .await
        .unwrap();
    replica.accept(&assertion).await.unwrap();
    (
        replica,
        Author::person(hex32(&roots[0])).asserter_iri(),
        Author::person(hex32(&roots[1])).asserter_iri(),
    )
}

fn assertion_statements(
    projection: &SyncProjection<Graph>,
) -> Vec<kernel::graph::SemanticStatement> {
    let from = projection.graph.get_node_key_by_id(A).unwrap();
    let to = projection.graph.get_node_key_by_id(B).unwrap();
    projection
        .graph
        .projected_relations_between(from, to)
        .flat_map(|(_, edge)| edge.held_assertions().cloned())
        .collect()
}

#[tokio::test]
async fn p1_sync_uses_stable_roots_for_distinct_assertions() {
    let (replica, alice, bob) = assertion_fixture().await;
    let projection = replica.projection_with(&IncipitFold).await.unwrap();
    let statements = assertion_statements(&projection);
    assert_eq!(statements.len(), 2);
    assert_ne!(statements[0].statement_id, statements[1].statement_id);
    for asserter in [alice, bob] {
        assert_eq!(
            statements
                .iter()
                .filter(|s| s.provenance_iri.as_deref() == Some(&asserter))
                .count(),
            1
        );
    }
    assert!(
        projection
            .writers
            .iter()
            .all(|receipt| receipt.signer != receipt.stable_subject)
    );
}

#[tokio::test]
async fn p1_sync_rebuilding_preserves_assertion_ids() {
    let (replica, _, _) = assertion_fixture().await;
    let first = assertion_statements(&replica.projection_with(&IncipitFold).await.unwrap());
    let second = assertion_statements(&replica.projection_with(&IncipitFold).await.unwrap());
    assert_eq!(first.len(), 2, "positive control: both sources were folded");
    assert_eq!(
        second.len(),
        2,
        "positive control: rebuilding retains both sources"
    );
    assert_eq!(
        first, second,
        "unchanged operations must preserve assertion handles"
    );
}

fn statement_event(kind: SemanticSubKind, id: &str, time: u64) -> PersonalGraphEvent {
    PersonalGraphEvent::AssertRelation {
        from: A,
        to: B,
        assertion: EdgeAssertion::Semantic {
            sub_kind: kind,
            label: None,
        },
        assertion_version: None,
        statement_id: Some(id.into()),
        asserted_at_ms: Some(time),
        basis: None,
    }
}

#[tokio::test]
async fn p1_sync_refuses_conflicting_ids_without_losing_valid_assertions() {
    let (mut replica, _, _) = assertion_fixture().await;
    replica
        .author(vec![statement_event(
            SemanticSubKind::Supports,
            "urn:co-op:one",
            100,
        )])
        .await
        .unwrap();
    let before = assertion_statements(&replica.projection_with(&IncipitFold).await.unwrap());
    let refusal = replica
        .author(vec![statement_event(
            SemanticSubKind::Contradicts,
            "urn:co-op:one",
            200,
        )])
        .await;
    assert!(matches!(
        refusal,
        Err(PersonalGraphError::Process(ProcessError::Rejected(_)))
    ));
    assert_eq!(
        assertion_statements(&replica.projection_with(&IncipitFold).await.unwrap()),
        before
    );
    replica
        .author(vec![statement_event(
            SemanticSubKind::Contradicts,
            "urn:co-op:two",
            200,
        )])
        .await
        .unwrap();
    assert_eq!(
        assertion_statements(&replica.projection_with(&IncipitFold).await.unwrap()).len(),
        4,
        "positive control: a distinct id admits the same second claim"
    );
}

fn selection() -> SyncSelection {
    SyncSelection::default()
        .with_facets(["graphshell.test/v1"])
        .with_access_records(true)
        .with_saved_scenes(true)
        .with_handler_preferences(true)
        .with_blob_availability(true)
}

fn access(device: &str, id: u128, at_ms: u64) -> AccessRecord {
    AccessRecord {
        record_id: Uuid::from_u128(id),
        container_id: A,
        address: "https://a.test/".into(),
        action: AccessAction::Examine,
        persona: "personae://profile/shared".into(),
        device: device.into(),
        application: "graphshell".into(),
        at_ms,
        handler: "graphshell.inspect".into(),
        dwell_ms: None,
        referring_container_id: None,
        referring_address: None,
        transition: AccessTransition::Unknown,
        capture_source: "graphshell.h7-test".into(),
        source_event_id: None,
        privacy: PrivacyClass::TrustedPeersOnly,
    }
}

fn fingerprint(projection: &SyncProjection<Graph>) -> (Vec<(Uuid, Vec<String>)>, usize, Vec<Uuid>) {
    let mut nodes = projection
        .graph
        .nodes()
        .map(|(_, node)| {
            let mut tags = node.tags.iter().cloned().collect::<Vec<_>>();
            tags.sort();
            (node.id, tags)
        })
        .collect::<Vec<_>>();
    nodes.sort_by_key(|(id, _)| *id);
    let mut accesses = projection
        .access_records
        .iter()
        .map(|record| record.record_id)
        .collect::<Vec<_>>();
    accesses.sort();
    (
        nodes,
        projection.graph.projected_relations().count(),
        accesses,
    )
}

#[tokio::test]
async fn partitioned_graph_events_converge_and_retain_each_device_chronology() {
    let alice_identity = Arc::new(InMemoryProvider::from_seed([0xa1; 32]));
    let bob_identity = Arc::new(InMemoryProvider::from_seed([0xb2; 32]));
    let roster = SyncRoster::new([
        alice_identity.master_public_key().to_bytes(),
        bob_identity.master_public_key().to_bytes(),
    ]);
    let mut alice = PersonalGraphReplica::for_identity(
        MemoryBackend::new(),
        GRAPH,
        alice_identity.as_ref(),
        roster.clone(),
        selection(),
    )
    .unwrap();
    let mut bob = PersonalGraphReplica::for_identity(
        MemoryBackend::new(),
        GRAPH,
        bob_identity.as_ref(),
        roster.clone(),
        selection(),
    )
    .unwrap();

    let seed = alice
        .author(vec![
            PersonalGraphEvent::AddNode {
                id: A,
                address: "https://a.test/".into(),
                title: "A".into(),
            },
            PersonalGraphEvent::AddNode {
                id: B,
                address: "https://b.test/".into(),
                title: "B".into(),
            },
        ])
        .await
        .unwrap();
    bob.accept(&seed).await.unwrap();

    alice
        .author(vec![
            PersonalGraphEvent::AddTag {
                node: A,
                tag: "alice".into(),
            },
            PersonalGraphEvent::AssertRelation {
                from: A,
                to: B,
                assertion: relation(),
                assertion_version: None,
                statement_id: None,
                asserted_at_ms: None,
                basis: None,
            },
            PersonalGraphEvent::AppendAccess {
                record: access("alice-laptop", 1, 100),
            },
        ])
        .await
        .unwrap();
    bob.author(vec![
        PersonalGraphEvent::AddTag {
            node: A,
            tag: "bob".into(),
        },
        PersonalGraphEvent::AppendAccess {
            record: access("bob-phone", 2, 90),
        },
        PersonalGraphEvent::SetFacet {
            node: A,
            facet: "graphshell.test/v1".into(),
            value: serde_json::json!({"device": "bob"}),
        },
        PersonalGraphEvent::SaveScene {
            node: B,
            scene: SavedSceneV2 {
                name: "Shared scene".into(),
                selected: vec![A, B],
                layout_strategy: Some("grid.default".into()),
                physics_paused: true,
                physics_damping: 0.7,
                physics_law: "spring.rapier".into(),
                physics_overlays: Vec::new(),
                physics_kind_source: "site".into(),
                physics_group_source: "site".into(),
                physics_mass_source: "degree".into(),
                physics_depth_source: "roots".into(),
                arrangement_pull: 0.4,
                arrangement_roles: None,
                dynamics: None,
                camera_offset: (12.0, 24.0),
                camera_zoom: 1.2,
                default_handler: "graphshell.inspect".into(),
                cartography: CartographyGeometry::default(),
            },
        },
        PersonalGraphEvent::SetHandlerPreference {
            key: "https".into(),
            handler: "turnstone".into(),
        },
        PersonalGraphEvent::ObserveBlobAvailability {
            observation: BlobAvailabilityObservation {
                record_id: Uuid::from_u128(3),
                container_id: A,
                blob: [0x42; 32],
                device: "bob-phone".into(),
                available: true,
                at_ms: 95,
            },
        },
    ])
    .await
    .unwrap();

    let alice_transport = P2pandaTransport::builder(alice_identity.master_keypair())
        .gossip()
        .bind()
        .await
        .unwrap();
    let bob_transport = P2pandaTransport::builder(bob_identity.master_keypair())
        .gossip()
        .bind()
        .await
        .unwrap();
    alice_transport
        .add_peer(bob_transport.endpoint_addr().await.unwrap())
        .await
        .unwrap();
    alice_transport
        .set_topics(
            PeerID::from_public_key(bob_identity.master_public_key()),
            &[sync_overlay_topic(GRAPH)],
        )
        .await
        .unwrap();
    bob_transport
        .add_peer(alice_transport.endpoint_addr().await.unwrap())
        .await
        .unwrap();
    bob_transport
        .set_topics(
            PeerID::from_public_key(alice_identity.master_public_key()),
            &[sync_overlay_topic(GRAPH)],
        )
        .await
        .unwrap();

    let alice_store = alice.sync_store();
    let alice_accept = alice_store.clone();
    let alice_roster = roster.clone();
    let (alice_endpoint, alice_gossip) = alice_transport.sync_parts().unwrap();
    let alice_space = JoinedSpace::join::<_, u64, _, _>(
        stickleback::lane_id("graphshell/personal-graph/v1", GRAPH),
        alice_store,
        alice_endpoint,
        alice_gossip,
        GRAPH,
        move |operation| {
            let store = alice_accept.clone();
            let roster = alice_roster.clone();
            async move {
                accept_into(&store, GRAPH, &roster, None, &operation)
                    .await
                    .unwrap_or(false)
            }
        },
    )
    .await
    .unwrap();

    let bob_store = bob.sync_store();
    let bob_accept = bob_store.clone();
    let bob_roster = roster.clone();
    let (bob_endpoint, bob_gossip) = bob_transport.sync_parts().unwrap();
    let bob_space = JoinedSpace::join::<_, u64, _, _>(
        stickleback::lane_id("graphshell/personal-graph/v1", GRAPH),
        bob_store,
        bob_endpoint,
        bob_gossip,
        GRAPH,
        move |operation| {
            let store = bob_accept.clone();
            let roster = bob_roster.clone();
            async move {
                accept_into(&store, GRAPH, &roster, None, &operation)
                    .await
                    .unwrap_or(false)
            }
        },
    )
    .await
    .unwrap();

    let (alice_projection, bob_projection) = tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            let a = alice.projection_with(&IncipitFold).await.unwrap();
            let b = bob.projection_with(&IncipitFold).await.unwrap();
            if a.access_records.len() == 2
                && b.access_records.len() == 2
                && a.graph.projected_relations().count() == 1
                && b.graph.projected_relations().count() == 1
            {
                break (a, b);
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("personal graph replicas did not converge");

    assert_eq!(fingerprint(&alice_projection), fingerprint(&bob_projection));
    let node = alice_projection.graph.get_node_by_id(A).unwrap().1;
    assert!(node.tags.contains("alice"));
    assert!(node.tags.contains("bob"));
    assert_eq!(
        alice_projection
            .graph
            .projected_relations()
            .filter(|(_, relation)| matches!(relation.kind, RelationKind::Semantic(_)))
            .count(),
        1
    );
    assert_eq!(
        alice_projection
            .access_records
            .iter()
            .map(|record| (record.device.as_str(), record.at_ms))
            .collect::<Vec<_>>(),
        [("bob-phone", 90), ("alice-laptop", 100)]
    );
    assert_eq!(
        alice_projection
            .graph
            .facets()
            .get(&A, &portolan::FacetId::new("graphshell.test/v1"),),
        Some(&serde_json::json!({"device": "bob"}))
    );
    assert_eq!(
        alice_projection
            .scenes
            .get(&B)
            .map(|scene| scene.name.as_str()),
        Some("Shared scene")
    );
    assert_eq!(
        alice_projection
            .handler_preferences
            .get("https")
            .map(String::as_str),
        Some("turnstone")
    );
    assert_eq!(alice_projection.blob_availability.len(), 1);
    assert_eq!(
        alice_projection.available_blobs.get(&[0x42; 32]),
        Some(&BTreeSet::from(["bob-phone".to_string()]))
    );
    assert!(
        alice_projection
            .writers
            .iter()
            .all(|receipt| roster.admits(&receipt.stable_subject))
    );
    assert!(alice_projection.pending.is_empty());
    assert!(alice_projection.conflicts.is_empty());
    assert!(alice_space.sync_status().sync_rounds > 0);
    assert!(bob_space.sync_status().sync_rounds > 0);
}

#[tokio::test]
async fn persona_handler_preference_is_opt_in_and_secret_free() {
    let alice_seed = [0x41; 32];
    let bob_seed = [0x42; 32];
    let alice_subject = *SigningKey::from_bytes(&alice_seed)
        .verifying_key()
        .as_bytes();
    let bob_subject = *SigningKey::from_bytes(&bob_seed).verifying_key().as_bytes();
    let roster = SyncRoster::new([alice_subject, bob_subject]);
    let mut alice = PersonalGraphReplica::new(
        MemoryBackend::new(),
        GRAPH,
        alice_seed,
        roster.clone(),
        SyncSelection::default().with_handler_preferences(true),
    );
    let bob = PersonalGraphReplica::new(
        MemoryBackend::new(),
        GRAPH,
        bob_seed,
        roster.clone(),
        SyncSelection::default().with_handler_preferences(true),
    );
    let bystander = PersonalGraphReplica::new(
        MemoryBackend::new(),
        GRAPH,
        [0x43; 32],
        roster,
        SyncSelection::default(),
    );

    let operation = alice
        .author(vec![PersonalGraphEvent::SetHandlerPreference {
            key: "https".into(),
            handler: "turnstone".into(),
        }])
        .await
        .unwrap();
    bob.accept(&operation).await.unwrap();
    bystander.accept(&operation).await.unwrap();

    assert_eq!(
        bob.projection_with(&IncipitFold)
            .await
            .unwrap()
            .handler_preferences
            .get("https")
            .map(String::as_str),
        Some("turnstone")
    );
    assert!(
        bystander
            .projection_with(&IncipitFold)
            .await
            .unwrap()
            .handler_preferences
            .is_empty(),
        "the ordinary preference remains opt-in at projection time"
    );

    let secret = alice
        .author(vec![PersonalGraphEvent::SetFacet {
            node: A,
            facet: "personae.vault-root/v1".into(),
            value: serde_json::json!("never"),
        }])
        .await
        .unwrap_err();
    assert!(matches!(secret, PersonalGraphError::Excluded(_)));
}

#[tokio::test]
async fn concurrent_scalar_edits_are_exposed_and_secret_facets_are_refused() {
    let alice_seed = [0x31; 32];
    let bob_seed = [0x32; 32];
    let alice_subject = *SigningKey::from_bytes(&alice_seed)
        .verifying_key()
        .as_bytes();
    let bob_subject = *SigningKey::from_bytes(&bob_seed).verifying_key().as_bytes();
    let roster = SyncRoster::new([alice_subject, bob_subject]);
    let mut alice = PersonalGraphReplica::new(
        MemoryBackend::new(),
        GRAPH,
        alice_seed,
        roster.clone(),
        selection(),
    );
    let mut bob =
        PersonalGraphReplica::new(MemoryBackend::new(), GRAPH, bob_seed, roster, selection());
    let seed = alice
        .author(vec![PersonalGraphEvent::AddNode {
            id: A,
            address: "https://a.test/".into(),
            title: "A".into(),
        }])
        .await
        .unwrap();
    bob.accept(&seed).await.unwrap();
    let left = alice
        .author(vec![PersonalGraphEvent::SetTitle {
            node: A,
            title: "Alice".into(),
        }])
        .await
        .unwrap();
    let right = bob
        .author(vec![PersonalGraphEvent::SetTitle {
            node: A,
            title: "Bob".into(),
        }])
        .await
        .unwrap();
    alice.accept(&right).await.unwrap();
    bob.accept(&left).await.unwrap();

    let projection = alice.projection_with(&IncipitFold).await.unwrap();
    assert_eq!(projection.conflicts.len(), 1);
    assert_eq!(projection.conflicts[0].target, format!("node/{A}/title"));

    let refused = alice
        .author(vec![PersonalGraphEvent::SetFacet {
            node: A,
            facet: "personae.vault-root/v1".into(),
            value: serde_json::json!("never"),
        }])
        .await
        .unwrap_err();
    assert!(matches!(refused, PersonalGraphError::Excluded(_)));

    let local_only = alice
        .author(vec![PersonalGraphEvent::AppendAccess {
            record: AccessRecord {
                privacy: PrivacyClass::LocalOnly,
                ..access("alice", 3, 110)
            },
        }])
        .await
        .unwrap_err();
    assert!(matches!(local_only, PersonalGraphError::Excluded(_)));

    assert_eq!(
        projection
            .graph
            .projected_relations()
            .filter(|(_, relation)| relation.kind.family() == EdgeFamily::Semantic)
            .count(),
        0
    );
}

#[tokio::test]
async fn redb_reopen_restores_projection_and_resumes_the_author_head() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("graphshell-personal-sync.redb");
    let seed = [0x51; 32];
    let subject = *SigningKey::from_bytes(&seed).verifying_key().as_bytes();
    let roster = SyncRoster::new([subject]);

    let first = {
        let backend = RedbBackend::open(&path).unwrap();
        let mut replica =
            PersonalGraphReplica::new(backend, GRAPH, seed, roster.clone(), selection());
        replica
            .author(vec![
                PersonalGraphEvent::AddNode {
                    id: A,
                    address: "https://a.test/".into(),
                    title: "A".into(),
                },
                PersonalGraphEvent::AddTag {
                    node: A,
                    tag: "before-restart".into(),
                },
                PersonalGraphEvent::AppendAccess {
                    record: access("durable-device", 4, 120),
                },
            ])
            .await
            .unwrap()
    };

    let backend = RedbBackend::open(&path).unwrap();
    let mut reopened = PersonalGraphReplica::new(backend, GRAPH, seed, roster.clone(), selection());
    let restored = reopened.projection_with(&IncipitFold).await.unwrap();
    assert!(
        restored
            .graph
            .get_node_by_id(A)
            .unwrap()
            .1
            .tags
            .contains("before-restart")
    );
    assert_eq!(restored.access_records.len(), 1);

    let second = reopened
        .author(vec![PersonalGraphEvent::AddTag {
            node: A,
            tag: "after-restart".into(),
        }])
        .await
        .unwrap();
    assert_eq!(second.header.seq_num, first.header.seq_num + 1);
    assert_eq!(
        second.header.backlink.as_ref().map(|hash| *hash.as_bytes()),
        Some(*first.hash.as_bytes())
    );
    let final_projection = reopened.projection_with(&IncipitFold).await.unwrap();
    let node = final_projection.graph.get_node_by_id(A).unwrap().1;
    assert!(node.tags.contains("before-restart"));
    assert!(node.tags.contains("after-restart"));
    assert!(final_projection.pending.is_empty());
}

fn keyring() -> Arc<DataKeyring> {
    let mut keyring = DataKeyring::new();
    keyring.rotate_random().expect("a fresh secret");
    Arc::new(keyring)
}

#[tokio::test]
async fn a_sealed_graph_reads_back_and_still_reads_what_it_wrote_in_plaintext() {
    let seed = [0x71; 32];
    let subject = *SigningKey::from_bytes(&seed).verifying_key().as_bytes();
    let roster = SyncRoster::new([subject]);
    let backend = MemoryBackend::new();
    let mut replica =
        PersonalGraphReplica::new(backend.clone(), GRAPH, seed, roster.clone(), selection());

    // Written before this device had a key.
    assert!(!replica.seals());
    let plain = replica
        .author(vec![PersonalGraphEvent::AddNode {
            id: A,
            address: "https://plain.test/".into(),
            title: "Written in the open".into(),
        }])
        .await
        .unwrap();
    assert_eq!(
        plain.header.extensions.encryption,
        PersonalEncryption::Plaintext
    );

    // The key arrives. Sealing starts from the next operation; nothing is
    // rewritten, which is the whole point of reading both.
    let keys = keyring();
    replica.set_keyring(Arc::clone(&keys));
    assert!(replica.seals());
    let sealed = replica
        .author(vec![PersonalGraphEvent::AddNode {
            id: B,
            address: "https://sealed.test/".into(),
            title: "Written under a key".into(),
        }])
        .await
        .unwrap();
    assert_eq!(
        sealed.header.extensions.encryption,
        PersonalEncryption::GroupV1
    );

    // The sealed body must not carry the plaintext it protects.
    let body = sealed.body.as_ref().unwrap().to_bytes();
    assert!(
        !String::from_utf8_lossy(&body).contains("sealed.test"),
        "a sealed operation must not leave its address in the body"
    );

    let projection = replica.projection_with(&IncipitFold).await.unwrap();
    assert!(
        projection.graph.get_node_by_id(A).is_some(),
        "the plaintext operation is still readable"
    );
    assert!(
        projection.graph.get_node_by_id(B).is_some(),
        "the sealed one is readable with the key"
    );
    assert!(projection.pending.is_empty());
}

/// A device without the key must say so, not quietly diverge. Two devices
/// silently holding different graphs is the failure mode worth refusing.
#[tokio::test]
async fn a_sealed_operation_is_refused_by_a_device_with_no_key() {
    let seed = [0x72; 32];
    let subject = *SigningKey::from_bytes(&seed).verifying_key().as_bytes();
    let roster = SyncRoster::new([subject]);
    let mut sealing = PersonalGraphReplica::new(
        MemoryBackend::new(),
        GRAPH,
        seed,
        roster.clone(),
        selection(),
    );
    sealing.set_keyring(keyring());
    let sealed = sealing
        .author(vec![PersonalGraphEvent::AddNode {
            id: A,
            address: "https://sealed.test/".into(),
            title: "Sealed".into(),
        }])
        .await
        .unwrap();

    let keyless = PersonalGraphReplica::new(MemoryBackend::new(), GRAPH, seed, roster, selection());
    let refused = keyless.accept(&sealed).await;
    assert!(
        refused.is_err() || !refused.unwrap(),
        "a device with no key must not report having accepted a sealed operation"
    );
    assert!(
        keyless
            .projection_with(&IncipitFold)
            .await
            .unwrap()
            .graph
            .get_node_by_id(A)
            .is_none(),
        "and it must not have materialized anything from it"
    );
}

/// The property that makes revocation fall out rather than be performed:
/// every device folds the same set, so none of them re-seats what another
/// retired.
#[tokio::test]
async fn the_reader_set_is_a_fold_every_device_computes_the_same_way() {
    let owner_seed = [0x81; 32];
    let sibling_seed = [0x82; 32];
    let owner_subject = *SigningKey::from_bytes(&owner_seed)
        .verifying_key()
        .as_bytes();
    let sibling_subject = *SigningKey::from_bytes(&sibling_seed)
        .verifying_key()
        .as_bytes();
    let roster = SyncRoster::new([owner_subject, sibling_subject]);
    let mut owner = PersonalGraphReplica::new(
        MemoryBackend::new(),
        GRAPH,
        owner_seed,
        roster.clone(),
        selection(),
    );
    let mut sibling = PersonalGraphReplica::new(
        MemoryBackend::new(),
        GRAPH,
        sibling_seed,
        roster,
        selection(),
    );

    // The first admit bootstraps: nobody could authorize it, so it stands
    // on its author's own authority and seats the author too.
    let opened = owner
        .author(vec![PersonalGraphEvent::AdmitReader {
            root: sibling_subject,
            label: "sibling".into(),
        }])
        .await
        .unwrap();
    sibling.accept(&opened).await.unwrap();

    let both = owner.projection_with(&IncipitFold).await.unwrap().readers;
    assert_eq!(
        both.len(),
        2,
        "the author seats itself alongside the one it named"
    );
    assert!(both.contains_key(&owner_subject));
    assert!(both.contains_key(&sibling_subject));
    assert_eq!(
        sibling.projection_with(&IncipitFold).await.unwrap().readers,
        both,
        "both devices fold the same set from the same operations"
    );

    // Retiring on ONE device is enough, because the other reads the same
    // fact rather than being told. This is the case the command-shaped
    // version could not do.
    let retired = sibling
        .author(vec![PersonalGraphEvent::RetireReader {
            root: owner_subject,
        }])
        .await
        .unwrap();
    owner.accept(&retired).await.unwrap();

    let after = sibling.projection_with(&IncipitFold).await.unwrap().readers;
    assert_eq!(
        after.keys().copied().collect::<Vec<_>>(),
        vec![sibling_subject]
    );
    assert_eq!(
        owner.projection_with(&IncipitFold).await.unwrap().readers,
        after,
        "the device that was retired agrees it was retired"
    );
}

/// Authority is checked in the fold. An admit from a device that is not a
/// reader is ignored rather than refused: it was validly authored and is
/// simply not authoritative about who reads.
#[tokio::test]
async fn only_a_reader_can_name_another() {
    let owner_seed = [0x83; 32];
    let stranger_seed = [0x84; 32];
    let owner_subject = *SigningKey::from_bytes(&owner_seed)
        .verifying_key()
        .as_bytes();
    let stranger_subject = *SigningKey::from_bytes(&stranger_seed)
        .verifying_key()
        .as_bytes();
    let roster = SyncRoster::new([owner_subject, stranger_subject]);
    let mut owner = PersonalGraphReplica::new(
        MemoryBackend::new(),
        GRAPH,
        owner_seed,
        roster.clone(),
        selection(),
    );
    let mut stranger = PersonalGraphReplica::new(
        MemoryBackend::new(),
        GRAPH,
        stranger_seed,
        roster,
        selection(),
    );

    let opened = owner
        .author(vec![PersonalGraphEvent::AdmitReader {
            root: owner_subject,
            label: "owner".into(),
        }])
        .await
        .unwrap();
    stranger.accept(&opened).await.unwrap();

    // On the roster, so this operation is admitted and stored. Not a
    // reader, so it does not get to decide who reads.
    let overreach = stranger
        .author(vec![PersonalGraphEvent::AdmitReader {
            root: stranger_subject,
            label: "helping myself".into(),
        }])
        .await
        .unwrap();
    owner.accept(&overreach).await.unwrap();

    let readers = owner.projection_with(&IncipitFold).await.unwrap().readers;
    assert_eq!(
        readers.keys().copied().collect::<Vec<_>>(),
        vec![owner_subject],
        "a writer that is not a reader cannot admit itself"
    );
}

/// The set must never empty out, or nobody could ever admit anyone again.
#[tokio::test]
async fn the_last_reader_cannot_retire_itself() {
    let seed = [0x85; 32];
    let subject = *SigningKey::from_bytes(&seed).verifying_key().as_bytes();
    let mut replica = PersonalGraphReplica::new(
        MemoryBackend::new(),
        GRAPH,
        seed,
        SyncRoster::new([subject]),
        selection(),
    );
    replica
        .author(vec![PersonalGraphEvent::AdmitReader {
            root: subject,
            label: "only".into(),
        }])
        .await
        .unwrap();
    replica
        .author(vec![PersonalGraphEvent::RetireReader { root: subject }])
        .await
        .unwrap();
    assert_eq!(
        replica
            .projection_with(&IncipitFold)
            .await
            .unwrap()
            .readers
            .len(),
        1,
        "retiring the last reader would strand the graph with no one able to admit"
    );
}

fn hex32(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
