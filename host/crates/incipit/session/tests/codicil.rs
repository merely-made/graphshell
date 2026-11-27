// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! pandect's graph-codicil tests (Mere 675fd8111, `graph_codicil.rs`), over
//! incipit-graph, whose snapshot, redaction and compose they test (engine
//! split D272). The second half is in `codicil_compose.rs`.

use eidetic::{BlobSource, PrivacyClass, Timestamp, TrustEnvelope, save_typed_sealed};
use euclid::default::Point2D;
use incipit_session::graph_codicil::*;
use kernel::graph::Graph;
use kernel::graph::fixtures::GraphFixtures;
use kernel::persistence::PersistedNodeSessionState;
use kernel::types::{ImageRef, ImageRole};
use mere::model::PersistedResourceRecord;
use muniment::Backend;
use pandect::graph_placement::PlacementProfile;

#[path = "common/codicil.rs"]
mod common;
use common::*;

/// In-memory `Store` for the round-trip tests — the same shape
/// `content_store`'s tests and eidetic's own tests use.
// The in-memory test store is muniment's (2026-07-12): the
// hand-rolled one was the same map behind the same seam.
use muniment::MemoryBackend as InMemoryStore;

fn set_pinned(graph: &mut Graph, url: &str, pinned: bool) {
    let (_, node) = graph.get_node_by_url(url).unwrap();
    let node_id = node.id;
    graph
        .facets_mut()
        .set(
            node_id,
            portolan::FacetId::new(kernel::graph::node_facets::ARRANGEMENT_PIN),
            serde_json::json!(pinned),
            &portolan::AcceptAll,
        )
        .unwrap();
}

fn sample_graph() -> Graph {
    let mut graph = Graph::new();
    graph.add_node("https://a.example".to_string(), Point2D::new(1.0, 2.0));
    graph.add_node("https://b.example".to_string(), Point2D::new(3.0, 4.0));
    set_pinned(&mut graph, "https://a.example", true);
    graph
}

#[test]
fn an_opened_codicil_names_its_source_and_the_codicil() {
    let id = "blake3:f612a0c4d9e8b7a6";
    assert_eq!(codicil_short_id(id), "blake3:f612a");
    assert_eq!(codicil_short_id("short"), "short");
    assert_eq!(
        opened_codicil_session_name(None, id),
        "Session (from blake3:f612a)",
        "an unnamed source still reads apart from the session it came from"
    );
    assert_eq!(
        opened_codicil_session_name(Some("Evening reading"), id),
        "Evening reading (from blake3:f612a)"
    );
}

