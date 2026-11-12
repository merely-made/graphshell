// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The `arrangement.*` facet namespace — cartography's per-node data as facets.
//!
//! The facet-convergence ruling (design doc
//! `2026-07-18_node_dissolution_facets_plan.md`, "Facet convergence of the
//! per-node sidecars"): cartography's per-node arrangement data (position,
//! size, sprite, sprite-hull, material, face) is typed metadata keyed by node
//! id, which is the facet store's definition. It persists as `arrangement.*`
//! facets in [`facets.json`](pandect::facet_store), not as a bespoke
//! `cartography.json` — and since no host ever wired that bespoke sidecar,
//! the durable layout store is **born** as facets; there is nothing to migrate.
//!
//! This module defines the namespace's facet ids + payload shapes and the
//! read/write helpers a host uses around the canvas seams:
//!
//! - save: `Canvas::cartography_geometry().iter()` →
//!   [`write_arrangement_positions`] → `save_node_facets`.
//! - load: `load_node_facets` → [`read_arrangement_positions`] →
//!   `Canvas::seed_cartography`.
//!
//! Boundaries (per the ruling): the *live* position lives in seiche and is
//! never a facet — only the durable save-time position lands here, so the
//! store holds cold data and the hot loop stays out of JSON. Graph-scoped
//! canvas flags (`size_by_degree`, `importance_metric`, …) are view settings,
//! not per-node facets; they belong to the view-intent/settings stores.
//!
//! The family: `arrangement.position` (first wired), then size / sprite /
//! sprite-hull / material / face / seed / regions / mosaic — one facet id +
//! payload shape + write/read pair each, all with the same
//! rewrite-clears-stale save semantics. Seed is per-node like the rest;
//! regions and mosaic describe the whole arrangement, so they are facets of
//! the scene's container node (as `scene.*` are), keyed by the caller's
//! container id. The
//! graph-scoped canvas flags the bespoke geometry carried (`size_by_degree`,
//! `size_by_importance`, `importance_metric`) are deliberately NOT here; they
//! await their view-settings home.

use std::collections::BTreeSet;

use serde_json::{Value, json};
use uuid::Uuid;

use pandect::{AcceptAll, CartographyGeometry, FacetId, NodeFacetStore, SavedMosaic, SavedRegion};

/// Facet id of a node's durable world position. Payload: `{"x": f32, "y": f32}`
/// (world coordinates, the semantic geometry — not pixels).
pub const ARRANGEMENT_POSITION: &str = "arrangement.position";

/// Facet id of a node's explicit face-size override. Payload: a number (px).
/// Only deliberate overrides persist; degree/importance-derived sizes recompute.
pub const ARRANGEMENT_SIZE: &str = "arrangement.size";

/// Facet id of a node's custom sprite face. Payload: a string (PNG data-URI).
pub const ARRANGEMENT_SPRITE: &str = "arrangement.sprite";

/// Facet id of a sprite's collider hull. Payload: `[[x, y], …]` — a
/// face-normalized convex polygon (coordinates in `[-0.5, 0.5]`), persisted
/// beside the sprite so the traced collider survives without re-decoding.
pub const ARRANGEMENT_SPRITE_HULL: &str = "arrangement.sprite_hull";

/// Facet id of a node's physical material override. Payload:
/// `{"restitution": f32, "friction": f32, "density": f32}`.
pub const ARRANGEMENT_MATERIAL: &str = "arrangement.material";

/// Facet id of a node's face override. Payload: a string code
/// (`favicon` / `sprite` / `bare` — the canvas's `Face::from_code` vocabulary).
pub const ARRANGEMENT_FACE: &str = "arrangement.face";

/// Facet id of a node's place in the graph grammar's saved seed. Payload:
/// `{"x": f32, "y": f32}`. No node carrying it means the recipe seed.
pub const ARRANGEMENT_SEED: &str = "arrangement.seed";

/// Facet id of the regions the active arrangement drew. Payload: an array of
/// [`SavedRegion`]. Carried by the scene's container node.
pub const ARRANGEMENT_REGIONS: &str = "arrangement.regions";

/// Facet id of the person's Mosaic arrangement. Payload: an
/// [`SavedMosaic`]. Carried by the scene's container node.
pub const ARRANGEMENT_MOSAIC: &str = "arrangement.mosaic";

/// The [`ARRANGEMENT_POSITION`] facet id, constructed.
pub fn arrangement_position_facet() -> FacetId {
    FacetId::new(ARRANGEMENT_POSITION)
}

