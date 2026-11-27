// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! pandect's legacy retraction test (Mere 675fd8111, `personal_sync.rs`) over
//! incipit-graph: it calls the fold's `retract_legacy`, which is this
//! module's (engine split D272). Its fixture is `tests/personal_sync.rs`'s.

use super::*;
use identity::{IdentityProvider, InMemoryProvider};
use mere::model::SemanticSubKind;
use mere::model::author::Author;
use muniment::MemoryBackend;
use uuid::Uuid;

fn hex32(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

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

fn assertion_statements(projection: &SyncProjection<Graph>) -> Vec<mere::model::SemanticStatement> {
    let from = projection.graph.get_node_key_by_id(A).unwrap();
    let to = projection.graph.get_node_key_by_id(B).unwrap();
    projection
        .graph
        .projected_relations_between(from, to)
        .flat_map(|(_, edge)| edge.held_assertions().cloned())
        .collect()
}

fn selection() -> SyncSelection {
    SyncSelection::default()
        .with_facets(["graphshell.test/v1"])
        .with_access_records(true)
        .with_saved_scenes(true)
        .with_handler_preferences(true)
        .with_blob_availability(true)
}

#[tokio::test]
async fn p1_sync_legacy_retraction_keeps_exact_claims_in_both_stores() {
    use mere::model::{
        PersistedEdge, PersistedEdgeFamily, PersistedSemanticStatement, PersistedSemanticSubKind,
    };

    let (replica, alice, bob) = assertion_fixture().await;
    let mut projection = replica.projection_with(&IncipitFold).await.unwrap();
    let statements = [
        (&alice, "urn:sync:surface:alice", 100),
        (&bob, "urn:sync:surface:bob", 200),
    ]
    .into_iter()
    .map(|(asserter, id, at)| PersistedSemanticStatement {
        statement_id: id.into(),
        predicate: mere::model::predicate_iri(SemanticSubKind::Supports).into(),
        recognized_sub_kind: Some(PersistedSemanticSubKind::Supports),
        label: Some("held Surface claim".into()),
        basis: AssertionBasis {
            note: Some("held Surface claim".into()),
            ..Default::default()
        },
        graph_scope: GraphScope::Default,
        provenance_iri: Some(asserter.clone()),
        asserted_at_ms: Some(at),
        first_asserted_ms: Some(at),
        withdrawn: None,
    })
    .collect();
    apply_graph_delta(
        &mut projection.graph,
        GraphDelta::ReplaySetEdgesByIds {
            from_id: A,
            to_id: B,
            edges: vec![PersistedEdge {
                from_node_id: A.to_string(),
                to_node_id: B.to_string(),
                families: vec![PersistedEdgeFamily::Semantic],
                semantic: None,
                traversal: None,
                containment: None,
                arrangement: None,
                imported: None,
                provenance: None,
                assertions: Some(statements),
            }],
        },
    );
    assert_eq!(
        projection.graph.edge_count(),
        1,
        "raw Surface positive control"
    );
    assert_eq!(projection.graph.resource_relations().count(), 1);
    let before = assertion_statements(&projection);
    assert_eq!(before.len(), 4);
    assert!(before.iter().all(|s| s.asserted_at_ms.is_some()));
    for (id, at) in [
        ("urn:sync:surface:alice", 100),
        ("urn:sync:surface:bob", 200),
    ] {
        assert_eq!(
            before
                .iter()
                .find(|s| s.statement_id == id)
                .unwrap()
                .asserted_at_ms,
            Some(at),
            "raw exact capture retains its recorded time",
        );
    }
    assert!(retract_legacy(
        &mut projection.graph,
        A,
        B,
        RelationSelector::Semantic(SemanticSubKind::Cites),
        &alice,
    ));
    assert_eq!(
        assertion_statements(&projection),
        before
            .iter()
            .filter(|s| !(s.provenance_iri.as_deref() == Some(alice.as_str())
                && s.recognized_sub_kind == Some(SemanticSubKind::Cites)))
            .cloned()
            .collect::<Vec<_>>(),
        "Resource withdrawal leaves exact raw Surface records and the peer claim",
    );
    assert!(retract_legacy(
        &mut projection.graph,
        A,
        B,
        RelationSelector::Family(EdgeFamily::Semantic),
        &alice,
    ));
    let remaining = assertion_statements(&projection);
    assert_eq!(
        remaining,
        before
            .iter()
            .filter(|s| s.provenance_iri.as_deref() == Some(bob.as_str()))
            .cloned()
            .collect::<Vec<_>>(),
        "both peer handles retain their full original metadata",
    );
    assert_eq!(projection.graph.edge_count(), 1);
    assert_eq!(projection.graph.resource_relations().count(), 1);
}
