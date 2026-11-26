// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The two pandect session names the copied modules reach for under
//! `crate::graph_session`, so they read as they did in pandect.

pub(crate) use pandect::graph_session::LegacyTransactionBackend;

use mere::session::SessionGraph;
use pandect::SessionError;

/// Apply one edit, naming its position when refused. A copy of pandect's
/// `graph_session_history.rs` `apply_one` (Mere 36a13570b), which is
/// crate-private there.
pub(crate) fn apply_one<G: SessionGraph>(
    graph: &mut G,
    position: usize,
    delta: G::Edit,
) -> Result<(), SessionError> {
    graph
        .apply(delta)
        .map_err(|reason| SessionError::Validation(format!("edit {position} refused: {reason}")))
}
