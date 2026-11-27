// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Transfer offers over the personal graph. The offers and their events are
//! pandect's and re-exported; [`offers_in`] reads them from a projection
//! into incipit-graph, and [`offer_for`] summarizes a manifest's selection
//! as incipit-graph decodes it, both copied from pandect's
//! `transfer_offer.rs` (Mere 36a13570b and 2c1199652).

pub use pandect::transfer_offer::*;

use eidetic::Hash;
use kernel::graph::Graph;
use pandect::personal_sync::SyncProjection;
use pandect::product_codicil::decode_profiled_codicil_with;
use pandect::transfer::TransferManifestV1;
use portolan::FacetId;

/// Summarize a prepared manifest, given where its bytes were staged.
pub fn offer_for(
    manifest: &TransferManifestV1,
    manifest_blob: Hash,
    manifest_byte_len: u64,
    pairing_id: impl Into<String>,
    offered_at_ms: u64,
) -> Result<TransferOfferV1, OfferError> {
    let product = decode_profiled_codicil_with::<Graph>(&manifest.selection.payload)
        .map_err(|error| OfferError::Selection(error.to_string()))?;
    Ok(TransferOfferV1 {
        schema: TRANSFER_OFFER_FACET.to_string(),
        transfer_id: manifest.transfer_id,
        operation: manifest.operation,
        source: manifest.source.clone(),
        destination: manifest.destination.clone(),
        pairing_id: pairing_id.into(),
        manifest_blob,
        manifest_byte_len,
        nodes: product.graph.nodes.len() as u64,
        relations: product.graph.edges.len() as u64,
        blobs: manifest.blobs.len() as u64,
        blob_bytes: manifest.blobs.iter().map(|blob| blob.byte_len).sum(),
        offered_at_ms,
    })
}

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
