// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! What pandect wrote over mere-kernel opens through incipit-session over
//! incipit-graph, unchanged (engine split D256, step 2b). Since Mere retired
//! the kernel (lane 4), each case reads stores the kernel wrote, frozen in
//! `tests/fixtures/kernel_written`, compares the digest or the snapshot with
//! what the kernel read back and, where there is one, the bytes a re-save
//! writes with the bytes the kernel's re-save wrote, and carries a control
//! the comparison must refuse.

use std::collections::BTreeMap;
use std::time::SystemTime;

use eidetic::Timestamp;
use incipit_session::addressable_graph::{AddressableGraphStore, ResidencyRoots};
use incipit_session::{IncipitSession, IncipitSessions};
use mere::model::author::Author;
use muniment::{Backend, MemoryBackend, WriteOp};
use pandect::graph_codicil::RedactionPolicy;
use pandect::graph_placement::PlacementProfile;
use pandect::{SessionError, SessionId};
use serde::Serialize;
use serde_json::{Value, json};
use uuid::Uuid;

#[path = "../common/kernel_written.rs"]
mod kernel_written;
use kernel_written::{expected, held, store, uuid_of};

type Graph = kernel::graph::Graph;
type Sessions = pandect::MereSessions<MemoryBackend, Graph>;

const AT: SystemTime = SystemTime::UNIX_EPOCH;

fn author() -> Author {
    Author::person("cross-read").via("graphshell")
}

fn id(n: u128) -> Uuid {
    Uuid::from_u128(0x5b00 + n)
}

fn session_id(value: &Value) -> SessionId {
    SessionId::from_uuid(uuid_of(value))
}

/// A graph's snapshot and facets as JSON: what two kernels' graphs compare by.
fn whole(snapshot: &impl Serialize, facets: &impl Serialize) -> Value {
    json!({"snapshot": snapshot, "facets": facets})
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

/// `ours` with the snapshot date at `pointer` set to the kernel's. A re-save
/// dates its snapshot by the wall clock (`Graph::to_snapshot`), so it can
/// match the kernel's frozen re-save only once both carry one date.
fn dated_as_kernel(mut ours: Value, kernel: &Value, pointer: &str) -> Value {
    let date = kernel.pointer(pointer).expect("the kernel's snapshot date");
    assert!(date.is_u64(), "{pointer} is a date: {date}");
    *ours.pointer_mut(pointer).expect("a snapshot date") = date.clone();
    ours
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

mod codicil;
mod store;
mod sync_legacy;
