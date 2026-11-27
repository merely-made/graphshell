// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! What pandect writes today over mere-kernel opens through incipit-session
//! over incipit-graph, unchanged (engine split D256, step 2b). Each case
//! writes with the kernel's path and reads with incipit's, compares the
//! digest or the snapshot and, where there is one, the bytes a re-save
//! writes, and carries a control the comparison must refuse.

use std::collections::BTreeMap;
use std::time::SystemTime;

use eidetic::Timestamp;
use incipit_session::addressable_graph::{AddressableGraphStore, ResidencyRoots};
use incipit_session::{IncipitSession, IncipitSessions};
use kernel::graph::NodeHistoryTransitionKind;
use mere::model::author::Author;
use mere::model::{EdgeAssertion, SemanticSubKind};
use muniment::{Backend, MemoryBackend, WriteOp};
use pandect::graph_codicil::{GraphCodicil, ProfiledGraphCodicil, RedactionPolicy};
use pandect::graph_placement::PlacementProfile;
use pandect::{SessionError, SessionId};
use serde::Serialize;
use serde_json::{Value, json};
use uuid::Uuid;

type KernelGraph = mere_kernel::graph::Graph;
type Graph = kernel::graph::Graph;
type KernelSessions = pandect::MereSessions<MemoryBackend>;
type Sessions = pandect::MereSessions<MemoryBackend, Graph>;

const AT: SystemTime = SystemTime::UNIX_EPOCH;

fn author() -> Author {
    Author::person("cross-read").via("graphshell")
}

fn id(n: u128) -> Uuid {
    Uuid::from_u128(0x5b00 + n)
}

/// A graph's snapshot and facets as JSON: what two kernels' graphs compare by.
fn whole(snapshot: &impl Serialize, facets: &impl Serialize) -> Value {
    json!({"snapshot": snapshot, "facets": facets})
}

fn kernel_whole(graph: &KernelGraph) -> Value {
    whole(&graph.to_snapshot_at(0), graph.facets())
}

fn incipit_whole(graph: &Graph) -> Value {
    whole(&graph.to_snapshot_at(0), graph.facets())
}

/// `whole` without the visit clock a node is minted with: a projection mints
/// its nodes at the wall clock, so two projections of the same operations
/// differ there whichever graph they fold into.
#[cfg(all(feature = "personal-sync", not(target_arch = "wasm32")))]
fn unclocked(mut value: Value) -> Value {
    if let Some(nodes) = value["facets"]["nodes"].as_object_mut() {
        for facets in nodes.values_mut() {
            if let Some(visits) = facets
                .get_mut("visit.history")
                .and_then(Value::as_object_mut)
            {
                visits.remove("last_visited_ms");
            }
        }
    }
    value
}

/// Every key and value the store holds.
async fn bytes(backend: &MemoryBackend) -> BTreeMap<String, Vec<u8>> {
    let mut held = BTreeMap::new();
    for key in backend.list("").await.unwrap() {
        if let Some(value) = backend.get(&key).await.unwrap() {
            held.insert(key, value);
        }
    }
    held
}

/// A second store holding exactly what `backend` holds.
async fn copy_of(backend: &MemoryBackend) -> MemoryBackend {
    let copy = MemoryBackend::new();
    let ops: Vec<WriteOp> = bytes(backend)
        .await
        .into_iter()
        .map(|(key, value)| WriteOp::Put { key, value })
        .collect();
    copy.apply(&ops).await.unwrap();
    copy
}

/// Replace the first `from` with `to` in the value at `key`.
async fn tamper(backend: &MemoryBackend, key: &str, from: &str, to: &str) {
    let value = String::from_utf8(backend.get(key).await.unwrap().unwrap()).unwrap();
    assert!(value.contains(from), "{key} holds {from}");
    backend
        .put(key, value.replacen(from, to, 1).as_bytes())
        .await
        .unwrap();
}