/// Replace every facet carrying `facet` with the given `(node, payload)` set —
/// the shared save-time semantics of the whole family: existing facets of this
/// id are cleared first, so a node absent from the new set carries none
/// afterwards; facets in other namespaces (and other family members) are
/// untouched. Payloads go through [`AcceptAll`]: the shapes are fixed by this
/// module, so schema validation adds nothing.
fn rewrite_family(store: &mut NodeFacetStore, facet: FacetId, entries: Vec<(Uuid, Value)>) {
    let stale: Vec<Uuid> = store
        .iter()
        .filter(|(_, facets)| facets.has(&facet))
        .map(|(id, _)| *id)
        .collect();
    for id in stale {
        store.remove(&id, &facet);
    }
    for (id, payload) in entries {
        let _ = store.set(id, facet.clone(), payload, &AcceptAll);
    }
}

/// The shared load-time read: every node carrying `facet`, its payload parsed
/// by `parse`. A malformed payload is skipped rather than failing the load:
/// one bad facet must not cost the whole session its layout.
fn read_family<T>(
    store: &NodeFacetStore,
    facet: &str,
    parse: impl Fn(&Value) -> Option<T>,
) -> Vec<(Uuid, T)> {
    let facet = FacetId::new(facet);
    store
        .iter()
        .filter_map(|(id, facets)| parse(facets.get(&facet)?).map(|t| (*id, t)))
        .collect()
}

/// Save-time write of [`ARRANGEMENT_POSITION`] (rewrite-clears-stale).
pub fn write_arrangement_positions(
    store: &mut NodeFacetStore,
    positions: impl IntoIterator<Item = (Uuid, (f32, f32))>,
) {
    rewrite_family(
        store,
        arrangement_position_facet(),
        positions
            .into_iter()
            .map(|(id, (x, y))| (id, json!({ "x": x, "y": y })))
            .collect(),
    );
}

/// Load-time read of [`ARRANGEMENT_POSITION`], shaped for
/// `Canvas::seed_cartography`.
pub fn read_arrangement_positions(store: &NodeFacetStore) -> Vec<(Uuid, (f32, f32))> {
    read_family(store, ARRANGEMENT_POSITION, |value| {
        let x = value.get("x")?.as_f64()? as f32;
        let y = value.get("y")?.as_f64()? as f32;
        (x.is_finite() && y.is_finite()).then_some((x, y))
    })
}

/// Save-time write of [`ARRANGEMENT_SIZE`] (rewrite-clears-stale).
pub fn write_arrangement_sizes(
    store: &mut NodeFacetStore,
    sizes: impl IntoIterator<Item = (Uuid, f32)>,
) {
    rewrite_family(
        store,
        FacetId::new(ARRANGEMENT_SIZE),
        sizes.into_iter().map(|(id, px)| (id, json!(px))).collect(),
    );
}

/// Load-time read of [`ARRANGEMENT_SIZE`], shaped for
/// `Canvas::apply_cartography_sizing` (which clamps).
pub fn read_arrangement_sizes(store: &NodeFacetStore) -> Vec<(Uuid, f32)> {
    read_family(store, ARRANGEMENT_SIZE, |value| {
        let px = value.as_f64()? as f32;
        px.is_finite().then_some(px)
    })
}

/// Save-time write of [`ARRANGEMENT_SPRITE`] (rewrite-clears-stale).
pub fn write_arrangement_sprites<S: AsRef<str>>(
    store: &mut NodeFacetStore,
    sprites: impl IntoIterator<Item = (Uuid, S)>,
) {
    rewrite_family(
        store,
        FacetId::new(ARRANGEMENT_SPRITE),
        sprites
            .into_iter()
            .map(|(id, uri)| (id, json!(uri.as_ref())))
            .collect(),
    );
}

/// Load-time read of [`ARRANGEMENT_SPRITE`] (owned data-URIs; the host lends
/// them to `Canvas::apply_cartography_sprites`).
pub fn read_arrangement_sprites(store: &NodeFacetStore) -> Vec<(Uuid, String)> {
    read_family(store, ARRANGEMENT_SPRITE, |value| {
        Some(value.as_str()?.to_string())
    })
}

/// Save-time write of [`ARRANGEMENT_SPRITE_HULL`] (rewrite-clears-stale).
pub fn write_arrangement_sprite_hulls(
    store: &mut NodeFacetStore,
    hulls: impl IntoIterator<Item = (Uuid, Vec<(f32, f32)>)>,
) {
    rewrite_family(
        store,
        FacetId::new(ARRANGEMENT_SPRITE_HULL),
        hulls
            .into_iter()
            .map(|(id, hull)| {
                let points: Vec<Value> = hull.iter().map(|(x, y)| json!([x, y])).collect();
                (id, Value::Array(points))
            })
            .collect(),
    );
}

