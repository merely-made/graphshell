// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Engine split P8, probe 3's content check (D147, D179): an arrangement edit
//! made in Graphshell moves the journal but leaves every Resource-stratum
//! quad as it was, compared both ways, and a Resource edit does not.

use super::*;
use crate::mere_host::{FIXTURE_DEVICE_TWO_ADDRESS, FIXTURE_PERSONA_ADDRESS, fixture_handlers};
use kernel::graph::apply::assert_relation;
use kernel::graph::{ArrangementSubKind, EdgeAssertion, SemanticStatementSpec};
use mere::projection::RdfSource;
use muniment::MemoryBackend;
use pandect::access::AccessContext;
use std::collections::BTreeSet;

fn host() -> MereHost<MemoryBackend> {
    MereHost::empty(
        MemoryBackend::new(),
        SelectedPersonaRef {
            persona: FIXTURE_PERSONA_ADDRESS.to_string(),
            profile: "profile:graphshell-p8".to_string(),
        },
        fixture_handlers(),
        AccessContext {
            persona: FIXTURE_PERSONA_ADDRESS.to_string(),
            device: FIXTURE_DEVICE_TWO_ADDRESS.to_string(),
            at_ms: 3_000,
        },
    )
}

fn node(n: u128) -> CapturedDelta {
    CapturedDelta::ReplayAddNodeWithIdIfMissing {
        id: uuid::Uuid::from_u128(n).to_string(),
        url: format!("https://arrangement.test/{n}"),
        position: [0.0, 0.0],
    }
}

fn key(host: &MereHost<MemoryBackend>, n: u128) -> NodeKey {
    host.graph()
        .get_node_key_by_id(uuid::Uuid::from_u128(n))
        .expect("the node is in the graph")
}

/// Every Resource-stratum quad of `graph`, held and withdrawn, as N-Quads.
fn resource_rows(graph: &Graph) -> BTreeSet<String> {
    let resources = RdfSource::resource_graph(graph);
    resources
        .held_quads()
        .chain(resources.withdrawn_reifier_quads())
        .map(|quad| quad.to_string())
        .collect()
}

/// Every quad of the whole projection, surface included.
fn all_rows(graph: &Graph) -> BTreeSet<String> {
    graph
        .held_quads()
        .chain(graph.withdrawn_reifier_quads())
        .map(|quad| quad.to_string())
        .collect()
}

fn state(host: &MereHost<MemoryBackend>) -> (BTreeSet<String>, BTreeSet<String>) {
    (resource_rows(host.graph()), all_rows(host.graph()))
}

fn state_counts(rows: &BTreeSet<String>, other: &BTreeSet<String>) -> (usize, usize) {
    (
        rows.difference(other).count(),
        other.difference(rows).count(),
    )
}

fn cites(host: &mut MereHost<MemoryBackend>, from: u128, to: u128, predicate: &str) {
    let (from, to) = (key(host, from), key(host, to));
    host.mutate_product_graph(|graph| {
        graph
            .assert_semantic_statement(
                from,
                to,
                SemanticStatementSpec {
                    predicate: predicate.into(),
                    ..Default::default()
                },
            )
            .expect("the statement is asserted");
    })
    .unwrap();
}

#[test]
fn an_arrangement_edit_leaves_every_resource_quad_and_a_resource_edit_does_not() {
    let mut host = host();
    host.apply_edits(vec![node(1), node(2), node(3)]).unwrap();
    cites(&mut host, 1, 2, "https://arrangement.test/cites");
    cites(&mut host, 2, 3, "https://arrangement.test/cites");
    let (resources, all) = state(&host);
    assert!(
        !resources.is_empty(),
        "the graph has Resource-stratum quads"
    );
    let cursor = host.graph_session().journal().live_cursor();

    // An arrangement edit made in Graphshell: a frame takes a member.
    let (frame, member) = (key(&host, 1), key(&host, 3));
    let relation = host
        .mutate_product_graph(|graph| {
            assert_relation(
                graph,
                frame,
                member,
                EdgeAssertion::Arrangement {
                    sub_kind: ArrangementSubKind::FrameMember,
                },
            )
        })
        .unwrap();
    assert!(relation.is_some(), "the arrangement edit applied");
    assert!(
        host.graph_session().journal().live_cursor() > cursor,
        "it is a journal edit"
    );
    let (arranged, arranged_all) = state(&host);
    assert_ne!(
        arranged_all, all,
        "the arrangement shows in the whole projection"
    );
    assert_eq!(
        state_counts(&resources, &arranged),
        (0, 0),
        "no Resource quad was lost or gained"
    );

    // Control: one corrupted Resource quad is caught.
    let mut corrupted = arranged.clone();
    let first = corrupted.pop_first().unwrap();
    corrupted.insert(first.replacen("https://arrangement.test/", "https://corrupt.test/", 1));
    assert_eq!(state_counts(&resources, &corrupted), (1, 1));

    // A Resource edit changes the Resource quads.
    cites(&mut host, 3, 1, "https://arrangement.test/answers");
    let (edited, _) = state(&host);
    let (lost, gained) = state_counts(&resources, &edited);
    assert_eq!(lost, 0);
    assert!(gained > 0, "the Resource edit added quads");
}
