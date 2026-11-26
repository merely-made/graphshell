// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Graph codicils over incipit-graph (engine split D265, D266). The payloads,
//! schema ids and the generic save, open and list paths are pandect's and
//! re-exported; the kernel-named functions are here over this crate's graph,
//! through [`INCIPIT_CODICIL`], which names the same v4 ids as pandect's
//! `KERNEL_CODICIL`, so a codicil either kernel saved opens through the
//! other. The redaction and compose are copied from pandect's
//! `graph_codicil.rs` (Mere 36a13570b), compose over this crate's
//! [`snapshot_merge`](crate::snapshot_merge).

pub use pandect::graph_codicil::*;

use eidetic::{
    BlobManifest, BlobSource, Error, PayloadSealer, PrivacyClass, ProvenanceOrigin,
    ProvenanceRecord, Result, SchemaRef, Store, Timestamp, TrustEnvelope, save_payload_sealed,
};
use kernel::graph::Graph;
use kernel::persistence::GraphSnapshot;
use kernel::types::ImageRole;
use pandect::graph_placement::PlacementProfile;
use pandect::{FacetId, NodeFacetStore};
use uuid::Uuid;

use crate::sessions::INCIPIT_CODICIL;

/// Strip what `policy` excludes from every node in `snapshot`, in place: the
/// kernel's `RedactionPolicy::apply` over this crate's snapshot.
pub fn redact(policy: &RedactionPolicy, snapshot: &mut GraphSnapshot) {
    for node in &mut snapshot.nodes {
        // Redaction drops the *reference*; the blob is the orphan sweep's
        // business. An exported codicil therefore names no image the
        // recipient could resolve, which is what redaction owes.
        if !policy.include_thumbnails {
            node.images.remove(&ImageRole::Preview);
            node.images.remove(&ImageRole::Snapshot);
        }
        if !policy.include_favicons {
            node.images.remove(&ImageRole::Favicon);
        }
        if !policy.include_session_state {
            node.session_state = None;
        }
    }
}

/// The facet half of a redaction, as pandect's private `apply_facets`.
fn redact_facets(policy: &RedactionPolicy, facets: &mut NodeFacetStore) {
    if policy.include_session_state {
        return;
    }
    let private_runtime_facets = [
        "web.scroll",
        "web.form_draft",
        "web.viewer",
        "web.compat",
        "web.content",
        "web.page_scale",
    ];
    let nodes = facets.iter().map(|(node, _)| *node).collect::<Vec<_>>();
    for node in nodes {
        for facet in private_runtime_facets {
            facets.remove(&node, &FacetId::new(facet));
        }
    }
}

/// Freeze a live `graph` into a graph codicil, returning its manifest id.
pub async fn save_graph_codicil(
    store: &mut dyn Store,
    graph: &Graph,
    redaction: RedactionPolicy,
    created_at: Timestamp,
) -> Result<ManifestId> {
    save_graph_codicil_sealed(store, None, graph, redaction, created_at).await
}

/// As [`save_graph_codicil`], sealing the codicil at rest under `sealer`.
pub async fn save_graph_codicil_sealed(
    store: &mut dyn Store,
    sealer: Option<&dyn PayloadSealer>,
    graph: &Graph,
    redaction: RedactionPolicy,
    created_at: Timestamp,
) -> Result<ManifestId> {
    save_graph_snapshot_codicil_sealed(
        store,
        sealer,
        graph.to_snapshot(),
        graph.facets().clone(),
        redaction,
        created_at,
    )
    .await
}

/// As [`save_graph_codicil`], from a snapshot and facet store.
pub async fn save_graph_snapshot_codicil(
    store: &mut dyn Store,
    snapshot: GraphSnapshot,
    facets: NodeFacetStore,
    redaction: RedactionPolicy,
    created_at: Timestamp,
) -> Result<ManifestId> {
    save_graph_snapshot_codicil_sealed(store, None, snapshot, facets, redaction, created_at).await
}

/// As [`save_graph_snapshot_codicil`], sealing the codicil at rest.
pub async fn save_graph_snapshot_codicil_sealed(
    store: &mut dyn Store,
    sealer: Option<&dyn PayloadSealer>,
    snapshot: GraphSnapshot,
    facets: NodeFacetStore,
    redaction: RedactionPolicy,
    created_at: Timestamp,
) -> Result<ManifestId> {
    save_graph_snapshot_codicil_sealed_with(
        store,
        sealer,
        &INCIPIT_CODICIL,
        snapshot,
        facets,
        redaction,
        created_at,
    )
    .await
}