fn live_digest<B: Backend, G: mere::session::SessionGraph>(
    session: &pandect::GraphSession<B, G>,
) -> String {
    session
        .pin(session.journal().live_cursor())
        .unwrap()
        .digest()
        .to_owned()
}

/// A kernel session holding a varied graph, published through mere-kernel's
/// addressable store: titled, tagged and pinned nodes, a facet, an access
/// record, a semantic relation, a visit and a removal. Its id, its container
/// and the generation it published.
async fn kernel_written(backend: &MemoryBackend) -> (SessionId, Uuid, String) {
    use mere_kernel::graph::CapturedDelta;
    use mere_kernel::graph::apply::GraphDelta;
    let sessions = KernelSessions::new(backend.clone());
    let mut session = sessions.begin_recorded(author(), None);
    session.flush(AT).await.unwrap();
    let adds = (1..=4)
        .map(|n| {
            CapturedDelta::ReplayAddNodeWithIdIfMissing {
                id: id(n).to_string(),
                url: format!("https://cross.test/{n}"),
                position: [n as f32, 0.0],
            }
            .replay_as(&author())
            .into_write()
            .unwrap()
        })
        .collect();
    session
        .transact_deltas_published(author(), adds, AT)
        .await
        .unwrap();
    let edits = vec![
        GraphDelta::ReplaySetNodeTitleById {
            node_id: id(1),
            title: "One".into(),
        },
        GraphDelta::ReplayInsertNodeTagById {
            node_id: id(1),
            tag: "kept".into(),
        },
        GraphDelta::ReplaySetNodeFacetById {
            node_id: id(2),
            facet: "cross.note".into(),
            value: json!({"n": 2}),
        },
        GraphDelta::ReplayAssertRelationByIds {
            from_id: id(1),
            to_id: id(2),
            assertion: EdgeAssertion::Semantic {
                sub_kind: SemanticSubKind::Cites,
                label: None,
            },
            asserter_iri: author().asserter_iri(),
        },
        GraphDelta::ReplayNavigateNodeById {
            node_id: id(3),
            url: "https://cross.test/3/visited".into(),
            transition: NodeHistoryTransitionKind::UrlTyped,
            timestamp_ms: 10,
            last_session_visited: 0,
        },
        GraphDelta::ReplayRemoveNodeById { node_id: id(4) },
    ];
    session
        .transact_deltas_published(author(), edits, AT)
        .await
        .unwrap();
    let key = session.graph().get_node_key_by_id(id(2)).unwrap();
    let pin = vec![GraphDelta::SetNodePinned {
        key,
        is_pinned: true,
    }];
    session
        .transact_deltas_published(author(), pin, AT)
        .await
        .unwrap();
    // An access record rides as a node facet, written by pandect's kernel
    // access writer through the session's edit path.
    let key = session.graph().get_node_key_by_id(id(1)).unwrap();
    session
        .edit_now(author(), |graph| {
            pandect::access::record_observation(graph, key, &observation()).unwrap()
        })
        .unwrap();
    let (_, generation) = session
        .transact_deltas_published(author(), Vec::new(), AT)
        .await
        .unwrap();
    (
        session.id(),
        *session.manifest().root_graph_id.as_uuid(),
        generation,
    )
}

fn observation() -> pandect::access::AccessObservation {
    use pandect::access::{AccessAction, AccessTransition};
    pandect::access::AccessObservation {
        record_id: Uuid::from_u128(0xacc),
        action: AccessAction::Open,
        persona: "personae://profile/cross".into(),
        device: "cross".into(),
        application: "graphshell".into(),
        handler: "graphshell.open".into(),
        at_ms: 5,
        dwell_ms: None,
        referring_container_id: None,
        referring_address: None,
        transition: AccessTransition::Unknown,
        capture_source: "cross.read".into(),
        source_event_id: None,
        privacy: eidetic::PrivacyClass::LocalOnly,
    }
}

mod codicil;
mod store;
mod sync_legacy;
