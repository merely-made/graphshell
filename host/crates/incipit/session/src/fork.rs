// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! A component fork's graph over incipit-graph, copied from pandect's
//! `graph_session.rs` `fork_component_graph` (Mere 36a13570b).

#[cfg(not(target_arch = "wasm32"))]
use kernel::graph::Graph;
#[cfg(not(target_arch = "wasm32"))]
use kernel::graph::node_facets::{PROVENANCE_DERIVATIONS, PROVENANCE_IMPORT};
#[cfg(not(target_arch = "wasm32"))]
use pandect::{AcceptAll, FacetId, NodeFacetStore, copy_node_facets};
#[cfg(not(target_arch = "wasm32"))]
use uuid::Uuid;

/// The graph a component fork starts from: the connected component around
/// `seed`, copied with its facets through the id remap, and every facet of the
/// donor container carried to the fork's container.
///
/// The container is one more remap pair, `(donor_container, fork_container)`,
/// so its facets travel verbatim and this crate names no namespace (engine
/// split D134). The values are the donor's exactly: a facet that names members
/// (a saved scene's selection, arrangement regions, a mosaic) still names the
/// donor's ids, including nodes outside the component, which the fork does not
/// hold. Those ids are inert references; the host that opens the fork reads
/// them against the fork's own nodes and may rewrite them. A donor container
/// with no facets leaves the fork's with none, which reads as the defaults.
///
/// Audited 2026-11-12 (engine split D134): the facets any writer puts on a
/// container id today are all canvas view state, none of which a fork must not
/// inherit: `scene.size_by_degree`, `scene.size_by_importance`,
/// `scene.importance_metric`, `scene.physics_damping` (Turnstone's older
/// scenes), `graphshell.saved-scene/v4` (and the read-only `/v3`, `/v2`),
/// `arrangement.regions` and `arrangement.mosaic`. No sharing or co-op
/// enablement, sync or replication state, identity or denizen binding is keyed
/// by a container; those ride per node, or beside the graph. A new container
/// namespace that is not view state needs this carry revisited.
///
/// The product-neutral half of Turnstone's tear-out; nested worlds and resident
/// admissions stay Turnstone's to carry. `None` when `seed` names no node;
/// refused when the carried facets fail the handle rule. Native only, as the
/// kernel's component copy is.
#[cfg(not(target_arch = "wasm32"))]
pub fn fork_component_graph(
    donor: &Graph,
    donor_container: Uuid,
    seed: Uuid,
    fork_container: Uuid,
) -> Result<Option<Graph>, kernel::graph::snapshot::ResourceSnapshotError> {
    let mut fork = Graph::new();
    let copy = fork.copy_component_from(donor, seed, Some(donor_container.to_string()));
    if copy.new_keys.is_empty() {
        return Ok(None);
    }
    let mut facets = NodeFacetStore::new();
    let mut remap = copy.id_remap.clone();
    remap.push((donor_container, fork_container));
    copy_node_facets(donor.facets(), &mut facets, &remap);
    // The copy stamped each minted node's derivation from its donor; the
    // overlay below would drop it, so keep it aside and set it again.
    let derivation = FacetId::new(PROVENANCE_DERIVATIONS);
    let derivations: Vec<(Uuid, serde_json::Value)> = copy
        .id_remap
        .iter()
        .filter_map(|(_, minted)| {
            fork.facets()
                .get(minted, &derivation)
                .cloned()
                .map(|value| (*minted, value))
        })
        .collect();
    let mut merged = fork.facets().clone();
    merged.overlay(facets);
    let import = FacetId::new(PROVENANCE_IMPORT);
    for (_, minted) in &copy.id_remap {
        merged.remove(minted, &import);
    }
    for (minted, value) in derivations {
        merged
            .set(minted, derivation.clone(), value, &AcceptAll)
            .expect("AcceptAll admits every value");
    }
    fork.install_facets(merged)?;
    Ok(Some(fork))
}
