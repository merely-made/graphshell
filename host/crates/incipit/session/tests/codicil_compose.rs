// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! pandect's graph-codicil tests (Mere 675fd8111, `graph_codicil.rs`), second
//! half: composition, handles, profiles and checked saves (engine split D272).

use eidetic::{ManifestId, ProvenanceOrigin, Timestamp};
use euclid::default::Point2D;
use incipit_session::graph_codicil::*;
use kernel::graph::Graph;
use kernel::graph::fixtures::GraphFixtures;
use mere::model::{NodeProperty, PersistedResourceFacet, PersistedResourceRecord};
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
fn list_graph_codicils_finds_saved_codicils_tagged_with_the_schema() {
    pollster::block_on(async {
        let mut store = InMemoryStore::default();
        save_graph_codicil(
            &mut store,
            &sample_graph(),
            RedactionPolicy::default(),
            Timestamp(1),
        )
        .await
        .expect("save");

        let codicils = list_graph_codicils(&mut store).await.expect("list");
        assert_eq!(codicils.len(), 1, "the saved codicil is listed");
        assert_eq!(
            codicils[0].schema,
            graph_snapshot_schema_ref(),
            "tagged with the graph-snapshot schema",
        );
    });
}

#[test]
fn codicil_survives_a_store_close_and_reopen() {
    // The faithful "survives restart" proof: save through a real fjall store,
    // drop it (shutdown), reopen at the same path, and thaw the same graph.
    use muniment::FjallStore;

    let dir = std::env::temp_dir().join("mere_graph_codicil_fjall_reopen");
    let _ = std::fs::remove_dir_all(&dir);

    let id = pollster::block_on(async {
        let mut store = FjallStore::open(&dir).expect("open store");
        save_graph_codicil(
            &mut store,
            &sample_graph(),
            RedactionPolicy::default(),
            Timestamp(1_700_000_000_000),
        )
        .await
        .expect("save")
        // `store` drops here — the keyspace closes, as on shutdown.
    });

    let opened = pollster::block_on(async {
        let mut store = FjallStore::open(&dir).expect("reopen store");
        open_codicil_as_session(&mut store, id)
            .await
            .expect("load ok")
    })
    .expect("the codicil is present after a store reopen");
    assert_eq!(
        opened.nodes().count(),
        2,
        "the frozen graph survives a store close + reopen (persisted to disk)",
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn open_unknown_id_returns_none() {
    pollster::block_on(async {
        let mut store = InMemoryStore::default();
        let unknown = ManifestId::of_blob(b"never-saved");
        assert!(
            open_codicil_as_session(&mut store, unknown)
                .await
                .expect("load ok")
                .is_none(),
            "an unknown id thaws to None, not an error",
        );
    });
}

#[test]
fn compose_unions_two_codicils_and_records_the_upstream_lineage() {
    pollster::block_on(async {
        let mut store = InMemoryStore::default();
        // Codicil A: x, y. Codicil B: y (shared url), z.
        let mut ga = Graph::new();
        ga.add_node("https://x.example".to_string(), Point2D::new(0.0, 0.0));
        ga.add_node("https://y.example".to_string(), Point2D::new(1.0, 0.0));
        set_pinned(&mut ga, "https://y.example", false);
        let ay = ga.get_node_by_url("https://y.example").unwrap().1.id;
        let mut gb = Graph::new();
        gb.add_node("https://y.example".to_string(), Point2D::new(0.0, 0.0));
        gb.add_node("https://z.example".to_string(), Point2D::new(1.0, 0.0));
        set_pinned(&mut gb, "https://y.example", true);
        set_pinned(&mut gb, "https://z.example", true);
        let by = gb.get_node_by_url("https://y.example").unwrap().1.id;

        let id_a = save_graph_codicil(&mut store, &ga, RedactionPolicy::default(), Timestamp(1))
            .await
            .expect("save a");
        let id_b = save_graph_codicil(&mut store, &gb, RedactionPolicy::default(), Timestamp(2))
            .await
            .expect("save b");

        let composed = compose_graph_codicils(
            &mut store,
            &[id_a, id_b],
            RedactionPolicy::default(),
            Timestamp(3),
        )
        .await
        .expect("compose ok")
        .expect("a non-empty id list composes a codicil");

        // Content may share a URL while each browsing surface stays distinct.
        let graph = open_codicil_as_session(&mut store, composed)
            .await
            .expect("load ok")
            .expect("the composed codicil is present");
        assert_eq!(
            graph.nodes().count(),
            4,
            "x, two independent y surfaces, and z"
        );
        assert!(graph.get_node_by_url("https://x.example").is_some());
        assert!(graph.get_node_by_url("https://z.example").is_some());
        let y = graph.get_node_key_by_id(ay).unwrap();
        let other_y = graph.get_node_key_by_id(by).unwrap();
        let (z, _) = graph.get_node_by_url("https://z.example").unwrap();
        assert_eq!(
            graph.node_is_pinned(y),
            Some(false),
            "A's surface keeps its own context"
        );
        assert_eq!(
            graph.node_is_pinned(other_y),
            Some(true),
            "B's surface keeps its own context"
        );
        assert_eq!(
            graph.node_is_pinned(z),
            Some(true),
            "facets on an added node are remapped into the union"
        );

        // The lineage: the composed codicil is `Derived` and names both sources —
        // the `upstream` Vec that is empty on every freeze, finally populated.
        let manifests = list_graph_codicils(&mut store).await.expect("list");
        let m = manifests
            .iter()
            .find(|m| m.id == composed)
            .expect("the composed manifest is listed");
        assert_eq!(
            m.provenance.origin,
            ProvenanceOrigin::Derived,
            "a composed codicil is Derived"
        );
        assert_eq!(
            m.provenance.upstream,
            vec![id_a, id_b],
            "upstream records the source codicils",
        );
    });
}

#[test]
fn compose_of_an_empty_id_list_is_none() {
    pollster::block_on(async {
        let mut store = InMemoryStore::default();
        assert!(
            compose_graph_codicils(&mut store, &[], RedactionPolicy::default(), Timestamp(1))
                .await
                .expect("ok")
                .is_none(),
            "no sources -> nothing composed",
        );
    });
}

#[test]
fn codicil_surface_sidecars_cannot_reuse_resource_handles() {
    pollster::block_on(async {
        let mut graph = sample_graph();
        let (surface, surface_id) = graph
            .nodes()
            .next()
            .map(|(key, node)| (key, node.id))
            .unwrap();
        let mut property = NodeProperty::new("urn:test:predicate".into(), "exact".into())
            .with_metadata(Some("urn:test:source".into()), Some(42));
        property.statement_id = "resource-literal-handle".into();
        assert!(graph.append_node_properties(surface, vec![property.clone()]));
        let original = graph.to_snapshot();
        let mut store = InMemoryStore::default();
        for collision in [false, true] {
            let mut facets = graph.facets().clone();
            let mut carried = property.clone();
            if !collision {
                carried.statement_id = "surface-literal-handle".into();
            }
            facets
                .set(
                    surface_id,
                    portolan::FacetId::new(kernel::graph::node_facets::SEMANTIC_PROPERTIES),
                    serde_json::to_value(vec![carried.clone(), carried]).unwrap(),
                    &portolan::AcceptAll,
                )
                .unwrap();
            facets
                .set(
                    surface_id,
                    portolan::FacetId::new("extension.origin-note"),
                    serde_json::json!({"statement_id": "resource-literal-handle"}),
                    &portolan::AcceptAll,
                )
                .unwrap();
            let codicil = GraphCodicil {
                snapshot: original.clone(),
                facets: facets.clone(),
                sessions: Vec::new(),
            };
            assert_eq!(codicil.clone().into_graph().is_err(), collision);
            assert_eq!(
                ProfiledGraphCodicil {
                    codicil,
                    placement: Some(PlacementProfile::RecordedStrataV1)
                }
                .into_graph()
                .is_err(),
                collision
            );
            let saved = save_graph_snapshot_codicil(
                &mut store,
                original.clone(),
                facets.clone(),
                RedactionPolicy::default(),
                Timestamp(2),
            )
            .await;
            assert_eq!(saved.is_err(), collision);
            let id = left_unchecked(
                &mut store,
                &GraphCodicil {
                    snapshot: original.clone(),
                    facets,
                    sessions: Vec::new(),
                },
            )
            .await;
            assert_eq!(
                open_codicil_as_session(&mut store, id).await.is_err(),
                collision
            );
            let count = list_graph_codicils(&mut store).await.unwrap().len();
            let composed = compose_graph_codicils(
                &mut store,
                &[id, id],
                RedactionPolicy::default(),
                Timestamp(3),
            )
            .await;
            assert_eq!(composed.is_err(), collision);
            if collision {
                assert_eq!(list_graph_codicils(&mut store).await.unwrap().len(), count);
            } else {
                let restored = open_codicil_as_session(&mut store, composed.unwrap().unwrap())
                    .await
                    .unwrap()
                    .unwrap();
                assert_eq!(
                    restored.resource_properties(graph.shown_resource_id(surface).unwrap()),
                    vec![property.clone()]
                );
                assert_eq!(
                    restored.facets().get(
                        &surface_id,
                        &portolan::FacetId::new("extension.origin-note")
                    ),
                    Some(&serde_json::json!({"statement_id": "resource-literal-handle"}))
                );
            }
        }
    });
}

#[test]
fn invalid_resource_codicil_open_and_compose_fail_with_valid_control() {
    pollster::block_on(async {
        let mut store = InMemoryStore::default();
        let graph = sample_graph();
        let valid =
            save_graph_codicil(&mut store, &graph, RedactionPolicy::default(), Timestamp(1))
                .await
                .unwrap();
        assert!(
            open_codicil_as_session(&mut store, valid)
                .await
                .unwrap()
                .is_some()
        );
        let mut snapshot = graph.to_snapshot();
        snapshot.resources.push(PersistedResourceRecord {
            canonical_iri: "https://invalid.test/resource".into(),
            facets: vec![PersistedResourceFacet {
                facet: "test.data".into(),
                value_json: "invalid JSON".into(),
            }],
        });
        assert!(
            save_graph_snapshot_codicil(
                &mut store,
                snapshot.clone(),
                graph.facets().clone(),
                RedactionPolicy::default(),
                Timestamp(2),
            )
            .await
            .is_err()
        );
        let invalid = left_unchecked(
            &mut store,
            &GraphCodicil {
                snapshot,
                facets: graph.facets().clone(),
                sessions: Vec::new(),
            },
        )
        .await;
        let before = list_graph_codicils(&mut store).await.unwrap().len();
        assert!(open_codicil_as_session(&mut store, invalid).await.is_err());
        assert!(
            compose_graph_codicils(
                &mut store,
                &[valid, invalid],
                RedactionPolicy::default(),
                Timestamp(3)
            )
            .await
            .is_err()
        );
        assert_eq!(
            list_graph_codicils(&mut store).await.unwrap().len(),
            before,
            "failed composition stores no partial result"
        );
        assert!(
            compose_graph_codicils(
                &mut store,
                &[valid, valid],
                RedactionPolicy::default(),
                Timestamp(4)
            )
            .await
            .unwrap()
            .is_some()
        );
    });
}

#[test]
fn a_codicil_save_refuses_what_checked_load_refuses() {
    pollster::block_on(async {
        let mut store = InMemoryStore::default();
        let graph = sample_graph();
        let mut repeated = graph.to_snapshot();
        repeated.nodes.push(repeated.nodes[0].clone());
        assert!(Graph::try_from_snapshot(&repeated).is_err());
        let policy = RedactionPolicy::default();

        assert!(
            save_graph_snapshot_codicil(
                &mut store,
                repeated.clone(),
                graph.facets().clone(),
                policy,
                Timestamp(1),
            )
            .await
            .is_err()
        );
        for placement in [None, Some(PlacementProfile::RecordedStrataV1)] {
            let input = ProfiledGraphCodicil {
                codicil: GraphCodicil {
                    snapshot: repeated.clone(),
                    facets: graph.facets().clone(),
                    sessions: Vec::new(),
                },
                placement,
            };
            assert!(
                save_profiled_graph_codicil_sealed(&mut store, None, input, policy, Timestamp(2))
                    .await
                    .is_err()
            );
        }
        assert!(list_graph_codicils(&mut store).await.unwrap().is_empty());

        let id = save_graph_codicil(&mut store, &graph, policy, Timestamp(3))
            .await
            .unwrap();
        assert!(
            open_codicil_as_session(&mut store, id)
                .await
                .unwrap()
                .is_some()
        );
    });
}

#[test]
fn a_profiled_codicil_save_materializes_with_its_placement() {
    pollster::block_on(async {
        let mut store = InMemoryStore::default();
        let mut graph = placement_test_graph();
        let (surface, _) = graph.nodes().next().unwrap();
        let surface_id = graph.get_node(surface).unwrap().id;
        let mut held = NodeProperty::new("urn:test:predicate".into(), "held".into());
        held.statement_id = "owned-handle".into();
        assert!(graph.append_node_properties(surface, vec![held.clone()]));
        let recorded = ProfiledGraphCodicil {
            placement: Some(PlacementProfile::RecordedStrataV1),
            codicil: GraphCodicil {
                snapshot: graph.to_snapshot(),
                facets: graph.facets().clone(),
                sessions: Vec::new(),
            },
        };
        let policy = RedactionPolicy::include_all();
        let id = save_profiled_graph_codicil_sealed(
            &mut store,
            None,
            recorded.clone(),
            policy,
            Timestamp(1),
        )
        .await
        .unwrap();
        let reopened = open_codicil_as_session(&mut store, id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            reopened.to_snapshot().edges,
            recorded.codicil.snapshot.edges
        );

        // Facets that take a Resource's handle are refused by the save, as
        // the open that would follow refuses them.
        let mut stolen = recorded;
        stolen
            .codicil
            .facets
            .set(
                surface_id,
                portolan::FacetId::new(kernel::graph::node_facets::SEMANTIC_PROPERTIES),
                serde_json::to_value(vec![held]).unwrap(),
                &portolan::AcceptAll,
            )
            .unwrap();
        assert!(stolen.clone().into_graph().is_err());
        assert!(
            save_profiled_graph_codicil_sealed(&mut store, None, stolen, policy, Timestamp(2))
                .await
                .is_err()
        );
        assert_eq!(list_graph_codicils(&mut store).await.unwrap().len(), 1);
    });
}
