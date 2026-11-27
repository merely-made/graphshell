// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! What pandect's codicil tests shared, over incipit-graph (engine split D272).

#![allow(dead_code)]

use eidetic::{
    BlobSource, ManifestId, PrivacyClass, ProvenanceOrigin, ProvenanceRecord, SchemaRef, Timestamp,
    TrustEnvelope, TypedPayload, save_payload_sealed,
};
use incipit_session::graph_codicil::{
    GRAPH_SNAPSHOT_SCHEMA_ID, GraphCodicil, LEGACY_GRAPH_SNAPSHOT_SCHEMA_ID,
    PROFILED_GRAPH_SNAPSHOT_SCHEMA_ID, ProfiledGraphCodicil, V2_GRAPH_SNAPSHOT_SCHEMA_ID,
    V3_PROFILED_GRAPH_SNAPSHOT_SCHEMA_ID,
};
use kernel::graph::{CapturedDelta, Graph};
use kernel::persistence::GraphSnapshot;
use mere::model::{EdgeAssertion, SemanticSubKind};
use muniment::MemoryBackend;
use pandect::NodeFacetStore;
use pandect::graph_placement::{PlacementProfile, materialize_snapshot_with};
use serde::{Deserialize, Serialize};

/// Provenance for a codicil a test writes directly, as pandect's freeze
/// names it: generated at `created_at`, no upstream.
pub fn graph_codicil_provenance(created_at: Timestamp) -> ProvenanceRecord {
    ProvenanceRecord {
        origin: ProvenanceOrigin::Generated,
        upstream: Vec::new(),
        tooling: Some("pandect/graph-codicil".into()),
        generated_at: created_at,
    }
}

pub fn schema(id: &[u8]) -> SchemaRef {
    SchemaRef::from_id(ManifestId::of_blob(id))
}

/// The input adapters' typed bindings, which the tests write old codicils with.
pub fn legacy_graph_snapshot_schema_ref() -> SchemaRef {
    schema(LEGACY_GRAPH_SNAPSHOT_SCHEMA_ID)
}

pub fn v2_graph_snapshot_schema_ref() -> SchemaRef {
    schema(V2_GRAPH_SNAPSHOT_SCHEMA_ID)
}

pub fn v3_profiled_graph_snapshot_schema_ref() -> SchemaRef {
    schema(V3_PROFILED_GRAPH_SNAPSHOT_SCHEMA_ID)
}

pub fn current_graph_snapshot_schema_ref() -> SchemaRef {
    schema(GRAPH_SNAPSHOT_SCHEMA_ID)
}

/// pandect's recorded placement fixture (Mere 675fd8111, `graph_placement.rs`):
/// two nodes and one held custom claim, read under recorded rules.
pub fn placement_test_graph() -> Graph {
    let mut graph = kernel::graph::replay_captured_deltas((1..=2).map(|id| {
        CapturedDelta::ReplayAddNodeWithIdIfMissing {
            id: uuid::Uuid::from_u128(id).to_string(),
            url: if id == 1 {
                "https://profile.test/".into()
            } else {
                "https://profile.test/child".into()
            },
            position: [id as f32, 0.0],
        }
    }));
    kernel::graph::replay_captured_deltas_onto(
        &mut graph,
        [CapturedDelta::ReplayAssertRelationByIds {
            from_id: uuid::Uuid::from_u128(1).to_string(),
            to_id: uuid::Uuid::from_u128(2).to_string(),
            assertion: EdgeAssertion::Semantic {
                sub_kind: SemanticSubKind::UserGrouped,
                label: Some("held surface".into()),
            },
        }],
    );
    let mut snapshot = graph.to_snapshot();
    let claim = &mut snapshot.edges[0].assertions.as_mut().unwrap()[0];
    claim.statement_id = "held-profile-handle".into();
    claim.recognized_sub_kind = None;
    claim.predicate = "urn:mere:profile:held-custom".into();
    claim.provenance_iri = Some("urn:mere:profile:source".into());
    claim.asserted_at_ms = Some(42);
    let mut recorded = Graph::try_from_recorded_snapshot(&snapshot).unwrap();
    recorded.install_facets(graph.facets().clone()).unwrap();
    recorded
}

