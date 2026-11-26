// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Transfer offers over the personal graph. The offers and their events are
//! pandect's and re-exported; [`offers_in`] reads them from a projection
//! into incipit-graph, copied from pandect's `transfer_offer.rs` (Mere
//! 36a13570b).

pub use pandect::transfer_offer::*;

use kernel::graph::Graph;
use pandect::personal_sync::SyncProjection;
use portolan::FacetId;

/// The offers this device projects, oldest first.
pub fn offers_in(projection: &SyncProjection<Graph>) -> Vec<TransferOfferV1> {
    let facet = FacetId::new(TRANSFER_OFFER_FACET);
    let mut offers: Vec<TransferOfferV1> = projection
        .graph
        .nodes()
        .filter(|(_, node)| {
            node.primary_address()
                .as_url_str()
                .starts_with(TRANSFER_ADDRESS_PREFIX)
        })
        .filter_map(|(_, node)| projection.graph.facets().get(&node.id, &facet))
        .filter_map(|value| serde_json::from_value::<TransferOfferV1>(value.clone()).ok())
        .filter(|offer| offer.schema == TRANSFER_OFFER_FACET)
        .collect();
    offers.sort_by_key(|offer| (offer.offered_at_ms, offer.transfer_id));
    offers
}