#[test]
fn codicil_recorded_profile_survives_thaw_and_composition_with_unqualified_controls() {
    pollster::block_on(async {
        let mut store = InMemoryStore::new();
        let graph = placement_test_graph();
        let input = ProfiledGraphCodicil {
            placement: Some(PlacementProfile::RecordedStrataV1),
            codicil: GraphCodicil {
                snapshot: graph.to_snapshot(),
                facets: graph.facets().clone(),
                sessions: Vec::new(),
            },
        };
        let id = save_profiled_graph_codicil_sealed(
            &mut store,
            None,
            input.clone(),
            RedactionPolicy::include_all(),
            Timestamp(1),
        )
        .await
        .unwrap();
        let loaded = open_codicil_as_session(&mut store, id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded.to_snapshot().edges, input.codicil.snapshot.edges);
        assert_eq!(
            loaded.to_snapshot().resource_edges,
            input.codicil.snapshot.resource_edges
        );
        let composed = compose_graph_codicils(
            &mut store,
            &[id, id],
            RedactionPolicy::include_all(),
            Timestamp(2),
        )
        .await
        .unwrap()
        .unwrap();
        let result = load_profiled_graph_codicil_sealed(&mut store, None, composed)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(result.placement, Some(PlacementProfile::RecordedStrataV1));
        assert_eq!(result.codicil.snapshot.edges, input.codicil.snapshot.edges);
        assert_eq!(
            result.clone().into_graph().unwrap().to_snapshot().edges,
            input.codicil.snapshot.edges
        );
        let bare = save_graph_snapshot_codicil(
            &mut store,
            input.codicil.snapshot.clone(),
            input.codicil.facets.clone(),
            RedactionPolicy::include_all(),
            Timestamp(3),
        )
        .await
        .unwrap();
        let mixed = compose_graph_codicils(
            &mut store,
            &[id, bare],
            RedactionPolicy::include_all(),
            Timestamp(4),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(
            load_profiled_graph_codicil_sealed(&mut store, None, mixed)
                .await
                .unwrap()
                .unwrap()
                .placement,
            None
        );
        let mut legacy = input.clone();
        legacy.placement = Some(PlacementProfile::LegacySurfaceV1);
        let before = store.list("").await.unwrap();
        assert!(
            save_profiled_graph_codicil_sealed(
                &mut store,
                None,
                legacy.clone(),
                RedactionPolicy::include_all(),
                Timestamp(5),
            )
            .await
            .is_err(),
            "a save refuses the placement an open refuses"
        );
        assert_eq!(store.list("").await.unwrap(), before);
        let legacy_id = left_unchecked(&mut store, &legacy).await;
        let before = store.list("").await.unwrap();
        assert!(
            open_codicil_as_session(&mut store, legacy_id)
                .await
                .is_err()
        );
        assert!(
            compose_graph_codicils(
                &mut store,
                &[id, legacy_id],
                RedactionPolicy::include_all(),
                Timestamp(6)
            )
            .await
            .is_err()
        );
        assert_eq!(store.list("").await.unwrap(), before);
        assert_eq!(
            load_profiled_graph_codicil_sealed(&mut store, None, legacy_id)
                .await
                .unwrap()
                .unwrap()
                .placement,
            Some(PlacementProfile::LegacySurfaceV1)
        );
    });
}

#[test]
fn codicil_profiles_are_explicit_and_legacy_inputs_require_qualified_replay() {
    pollster::block_on(async {
        let mut store = InMemoryStore::default();
        let graph = sample_graph();
        let mut snapshot = graph.to_snapshot();
        snapshot.resources.push(PersistedResourceRecord {
            canonical_iri: "urn:mere:placement-control".into(),
            facets: Vec::new(),
        });
        let mut ids = Vec::new();
        for placement in [
            None,
            Some(PlacementProfile::RecordedStrataV1),
            Some(PlacementProfile::LegacySurfaceV1),
        ] {
            let input = ProfiledGraphCodicil {
                codicil: GraphCodicil {
                    snapshot: snapshot.clone(),
                    facets: graph.facets().clone(),
                    sessions: Vec::new(),
                },
                placement,
            };
            let saved = save_profiled_graph_codicil_sealed(
                &mut store,
                None,
                input.clone(),
                RedactionPolicy::include_all(),
                Timestamp(1),
            )
            .await;
            let id = if placement == Some(PlacementProfile::LegacySurfaceV1) {
                assert!(
                    saved.is_err(),
                    "a save refuses the placement an open refuses"
                );
                left_unchecked(&mut store, &input).await
            } else {
                saved.unwrap()
            };
            ids.push(id);
            let loaded = load_profiled_graph_codicil_sealed(&mut store, None, id)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(loaded.placement, placement);
            assert_eq!(loaded.codicil.snapshot.resources, snapshot.resources);
            if placement == Some(PlacementProfile::LegacySurfaceV1) {
                assert!(open_codicil_as_session(&mut store, id).await.is_err());
                assert!(load_graph_codicil(&mut store, id).await.is_err());
            } else {
                assert_eq!(
                    open_codicil_as_session(&mut store, id)
                        .await
                        .unwrap()
                        .unwrap()
                        .to_snapshot()
                        .resources,
                    snapshot.resources
                );
            }
        }
        for version in [1, 2] {
            let id = if version == 1 {
                save_typed_sealed(
                    &mut store,
                    None,
                    &LegacyGraphCodicil(snapshot.clone()),
                    Vec::<BlobSource>::new(),
                    PrivacyClass::LocalOnly,
                    graph_codicil_provenance(Timestamp(1)),
                    TrustEnvelope::self_asserted(),
                    Timestamp(1),
                )
                .await
                .unwrap()
            } else {
                save_typed_sealed(
                    &mut store,
                    None,
                    &V2GraphCodicil(GraphCodicil {
                        snapshot: snapshot.clone(),
                        facets: graph.facets().clone(),
                        sessions: Vec::new(),
                    }),
                    Vec::<BlobSource>::new(),
                    PrivacyClass::LocalOnly,
                    graph_codicil_provenance(Timestamp(1)),
                    TrustEnvelope::self_asserted(),
                    Timestamp(1),
                )
                .await
                .unwrap()
            };
            let input = load_profiled_graph_codicil_sealed(&mut store, None, id)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(
                input.placement, None,
                "v1/v2 resource columns do not prove legacy or recorded placement"
            );
            assert_eq!(input.codicil.snapshot.resources, snapshot.resources);
            ids.push(id);
        }
        let listed = list_graph_codicils(&mut store).await.unwrap();
        for id in ids {
            assert!(listed.iter().any(|manifest| manifest.id == id));
        }
    });
}

#[test]
fn save_then_open_round_trips_the_graph() {
    pollster::block_on(async {
        let mut store = InMemoryStore::default();
        let id = save_graph_codicil(
            &mut store,
            &sample_graph(),
            RedactionPolicy::default(),
            Timestamp(1_700_000_000_000),
        )
        .await
        .expect("save");

        let opened = open_codicil_as_session(&mut store, id)
            .await
            .expect("load ok")
            .expect("codicil present after save");
        assert_eq!(opened.nodes().count(), 2, "both nodes survive freeze/thaw");
        assert!(
            opened.get_node_by_url("https://a.example").is_some(),
            "the URL index rebuilds from the thawed snapshot",
        );
        let (a, _) = opened.get_node_by_url("https://a.example").unwrap();
        assert_eq!(
            opened.node_is_pinned(a),
            Some(true),
            "the graph-owned facet store survives freeze/thaw"
        );
    });
}

#[test]
fn legacy_v1_codicil_loads_and_imports_its_inline_metadata() {
    pollster::block_on(async {
        let mut store = InMemoryStore::default();
        let mut snapshot = sample_graph().to_snapshot();
        snapshot.nodes[0].is_pinned = true;
        let id = save_typed_sealed(
            &mut store,
            None,
            &LegacyGraphCodicil(snapshot),
            Vec::<BlobSource>::new(),
            PrivacyClass::LocalOnly,
            graph_codicil_provenance(Timestamp(1)),
            TrustEnvelope::self_asserted(),
            Timestamp(1),
        )
        .await
        .unwrap();

        let opened = open_codicil_as_session(&mut store, id)
            .await
            .unwrap()
            .unwrap();
        let (a, _) = opened.get_node_by_url("https://a.example").unwrap();
        assert_eq!(opened.node_is_pinned(a), Some(true));
    });
}

#[test]
fn default_redaction_strips_private_web_facets_but_keeps_other_facets() {
    pollster::block_on(async {
        let mut store = InMemoryStore::default();
        let mut graph = sample_graph();
        let (_, node) = graph.get_node_by_url("https://a.example").unwrap();
        let node_id = node.id;
        graph
            .facets_mut()
            .set(
                node_id,
                portolan::FacetId::new("web.form_draft"),
                serde_json::json!("secret"),
                &portolan::AcceptAll,
            )
            .unwrap();
        graph
            .facets_mut()
            .set(
                node_id,
                portolan::FacetId::new("web.page_scale"),
                serde_json::json!(1.5),
                &portolan::AcceptAll,
            )
            .unwrap();
        graph
            .facets_mut()
            .set(
                node_id,
                portolan::FacetId::new("foreign.keep"),
                serde_json::json!({"portable": true}),
                &portolan::AcceptAll,
            )
            .unwrap();

        let id = save_graph_codicil(&mut store, &graph, RedactionPolicy::default(), Timestamp(1))
            .await
            .unwrap();
        let opened = open_codicil_as_session(&mut store, id)
            .await
            .unwrap()
            .unwrap();
        assert!(
            opened
                .facets()
                .get(&node_id, &portolan::FacetId::new("web.form_draft"))
                .is_none()
        );
        assert!(
            opened
                .facets()
                .get(&node_id, &portolan::FacetId::new("web.page_scale"))
                .is_none()
        );
        assert_eq!(
            opened
                .facets()
                .get(&node_id, &portolan::FacetId::new("foreign.keep")),
            Some(&serde_json::json!({"portable": true}))
        );
    });
}

#[test]
fn default_redaction_strips_private_fields() {
    // A snapshot carrying a thumbnail, favicon, and session state (scroll +
    // form draft); the default policy must drop all three but keep structure.
    let mut snapshot = sample_graph().to_snapshot();
    for node in &mut snapshot.nodes {
        node.images
            .insert(ImageRole::Preview, ImageRef::new([1u8; 32], 2, 2));
        node.images
            .insert(ImageRole::Favicon, ImageRef::new([4u8; 32], 1, 1));
        node.session_state = Some(PersistedNodeSessionState {
            scroll_x: Some(10.0),
            scroll_y: Some(20.0),
            form_draft: Some("a secret draft".to_string()),
            last_visited_ms: None,
        });
    }

    redact(&RedactionPolicy::default(), &mut snapshot);

    for node in &snapshot.nodes {
        assert!(
            !node.images.contains_key(&ImageRole::Preview),
            "thumbnail stripped",
        );
        assert!(
            !node.images.contains_key(&ImageRole::Favicon),
            "favicon stripped",
        );
        assert!(
            node.session_state.is_none(),
            "session state (scroll + form draft) stripped",
        );
    }
    assert_eq!(snapshot.nodes.len(), 2, "graph structure kept");
}

#[test]
fn include_all_keeps_private_fields() {
    let mut snapshot = sample_graph().to_snapshot();
    for node in &mut snapshot.nodes {
        node.images
            .insert(ImageRole::Preview, ImageRef::new([9u8; 32], 1, 1));
        node.images
            .insert(ImageRole::Favicon, ImageRef::new([9u8; 32], 1, 1));
    }
    redact(&RedactionPolicy::include_all(), &mut snapshot);
    assert!(
        snapshot
            .nodes
            .iter()
            .all(|n| n.images.contains_key(&ImageRole::Preview)),
        "include_all keeps thumbnails (opt-in, never the default)",
    );
}
