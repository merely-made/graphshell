// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! # incipit-session
//!
//! The surface-reading half of a mere's sessions, over incipit-graph's
//! [`Graph`](kernel::graph::Graph) (engine split D256). pandect keeps the
//! session core, generic over any [`SessionGraph`](mere::session::SessionGraph);
//! what reads nodes, urls, tags, visits and the snapshot's columns is copied
//! here from pandect, where it was mere-kernel's (engine split D256, lane 4).
//!
//! - [`sessions`]: incipit's publication, legacy and codicil hooks, the one
//!   constructor that installs them, and the session methods pandect held
//!   only for mere-kernel's graph, as extension traits with the same names.
//! - the modules pandect's are copied from, under the same names:
//!   [`addressable_graph`], [`live_graph`], [`access`], [`memory_levels`],
//!   [`image_store`], [`snapshot_merge`], [`merge_rules`], [`graph_codicil`],
//!   the furnace and personal sync's fold.

pub mod access;
pub mod addressable_graph;
#[cfg(feature = "furnace")]
pub mod furnace;
pub mod graph_codicil;
pub mod image_store;
pub mod live_graph;
pub mod memory_levels;
pub mod merge_rules;
#[cfg(all(feature = "personal-sync", not(target_arch = "wasm32")))]
pub mod personal_sync;
pub mod sessions;
pub mod snapshot_merge;
#[cfg(all(feature = "personal-sync", not(target_arch = "wasm32")))]
pub mod transfer_offer;

mod fork;
mod graph_session;

#[cfg(not(target_arch = "wasm32"))]
pub use fork::fork_component_graph;
pub use sessions::{
    INCIPIT_CODICIL, INCIPIT_LEGACY, INCIPIT_PUBLICATION, IncipitSession, IncipitSessions,
};

// The copied modules name these from their crate root, as pandect's do.
pub(crate) use pandect::{Applied, SessionError};
