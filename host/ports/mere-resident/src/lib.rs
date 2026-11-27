// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Mere's concrete resident composition, served by independent Djinn.
//!
//! This package constructs Knot, graph/reservoir, Distillery and published-site
//! services and supplies their authorities and shutdown order. Djinn owns the
//! application door and resident task lifetime. Custodian remains a separately
//! protected authority using custody-wire. Executable names remain `djinn`.

#![doc(html_no_source)]


// pandect's graph-generic types over this crate's graph, one alias each (engine split D244).
pub(crate) type DroppedChoice = pandect::DroppedChoice<kernel::graph::Graph>;
pub(crate) type GraphAdmissionValidator = pandect::GraphAdmissionValidator<kernel::graph::Graph>;
pub(crate) type GraphCandidate = pandect::graph_codicil::GraphCandidate<kernel::graph::Graph>;
pub(crate) type GraphValidator = pandect::GraphValidator<kernel::graph::Graph>;
pub(crate) type MereSessions<B> = pandect::MereSessions<B, kernel::graph::Graph>;
pub(crate) type PendingMerge = pandect::PendingMerge<kernel::graph::Graph>;
#[cfg(test)]
pub(crate) type ProductCodicilV3 = pandect::product_codicil::ProductCodicilV3<kernel::graph::Graph>;
#[cfg(test)]
pub(crate) type ProfiledProductCodicil =
    pandect::product_codicil::ProfiledProductCodicil<kernel::graph::Graph>;
#[cfg(test)]
pub(crate) type SyncProjection = pandect::personal_sync::SyncProjection<kernel::graph::Graph>;

pub mod applications;
pub mod authority_guest;
pub mod conditions;
pub mod custody;
pub mod domain_registry;
pub mod door;
pub mod embedded_reservoir;
pub mod identity_ui;
pub mod lock_triggers;
pub mod pairing;
pub mod personal_sync;
pub mod personal_facts;
pub mod resident;
pub mod resident_blob_door;
pub mod resident_blobs;
pub mod resident_devices;
pub mod resident_distillery;
pub mod resident_events;
pub mod resident_inference;
pub mod resident_mere;
pub mod resident_projection;
pub mod resident_reservoir;
pub mod retained_custody;
// Session worker declarations and their runner, from pandect (engine split D43).
pub mod session_workers;
pub mod supervisor;
pub mod resident_site;
#[cfg(feature = "moot-site")]
pub mod moot_site;
pub mod resident_status;
pub mod persona_settings_store;
pub mod settings;
pub mod settings_store;
pub mod sync;