/// Freeze caller-qualified placement with its snapshot under the profiled schema.
pub async fn save_profiled_graph_codicil_sealed(
    store: &mut dyn Store,
    sealer: Option<&dyn PayloadSealer>,
    input: ProfiledGraphCodicil<Graph>,
    redaction: RedactionPolicy,
    created_at: Timestamp,
) -> Result<ManifestId> {
    save_profiled_graph_codicil_sealed_with(
        store,
        sealer,
        &INCIPIT_CODICIL,
        input,
        redaction,
        created_at,
    )
    .await
}

/// Thaw a stored graph codicil into a live [`Graph`]; `Ok(None)` if none.
pub async fn open_codicil_as_session(
    store: &mut dyn Store,
    id: ManifestId,
) -> Result<Option<Graph>> {
    open_codicil_as_session_sealed(store, None, id).await
}

/// As [`open_codicil_as_session`], unsealing with `sealer`.
pub async fn open_codicil_as_session_sealed(
    store: &mut dyn Store,
    sealer: Option<&dyn PayloadSealer>,
    id: ManifestId,
) -> Result<Option<Graph>> {
    open_codicil_as_session_sealed_with(store, sealer, &INCIPIT_CODICIL, id).await
}

/// Load the stored payload without materializing a live graph.
pub async fn load_graph_codicil(
    store: &mut dyn Store,
    id: ManifestId,
) -> Result<Option<GraphCodicil<Graph>>> {
    load_graph_codicil_sealed(store, None, id).await
}

async fn load_graph_codicil_sealed(
    store: &mut dyn Store,
    sealer: Option<&dyn PayloadSealer>,
    id: ManifestId,
) -> Result<Option<GraphCodicil<Graph>>> {
    load_profiled_graph_codicil_sealed(store, sealer, id)
        .await?
        .map(|input| {
            if input.placement == Some(PlacementProfile::LegacySurfaceV1) {
                return Err(Error::new(
                    "legacy graph placement requires qualified replay",
                ));
            }
            Ok(input.codicil)
        })
        .transpose()
}

/// Read the explicit input profile, through [`INCIPIT_CODICIL`].
pub async fn load_profiled_graph_codicil_sealed(
    store: &mut dyn Store,
    sealer: Option<&dyn PayloadSealer>,
    id: ManifestId,
) -> Result<Option<ProfiledGraphCodicil<Graph>>> {
    load_profiled_graph_codicil_sealed_with(store, sealer, &INCIPIT_CODICIL, id).await
}

/// Manifests for v1-v4 graph codicils.
pub async fn list_graph_codicils(store: &mut dyn Store) -> Result<Vec<BlobManifest>> {
    list_graph_codicils_with(store, &INCIPIT_CODICIL).await
}

/// Save a complete local reservoir archive, unredacted, validated first.
pub async fn save_session_codicil_checked(
    store: &mut dyn Store,
    payload: GraphCodicil<Graph>,
    created_at: Timestamp,
    validate: impl FnOnce(&Graph) -> std::result::Result<(), String>,
) -> Result<ManifestId> {
    save_profiled_session_codicil_checked(
        store,
        ProfiledGraphCodicil {
            codicil: payload,
            placement: None,
        },
        created_at,
        validate,
    )
    .await
}

/// Save an archive with the source session's explicit placement.
pub async fn save_profiled_session_codicil_checked(
    store: &mut dyn Store,
    payload: ProfiledGraphCodicil<Graph>,
    created_at: Timestamp,
    validate: impl FnOnce(&Graph) -> std::result::Result<(), String>,
) -> Result<ManifestId> {
    save_profiled_session_codicil_checked_with(
        store,
        &INCIPIT_CODICIL,
        payload,
        created_at,
        validate,
    )
    .await
}

/// The provenance tooling of a compose: pandect's own string, so a compose here
/// names the manifest the kernel's compose of the same inputs names (engine
/// split D268). It follows pandect's version; the codicil cross-read catches
/// a drift.
const COMPOSE_TOOLING: &str = "pandect/graph-codicil-compose@0.2.0";

fn schema(id: &[u8]) -> SchemaRef {
    SchemaRef::from_id(ManifestId::of_blob(id))
}

/// Save `payload` under its schema with `provenance`, as pandect's archive save does.
async fn save_composed(
    store: &mut dyn Store,
    sealer: Option<&dyn PayloadSealer>,
    payload: ProfiledGraphCodicil<Graph>,
    provenance: ProvenanceRecord,
    created_at: Timestamp,
) -> Result<ManifestId> {
    let (id, bytes) = if payload.placement.is_some() {
        (
            INCIPIT_CODICIL.profiled_schema,
            serde_json::to_vec(&payload),
        )
    } else {
        (
            INCIPIT_CODICIL.graph_schema,
            serde_json::to_vec(&payload.codicil),
        )
    };
    let bytes = bytes.map_err(|e| Error::new(format!("typed serialize: {e}")))?;
    save_payload_sealed(
        store,
        sealer,
        schema(id),
        &bytes,
        Vec::<BlobSource>::new(),
        PrivacyClass::LocalOnly,
        provenance,
        TrustEnvelope::self_asserted(),
        created_at,
    )
    .await
}

