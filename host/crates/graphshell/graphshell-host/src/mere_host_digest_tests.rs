// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Engine binding E4 (D1, D2): the Resource digest the host reports for a
//! pinned view is unchanged by an arrangement edit made in Graphshell and
//! moves with a Resource statement edit.

use super::*;
use crate::mere_host::{FIXTURE_DEVICE_TWO_ADDRESS, FIXTURE_PERSONA_ADDRESS, fixture_handlers};
use incipit_session::access::AccessContext;
use kernel::graph::apply::assert_relation;
use mere::formats::canon::SortedNQuadsError;
use mere::model::{ArrangementSubKind, EdgeAssertion, SemanticStatementSpec};
use mere::projection::RdfSource;
use muniment::MemoryBackend;
use pandect::{RESOURCE_DIGEST_ALGORITHM, RESOURCE_DIGEST_VERSION};
use std::collections::BTreeSet;

fn host() -> MereHost<MemoryBackend> {
    MereHost::empty(
        MemoryBackend::new(),
        SelectedPersonaRef {
            persona: FIXTURE_PERSONA_ADDRESS.to_string(),
            profile: "profile:graphshell-e4".to_string(),
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
        url: format!("https://digest.test/{n}"),
        position: [0.0, 0.0],
    }
}

fn key(host: &MereHost<MemoryBackend>, n: u128) -> NodeKey {
    host.graph()
        .get_node_key_by_id(uuid::Uuid::from_u128(n))
        .expect("the node is in the graph")
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

/// A frame takes a member: an arrangement edit made in Graphshell.
fn arrange(host: &mut MereHost<MemoryBackend>, frame: u128, member: u128) {
    let (frame, member) = (key(host, frame), key(host, member));
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
}

/// Show a pin at the live cursor and report its digest, checking that the
/// host hands back the one the pin keeps.
fn pin_and_digest(host: &mut MereHost<MemoryBackend>) -> (pandect::GraphRevision, ResourceDigest) {
    let cursor = host.graph_session().journal().live_cursor();
    let pin = host.graph_session().pin(cursor).unwrap();
    host.show_pinned(pin).unwrap();
    let first = host
        .pinned_resource_digest()
        .expect("a blank-free pin has a digest");
    let again = host.pinned_resource_digest().unwrap();
    assert!(std::ptr::eq(first, again), "the pin keeps its digest");
    assert_eq!(first.algorithm, RESOURCE_DIGEST_ALGORITHM);
    assert_eq!(first.version, RESOURCE_DIGEST_VERSION);
    assert_eq!(first.digest.len(), 64, "32 bytes as hex");
    assert!(
        first
            .digest
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    );
    let digest = first.clone();
    (host.pinned().unwrap().revision_identity(), digest)
}

/// Every quad of a pinned view's whole projection, surface included.
fn all_rows(host: &MereHost<MemoryBackend>) -> BTreeSet<String> {
    let graph = host.pinned().expect("a pin is shown").graph();
    graph
        .held_quads()
        .chain(graph.withdrawn_reifier_quads())
        .map(|quad| quad.to_string())
        .collect()
}

fn assert_same_digest(before: &ResourceDigest, after: &ResourceDigest, what: &str) {
    assert_eq!(before, after, "the Resource digest moved across {what}");
}

/// Three nodes citing in a chain, so the Resource stratum is not empty.
fn cited_host() -> MereHost<MemoryBackend> {
    let mut host = host();
    host.apply_edits(vec![node(1), node(2), node(3)]).unwrap();
    cites(&mut host, 1, 2, "https://digest.test/cites");
    cites(&mut host, 2, 3, "https://digest.test/cites");
    host
}

#[test]
fn an_arrangement_edit_keeps_the_reported_digest_and_a_resource_edit_moves_it() {
    let mut host = cited_host();
    let (start, before) = pin_and_digest(&mut host);
    let start_rows = all_rows(&host);

    arrange(&mut host, 1, 3);
    let (arranged_at, arranged) = pin_and_digest(&mut host);
    // Positive control: the edit is a real journal edit that a quad-level
    // instrument sees in the whole projection.
    assert!(
        arranged_at.cursor > start.cursor,
        "the arrangement moved the journal"
    );
    assert_ne!(arranged_at.digest, start.digest, "the journal digest moved");
    assert_ne!(
        all_rows(&host),
        start_rows,
        "the arrangement shows in the projection"
    );
    assert_same_digest(&before, &arranged, "an arrangement edit");

    cites(&mut host, 3, 1, "https://digest.test/answers");
    let (edited_at, edited) = pin_and_digest(&mut host);
    assert!(edited_at.cursor > arranged_at.cursor);
    assert_ne!(
        edited.digest, arranged.digest,
        "a Resource edit moves the digest"
    );
    assert_ne!(edited.digest, before.digest);

    // The older pin still reports what it did: the digest is the pin's.
    let earlier = host.graph_session().pin(start.cursor).unwrap();
    host.show_pinned(earlier).unwrap();
    assert_same_digest(
        &before,
        host.pinned_resource_digest().unwrap(),
        "re-pinning",
    );
}

#[test]
#[should_panic(expected = "the Resource digest moved across a Resource edit")]
fn equality_across_a_resource_edit_fails() {
    let mut host = cited_host();
    let (_, before) = pin_and_digest(&mut host);
    cites(&mut host, 3, 1, "https://digest.test/answers");
    let (_, after) = pin_and_digest(&mut host);
    assert_same_digest(&before, &after, "a Resource edit");
}

#[test]
fn the_digest_is_a_pins_and_the_live_view_has_none() {
    let mut host = cited_host();
    assert!(matches!(
        host.pinned_resource_digest(),
        Err(MereHostError::NotPinned)
    ));
    pin_and_digest(&mut host);
    assert!(host.follow_live().is_some());
    assert!(matches!(
        host.pinned_resource_digest(),
        Err(MereHostError::NotPinned)
    ));
}

/// No host path puts a blank node in the Resource stratum (the projection
/// mints named nodes only), so the refusal is checked where it becomes the
/// host's error: Mere's real refusal converts and keeps its quad.
#[test]
fn a_blank_node_refusal_becomes_the_hosts_error() {
    let blank = mere::parse_nquads(
        "_:b0 <https://digest.test/p> <https://digest.test/o> .\n",
        None,
    )
    .expect("the N-Quads parse");
    let refusal = mere::sorted_nquads(blank.quads(mere::dataset::View::Live))
        .expect_err("a blank node is refused");
    let SortedNQuadsError::BlankNode { quad } = &refusal;
    assert!(
        quad.starts_with("_:"),
        "the refusal names the blank quad: {quad}"
    );

    let lifted = || -> Result<(), MereHostError> {
        Err::<(), _>(refusal.clone())?;
        Ok(())
    };
    let error = lifted().unwrap_err();
    assert!(matches!(&error, MereHostError::ResourceDigest(kept) if *kept == refusal));
    assert!(error.to_string().contains(quad.as_str()), "{error}");
}