/// pandect's typed bindings for the payloads the tests write directly
/// (its private `TypedPayload` impls, Mere 675fd8111 `graph_codicil.rs`).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Profiled(pub ProfiledGraphCodicil<Graph>);

impl TypedPayload for Profiled {
    fn schema_ref() -> SchemaRef {
        schema(PROFILED_GRAPH_SNAPSHOT_SCHEMA_ID)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Plain(pub GraphCodicil<Graph>);

impl TypedPayload for Plain {
    fn schema_ref() -> SchemaRef {
        current_graph_snapshot_schema_ref()
    }
}

/// The v2 JSON envelope, still readable after the current codicil advanced.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct V2GraphCodicil(pub GraphCodicil<Graph>);

impl TypedPayload for V2GraphCodicil {
    fn schema_ref() -> SchemaRef {
        v2_graph_snapshot_schema_ref()
    }
}

/// The bare-snapshot v1 envelope.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LegacyGraphCodicil(pub GraphSnapshot);

impl TypedPayload for LegacyGraphCodicil {
    fn schema_ref() -> SchemaRef {
        legacy_graph_snapshot_schema_ref()
    }
}

/// A codicil payload a writer that checked nothing could leave.
pub trait Unchecked {
    fn typed_bytes(&self) -> Box<dyn Fn() -> (SchemaRef, Vec<u8>) + '_>;
}

impl Unchecked for ProfiledGraphCodicil<Graph> {
    fn typed_bytes(&self) -> Box<dyn Fn() -> (SchemaRef, Vec<u8>) + '_> {
        Box::new(|| (Profiled::schema_ref(), serde_json::to_vec(self).unwrap()))
    }
}

impl Unchecked for GraphCodicil<Graph> {
    fn typed_bytes(&self) -> Box<dyn Fn() -> (SchemaRef, Vec<u8>) + '_> {
        Box::new(|| (Plain::schema_ref(), serde_json::to_vec(self).unwrap()))
    }
}

/// A codicil as a writer that checked nothing would have left it, under the
/// schema eidetic's typed save gives its payload.
pub async fn left_unchecked<T: Unchecked>(store: &mut MemoryBackend, payload: &T) -> ManifestId {
    let (schema, bytes) = (payload.typed_bytes())();
    save_payload_sealed(
        store,
        None,
        schema,
        &bytes,
        Vec::<BlobSource>::new(),
        PrivacyClass::LocalOnly,
        graph_codicil_provenance(Timestamp(9)),
        TrustEnvelope::self_asserted(),
        Timestamp(9),
    )
    .await
    .unwrap()
}

/// The graph a codicil loads as: pandect's checked load, kernel-free.
pub trait IntoGraph {
    fn into_graph(self) -> eidetic::Result<Graph>;
}

fn checked(
    snapshot: &GraphSnapshot,
    facets: NodeFacetStore,
    placement: Option<PlacementProfile>,
) -> eidetic::Result<Graph> {
    let mut graph = materialize_snapshot_with::<Graph>(snapshot, placement)
        .map_err(|error| eidetic::Error::new(format!("invalid graph codicil snapshot: {error}")))?;
    graph
        .install_facets(facets)
        .map_err(|error| eidetic::Error::new(format!("invalid graph codicil facets: {error}")))?;
    Ok(graph)
}

impl IntoGraph for ProfiledGraphCodicil<Graph> {
    fn into_graph(self) -> eidetic::Result<Graph> {
        checked(&self.codicil.snapshot, self.codicil.facets, self.placement)
    }
}

impl IntoGraph for GraphCodicil<Graph> {
    fn into_graph(self) -> eidetic::Result<Graph> {
        checked(&self.snapshot, self.facets, None)
    }
}
