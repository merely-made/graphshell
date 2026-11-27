// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! pandect's session tests whose code is now Graphshell's (Mere 675fd8111,
//! `graph_session.rs` and `graph_session_refusal_tests.rs`, engine split
//! D272): the component fork, archive compose, in-process graph deltas and
//! `transact_live`'s whole-change refusal, over incipit-graph.

use euclid::default::Point2D;
use incipit_session::{IncipitSession, IncipitSessions};
use kernel::graph::apply::GraphDelta;
use kernel::graph::{CapturedDelta, Graph};
use mere::model::author::Author;
use mere::model::{SemanticStatementSpec, SemanticSubKind};
use muniment::journal::Seq;
use muniment::{Backend, MemoryBackend};
use pandect::{ChangeKind, SessionError};
use std::time::SystemTime;
use uuid::Uuid;

type MereSessions<B> = pandect::MereSessions<B, Graph>;
type GraphSession<B> = pandect::GraphSession<B, Graph>;

async fn stored_bytes(store: &MemoryBackend) -> Vec<(String, Vec<u8>)> {
    let mut keys = store.list("").await.unwrap();
    keys.sort();
    let mut bytes = Vec::new();
    for key in keys {
        bytes.push((key.clone(), store.get(&key).await.unwrap().unwrap()));
    }
    bytes
}

/// A graph's whole persisted state: its snapshot, undated, and its facets.
fn whole(graph: &Graph) -> (serde_json::Value, serde_json::Value) {
    let mut snapshot = graph.to_snapshot();
    snapshot.timestamp_secs = 0;
    (
        serde_json::to_value(&snapshot).unwrap(),
        serde_json::to_value(graph.facets()).unwrap(),
    )
}

/// What a refused change leaves as it was.
type Held = ((serde_json::Value, serde_json::Value), Seq, usize, u64);

fn held(session: &GraphSession<MemoryBackend>) -> Held {
    (
        whole(session.graph()),
        session.journal().live_cursor(),
        session.changes().len(),
        session.revision(),
    )
}

/// The live graph `a -> b` with one Resource statement, and its handle.
fn related() -> (Graph, String) {
    let mut graph = kernel::graph::replay_captured_deltas([add(1), add(2)]);
    let key = |graph: &Graph, id| graph.get_node_key_by_id(Uuid::from_u128(id)).unwrap();
    let (from, to) = (key(&graph, 1), key(&graph, 2));
    let spec = SemanticStatementSpec {
        predicate: "https://import.test/vocab#next".into(),
        ..Default::default()
    };
    let (relation, _) = graph.assert_semantic_statement(from, to, spec).unwrap();
    let handle = graph.get_relation(relation).unwrap().assertions()[0]
        .statement_id
        .clone();
    (graph, handle)
}

fn person() -> Author {
    Author::person("persona-a").via("turnstone")
}

fn knot() -> Author {
    Author::person("persona-a").via("knot-editor")
}

fn add(id: u128) -> CapturedDelta {
    CapturedDelta::ReplayAddNodeWithIdIfMissing {
        id: Uuid::from_u128(id).to_string(),
        url: format!("https://{id}.test/"),
        position: [0.0, 0.0],
    }
}

fn relate(from: u128, to: u128) -> CapturedDelta {
    CapturedDelta::ReplayAssertRelationByIds {
        from_id: Uuid::from_u128(from).to_string(),
        to_id: Uuid::from_u128(to).to_string(),
        assertion: mere::model::EdgeAssertion::Semantic {
            sub_kind: SemanticSubKind::Cites,
            label: None,
        },
    }
}