/// Load-time read of [`ARRANGEMENT_SPRITE_HULL`], shaped for
/// `Canvas::apply_cartography_sprite_hulls`. A hull with any malformed or
/// non-finite point is skipped whole (a partial polygon is a wrong collider).
pub fn read_arrangement_sprite_hulls(store: &NodeFacetStore) -> Vec<(Uuid, Vec<(f32, f32)>)> {
    read_family(store, ARRANGEMENT_SPRITE_HULL, |value| {
        value
            .as_array()?
            .iter()
            .map(|point| {
                let pair = point.as_array()?;
                let x = pair.first()?.as_f64()? as f32;
                let y = pair.get(1)?.as_f64()? as f32;
                (x.is_finite() && y.is_finite()).then_some((x, y))
            })
            .collect::<Option<Vec<(f32, f32)>>>()
    })
}

/// Save-time write of [`ARRANGEMENT_MATERIAL`] (rewrite-clears-stale). The
/// tuple is `(restitution, friction, density)`, the geometry sidecar's order.
pub fn write_arrangement_materials(
    store: &mut NodeFacetStore,
    materials: impl IntoIterator<Item = (Uuid, (f32, f32, f32))>,
) {
    rewrite_family(
        store,
        FacetId::new(ARRANGEMENT_MATERIAL),
        materials
            .into_iter()
            .map(|(id, (restitution, friction, density))| {
                (
                    id,
                    json!({
                        "restitution": restitution,
                        "friction": friction,
                        "density": density,
                    }),
                )
            })
            .collect(),
    );
}

/// Load-time read of [`ARRANGEMENT_MATERIAL`] as
/// `(restitution, friction, density)`, shaped for
/// `Canvas::apply_cartography_materials`.
pub fn read_arrangement_materials(store: &NodeFacetStore) -> Vec<(Uuid, (f32, f32, f32))> {
    read_family(store, ARRANGEMENT_MATERIAL, |value| {
        let restitution = value.get("restitution")?.as_f64()? as f32;
        let friction = value.get("friction")?.as_f64()? as f32;
        let density = value.get("density")?.as_f64()? as f32;
        (restitution.is_finite() && friction.is_finite() && density.is_finite()).then_some((
            restitution,
            friction,
            density,
        ))
    })
}

/// Save-time write of [`ARRANGEMENT_FACE`] (rewrite-clears-stale).
pub fn write_arrangement_faces<S: AsRef<str>>(
    store: &mut NodeFacetStore,
    faces: impl IntoIterator<Item = (Uuid, S)>,
) {
    rewrite_family(
        store,
        FacetId::new(ARRANGEMENT_FACE),
        faces
            .into_iter()
            .map(|(id, code)| (id, json!(code.as_ref())))
            .collect(),
    );
}

/// Load-time read of [`ARRANGEMENT_FACE`] (owned codes; the host lends them to
/// `Canvas::apply_cartography_faces`, whose `Face::from_code` owns the
/// vocabulary — an unknown code degrades there, not here).
pub fn read_arrangement_faces(store: &NodeFacetStore) -> Vec<(Uuid, String)> {
    read_family(store, ARRANGEMENT_FACE, |value| {
        Some(value.as_str()?.to_string())
    })
}

/// Save-time write of [`ARRANGEMENT_SEED`] (rewrite-clears-stale). `None` is
/// the recipe seed and writes nothing; an empty `Some` reads back as `None`.
pub fn write_arrangement_seed(
    store: &mut NodeFacetStore,
    seed: Option<impl IntoIterator<Item = (Uuid, (f32, f32))>>,
) {
    rewrite_family(
        store,
        FacetId::new(ARRANGEMENT_SEED),
        seed.into_iter()
            .flatten()
            .map(|(id, (x, y))| (id, json!({ "x": x, "y": y })))
            .collect(),
    );
}

/// Load-time read of [`ARRANGEMENT_SEED`], shaped for
/// `CartographyGeometry::with_seed`: `None` when no node carries a seed place.
pub fn read_arrangement_seed(store: &NodeFacetStore) -> Option<Vec<(Uuid, (f32, f32))>> {
    let seed = read_family(store, ARRANGEMENT_SEED, |value| {
        let x = value.get("x")?.as_f64()? as f32;
        let y = value.get("y")?.as_f64()? as f32;
        (x.is_finite() && y.is_finite()).then_some((x, y))
    });
    (!seed.is_empty()).then_some(seed)
}

/// Save-time write of [`ARRANGEMENT_REGIONS`] on `container` (rewrite-clears-stale;
/// an empty set leaves no facet).
pub fn write_arrangement_regions(
    store: &mut NodeFacetStore,
    container: Uuid,
    regions: &[SavedRegion],
) {
    let entries = match regions {
        [] => Vec::new(),
        regions => vec![(container, json!(regions))],
    };
    rewrite_family(store, FacetId::new(ARRANGEMENT_REGIONS), entries);
}