/// Both inputs' placement, when both are recorded.
fn joined(a: Option<PlacementProfile>, b: Option<PlacementProfile>) -> Option<PlacementProfile> {
    match (a, b) {
        (Some(PlacementProfile::RecordedStrataV1), Some(PlacementProfile::RecordedStrataV1)) => {
            Some(PlacementProfile::RecordedStrataV1)
        },
        _ => None,
    }
}

/// Load and merge `ids` in order; `None` when any is missing.
async fn composed(
    store: &mut dyn Store,
    sealer: Option<&dyn PayloadSealer>,
    ids: &[ManifestId],
) -> Result<Option<ProfiledGraphCodicil<Graph>>> {
    let mut acc: Option<ProfiledGraphCodicil<Graph>> = None;
    for id in ids {
        let Some(payload) = load_profiled_graph_codicil_sealed(store, sealer, *id).await? else {
            return Ok(None);
        };
        payload.clone().into_graph_with()?;
        acc = Some(match acc {
            None => payload,
            Some(base) => ProfiledGraphCodicil {
                placement: joined(base.placement, payload.placement),
                codicil: merge_graph_codicils(base.codicil, payload.codicil)?,
            },
        });
    }
    Ok(acc)
}

/// Compose complete local archives after validating the merged candidate.
pub async fn compose_graph_codicils_checked(
    store: &mut dyn Store,
    ids: &[ManifestId],
    created_at: Timestamp,
    validate: impl FnOnce(&Graph) -> std::result::Result<(), String>,
) -> Result<Option<ManifestId>> {
    if ids.is_empty() {
        return Ok(None);
    }
    let Some(payload) = composed(store, None, ids).await? else {
        return Ok(None);
    };
    validate(&payload.clone().into_graph_with()?).map_err(Error::new)?;
    let provenance = ProvenanceRecord {
        origin: ProvenanceOrigin::Derived,
        upstream: ids.to_vec(),
        tooling: Some(COMPOSE_TOOLING.into()),
        generated_at: created_at,
    };
    save_composed(store, None, payload, provenance, created_at)
        .await
        .map(Some)
}

/// Compose several graph codicils, retaining surface identities and sharing
/// resource identities; `Ok(None)` if `ids` is empty or any is missing.
pub async fn compose_graph_codicils(
    store: &mut dyn Store,
    ids: &[ManifestId],
    redaction: RedactionPolicy,
    created_at: Timestamp,
) -> Result<Option<ManifestId>> {
    compose_graph_codicils_sealed(store, None, ids, redaction, created_at).await
}

/// As [`compose_graph_codicils`], unsealing sources and sealing the result.
pub async fn compose_graph_codicils_sealed(
    store: &mut dyn Store,
    sealer: Option<&dyn PayloadSealer>,
    ids: &[ManifestId],
    redaction: RedactionPolicy,
    created_at: Timestamp,
) -> Result<Option<ManifestId>> {
    if ids.is_empty() {
        return Ok(None);
    }
    let Some(mut codicil) = composed(store, sealer, ids).await? else {
        return Ok(None);
    };
    redact(&redaction, &mut codicil.codicil.snapshot);
    redact_facets(&redaction, &mut codicil.codicil.facets);
    if redaction != RedactionPolicy::include_all() {
        codicil.codicil.sessions.clear();
    }
    codicil.clone().into_graph_with()?;
    let provenance = ProvenanceRecord {
        origin: ProvenanceOrigin::Derived,
        upstream: ids.to_vec(),
        tooling: Some(COMPOSE_TOOLING.into()),
        generated_at: created_at,
    };
    save_composed(store, sealer, codicil, provenance, created_at)
        .await
        .map(Some)
}

fn merge_graph_codicils(
    a: GraphCodicil<Graph>,
    b: GraphCodicil<Graph>,
) -> Result<GraphCodicil<Graph>> {
    let (snapshot, _, remap) =
        crate::snapshot_merge::merge_snapshots_with_remap(&a.snapshot, &b.snapshot)?;
    let mut facets = a.facets;
    for (node, node_facets) in b.facets.iter() {
        let canonical = remap
            .get(&node.to_string())
            .and_then(|id| Uuid::parse_str(id).ok())
            .unwrap_or(*node);
        for (facet, value) in node_facets.iter() {
            if facets.get(&canonical, facet).is_none() {
                facets
                    .set(
                        canonical,
                        facet.clone(),
                        value.clone(),
                        &portolan::AcceptAll,
                    )
                    .expect("AcceptAll cannot reject a composed facet");
            }
        }
    }
    let mut sessions = a.sessions;
    sessions.extend(b.sessions);
    Ok(GraphCodicil {
        snapshot,
        facets,
        sessions,
    })
}