#[test]
fn archive_compose_retains_both_sources_and_checks_before_writing() {
    pollster::block_on(async {
        use incipit_session::graph_codicil::{
            compose_graph_codicils_checked, load_graph_codicil, save_session_codicil_checked,
        };
        let mut store = muniment::MemoryBackend::default();
        let sessions = MereSessions::new(store.clone());
        let mut ids = Vec::new();
        for n in [1, 2] {
            let mut s = sessions.begin(person(), None);
            s.apply(person(), vec![add(n)]).await.unwrap();
            ids.push(
                save_session_codicil_checked(
                    &mut store,
                    s.codicil_payload(),
                    eidetic::Timestamp(n as u64),
                    |_| Ok(()),
                )
                .await
                .unwrap(),
            );
        }
        let before = stored_bytes(&store).await;
        assert!(
            compose_graph_codicils_checked(&mut store, &ids, eidetic::Timestamp(3), |_| Err(
                "no union".into()
            ))
            .await
            .is_err()
        );
        assert!(
            sessions
                .open_codicil_checked(ids[0], person(), |_| Err("no thaw".into()))
                .await
                .is_err()
        );
        assert_eq!(stored_bytes(&store).await, before);
        let composed =
            compose_graph_codicils_checked(&mut store, &ids, eidetic::Timestamp(3), |_| Ok(()))
                .await
                .unwrap()
                .unwrap();
        let manifest = eidetic::manifest::load_manifest(&mut store, composed)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(manifest.provenance.upstream, ids);
        let payload = load_graph_codicil(&mut store, composed)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(payload.sessions.len(), 2);
        assert_eq!(payload.snapshot.nodes.len(), 2);
        let thaw = sessions
            .open_codicil_checked(composed, person(), |_| Ok(()))
            .await
            .unwrap();
        assert_eq!(
            sessions
                .open(thaw.session_id)
                .await
                .unwrap()
                .graph()
                .nodes()
                .count(),
            2
        );
    });
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn a_component_fork_takes_only_the_seed_s_component() {
    pollster::block_on(async {
        let store = MemoryBackend::new();
        let mere = MereSessions::new(store.clone());
        let id = mere.mint(person(), None).await.unwrap().session_id;
        let mut parent = mere.open(id).await.unwrap();
        parent
            .apply(person(), vec![add(1), add(2), add(3), relate(1, 2)])
            .await
            .unwrap();
        let fork = mere
            .fork_component(&parent, Uuid::from_u128(1), knot())
            .await
            .unwrap();
        assert_eq!(
            fork.display_name.as_deref(),
            Some("Session (picked from Session)"),
            "a picked-off part names where it was picked from"
        );
        // A pick records where it came from, and is not a fork.
        assert_eq!(
            fork.picked_from,
            Some(pandect::manifest::PickedFrom {
                session: id,
                at: parent.journal().live_cursor().0,
                container: *parent.manifest().root_graph_id.as_uuid(),
            })
        );
        assert_eq!(fork.parent_session, None);
        assert_eq!(fork.forked_at, None);
        let fork = mere.open(fork.session_id).await.unwrap();
        assert_eq!(
            fork.changes()[0].kind,
            ChangeKind::Picked {
                from: id,
                at: parent.journal().live_cursor()
            }
        );
        assert_eq!(
            fork.graph().node_count(),
            2,
            "nodes 1 and 2, not the lone 3"
        );
        assert!(
            mere.fork_component(&parent, Uuid::from_u128(9), knot())
                .await
                .is_err()
        );
    });
}

#[test]
fn in_process_hosts_apply_graph_deltas_by_key() {
    pollster::block_on(async {
        let mere = MereSessions::new(MemoryBackend::new());
        let id = mere.mint(person(), None).await.unwrap().session_id;
        let mut session = mere.open(id).await.unwrap();
        session
            .apply_deltas(
                person(),
                vec![GraphDelta::AddNode {
                    id: Some(Uuid::from_u128(7)),
                    url: "https://seven.test/".into(),
                    position: Point2D::new(1.0, 2.0),
                }],
            )
            .await
            .unwrap();
        let (key, _) = session.graph().get_node_by_id(Uuid::from_u128(7)).unwrap();
        let result = session
            .apply_deltas(
                person(),
                vec![GraphDelta::SetNodeTitle {
                    key,
                    title: "Seven".into(),
                }],
            )
            .await
            .unwrap();
        assert_eq!(result.first, Seq(4));
        assert_eq!(result.end, Seq(5));
        assert_eq!(session.graph().get_node(key).unwrap().title, "Seven");
        let resource_id = mere::model::ResourceNode::new("https://seven.test/").id();
        assert_eq!(session.graph().shown_resource_id(key), Some(resource_id));
        let [added, visit, resource, shown, title] = session.journal().entries() else {
            panic!("expected surface, visit, resource, shown binding and title");
        };
        let surface_id = Uuid::from_u128(7).to_string();
        let resource_id_text = resource_id.to_string();
        assert!(
            matches!(&added.delta, CapturedDelta::ReplayAddNodeWithIdIfMissing {
            id, url, ..
        } if id == &surface_id && url == "https://seven.test/")
        );
        assert!(
            matches!(&visit.delta, CapturedDelta::ReplayTouchNodeLastVisitedById {
            node_id, ..
        } if node_id == &surface_id)
        );
        assert!(
            matches!(&resource.delta, CapturedDelta::ReplaySetResourceRecordById {
            resource_id, record: Some(record)
        } if resource_id == &resource_id_text
            && record.canonical_iri == "https://seven.test")
        );
        assert!(
            matches!(&shown.delta, CapturedDelta::ReplaySetShownResourceById {
            surface_id: captured_surface, resource_id: Some(captured_resource)
        } if captured_surface == &surface_id && captured_resource == &resource_id_text)
        );
        assert!(
            matches!(&title.delta, CapturedDelta::ReplaySetNodeTitleById {
            node_id, title
        } if node_id == &surface_id && title == "Seven")
        );
        assert!(
            session
                .journal()
                .entries()
                .iter()
                .all(|entry| entry.author == person())
        );
        drop(session);
        let reopened = mere.open(id).await.unwrap();
        let (key, node) = reopened.graph().get_node_by_id(Uuid::from_u128(7)).unwrap();
        assert_eq!(node.title, "Seven");
        assert_eq!(reopened.graph().shown_resource_id(key), Some(resource_id));
    });
}

/// `transact_live` refuses a change whole (engine-split D166): commands whose
/// middle edit gives a Surface a literal holding a held handle are refused,
/// naming the edit and the handle; the same commands with a fresh handle
/// commit whole.
#[test]
fn transact_live_refuses_a_change_whole() {
    use incipit_session::addressable_graph::{AddressableGraphStore, GraphAddress};
    use incipit_session::live_graph::{LiveCheck, LiveCommand, LiveEditError, ObservedCommand};
    pollster::block_on(async {
        let store = MemoryBackend::new();
        let mere = MereSessions::new(store.clone());
        let (graph, handle) = related();
        let mut session = mere.begin_recorded(person(), Some(graph));
        session.flush(SystemTime::now()).await.unwrap();
        let (one, nine) = (Uuid::from_u128(1), Uuid::from_u128(9));
        // The record as a publication of the live graph names it.
        let digest = AddressableGraphStore::new(MemoryBackend::new(), Uuid::nil())
            .prepare_recorded_graph(session.graph())
            .unwrap()
            .record_digest(GraphAddress::Surface(one))
            .unwrap()
            .to_owned();
        let observed = vec![LiveCheck::RecordUnchanged {
            address: GraphAddress::Surface(one),
            digest,
        }];
        let commands = |statement: &str| {
            let mut property = mere::model::NodeProperty::new(
                "https://live.test/vocab#note".to_string(),
                "held".to_string(),
            );
            property.statement_id = statement.into();
            vec![
                ObservedCommand {
                    command: LiveCommand::SetPinned {
                        surface: one,
                        pinned: true,
                    },
                    checks: observed.clone(),
                },
                ObservedCommand {
                    command: LiveCommand::SetFacet {
                        surface: one,
                        facet: kernel::graph::node_facets::SEMANTIC_PROPERTIES.into(),
                        value: serde_json::to_value([property]).unwrap(),
                    },
                    checks: observed.clone(),
                },
                ObservedCommand {
                    command: LiveCommand::CreateAddress {
                        id: nine,
                        address: "https://live.test/nine".into(),
                        title: "Nine".into(),
                    },
                    checks: vec![LiveCheck::Absent(GraphAddress::Surface(nine))],
                },
            ]
        };
        let before = held(&session);
        let stored = stored_bytes(&store).await;

        let refused = session
            .transact_live(person(), commands(&handle), SystemTime::now())
            .await
            .unwrap_err();
        assert!(
            matches!(refused, LiveEditError::Session(SessionError::Validation(_))),
            "{refused:?}"
        );
        let message = refused.to_string();
        assert!(message.contains(&handle), "{message}");
        assert!(message.contains("edit 1"), "{message}");
        assert_eq!(held(&session), before, "nothing is applied");
        assert_eq!(stored_bytes(&store).await, stored, "nothing is stored");

        // Control: the same commands with a fresh handle commit whole.
        let (live, _) = session
            .transact_live(person(), commands("fresh-live"), SystemTime::now())
            .await
            .unwrap();
        assert!(live.applied.end > live.applied.first);
        assert_eq!(live.created, vec![nine]);
        let graph = session.graph();
        let key = graph.get_node_key_by_id(one).unwrap();
        assert_eq!(graph.node_is_pinned(key), Some(true));
        assert!(graph.get_node_key_by_id(nine).is_some());
        assert_ne!(stored_bytes(&store).await, stored, "stored");
    });
}