/// Load-time read of [`ARRANGEMENT_REGIONS`] from `container`, in drawn order.
/// A malformed payload or any non-finite contour point yields no regions: a
/// partial set is a wrong picture of the arrangement.
pub fn read_arrangement_regions(store: &NodeFacetStore, container: Uuid) -> Vec<SavedRegion> {
    let regions: Option<Vec<SavedRegion>> = store
        .get(&container, &FacetId::new(ARRANGEMENT_REGIONS))
        .and_then(|value| serde_json::from_value(value.clone()).ok());
    regions
        .filter(|regions| {
            regions.iter().all(|r| {
                r.contour
                    .iter()
                    .all(|(x, y)| x.is_finite() && y.is_finite())
            })
        })
        .unwrap_or_default()
}

/// Save-time write of [`ARRANGEMENT_MOSAIC`] on `container`
/// (rewrite-clears-stale; `None` leaves no facet).
pub fn write_arrangement_mosaic(
    store: &mut NodeFacetStore,
    container: Uuid,
    mosaic: Option<&SavedMosaic>,
) {
    rewrite_family(
        store,
        FacetId::new(ARRANGEMENT_MOSAIC),
        mosaic
            .map(|mosaic| (container, json!(mosaic)))
            .into_iter()
            .collect(),
    );
}

/// Load-time read of [`ARRANGEMENT_MOSAIC`] from `container`. A malformed
/// payload or a non-finite scale yields `None` (the default arrangement).
pub fn read_arrangement_mosaic(
    store: &NodeFacetStore,
    container: Uuid,
) -> Option<SavedMosaic> {
    let mosaic: SavedMosaic = serde_json::from_value(
        store
            .get(&container, &FacetId::new(ARRANGEMENT_MOSAIC))?
            .clone(),
    )
    .ok()?;
    mosaic
        .scale
        .iter()
        .all(|(_, factor)| factor.is_finite())
        .then_some(mosaic)
}

/// The whole family as one [`CartographyGeometry`]: how a session saved
/// before the saved scene (`saved_scene`) keeps its layout, read once to be
/// carried into one. The container-keyed members (regions, mosaic) read from
/// `container` when there is one.
pub fn read_arrangement_geometry(
    store: &NodeFacetStore,
    container: Option<Uuid>,
) -> CartographyGeometry {
    CartographyGeometry::from_positions(read_arrangement_positions(store))
        .with_sizes(read_arrangement_sizes(store))
        .with_sprites(read_arrangement_sprites(store))
        .with_sprite_hulls(read_arrangement_sprite_hulls(store))
        .with_materials(read_arrangement_materials(store))
        .with_faces(read_arrangement_faces(store))
        .with_seed(read_arrangement_seed(store))
        .with_regions(
            container
                .map(|container| read_arrangement_regions(store, container))
                .unwrap_or_default(),
        )
        .with_mosaic(container.and_then(|container| read_arrangement_mosaic(store, container)))
}

/// Remove every `arrangement.*` facet the family writes, once a saved scene
/// holds what they did.
pub fn clear_arrangement(store: &mut NodeFacetStore, container: Option<Uuid>) {
    write_arrangement_positions(store, []);
    write_arrangement_sizes(store, []);
    write_arrangement_sprites(store, Vec::<(Uuid, String)>::new());
    write_arrangement_sprite_hulls(store, []);
    write_arrangement_materials(store, []);
    write_arrangement_faces(store, Vec::<(Uuid, String)>::new());
    write_arrangement_seed(store, None::<Vec<(Uuid, (f32, f32))>>);
    if let Some(container) = container {
        write_arrangement_regions(store, container, &[]);
        write_arrangement_mosaic(store, container, None);
    }
}

/// Drop every facet of every node not in `present` — the load-time reconcile,
/// mirroring the graph-membership retain the bespoke geometry sidecar did. A
/// node deleted between sessions takes its whole facet record with it (all
/// namespaces: its facets describe a node that no longer exists); facets of
/// live nodes — including foreign/mod namespaces — are untouched.
///
/// `present` is every id whose facets should survive — which is the live graph
/// members **plus any container id** carrying `scene.*` facets (a container is
/// not a leaf node, so a caller keying scene facets by `root_graph_id` must
/// include it here, or the reconcile would sweep the whole scene away).
pub fn retain_present_nodes(store: &mut NodeFacetStore, present: &BTreeSet<Uuid>) {
    let departed: Vec<Uuid> = store
        .iter()
        .map(|(id, _)| *id)
        .filter(|id| !present.contains(id))
        .collect();
    for id in departed {
        store.remove_node(&id);
    }
}

#[cfg(test)]
mod tests;
