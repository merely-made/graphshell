// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! pandect's transfer-offer projection tests (Mere 36a13570b,
//! `transfer_offer.rs`) over incipit-graph: each projection folds through
//! `IncipitFold` and `offers_in` reads incipit's graph. The three that never
//! project stay pandect's.

#![cfg(all(feature = "personal-sync", not(target_arch = "wasm32")))]

use eidetic::Hash;
use incipit_session::personal_sync::{
    IncipitFold, PersonalGraphReplica, SyncRoster, SyncSelection,
};
use incipit_session::transfer_offer::*;
use muniment::MemoryBackend;
use p2panda_core::SigningKey;
use pandect::FacetId;
use pandect::transfer::{TransferEndpointV1, TransferOperation};
use uuid::Uuid;

const GRAPH: [u8; 32] = [0x5c; 32];
const SOURCE_SEED: [u8; 32] = [0x11; 32];
const OFFER: Uuid = Uuid::from_u128(0x0ffe);

fn endpoint(device: &str) -> TransferEndpointV1 {
    TransferEndpointV1 {
        graph: "graphshell://graph/personal".to_string(),
        persona: "personae://persona/alice".to_string(),
        device: format!("personae://device/{device}"),
    }
}

fn offer() -> TransferOfferV1 {
    TransferOfferV1 {
        schema: TRANSFER_OFFER_FACET.to_string(),
        transfer_id: OFFER,
        operation: TransferOperation::Copy,
        source: endpoint("laptop"),
        destination: endpoint("phone"),
        pairing_id: "pairing-1".to_string(),
        manifest_blob: Hash::of(b"manifest"),
        manifest_byte_len: 4096,
        nodes: 2,
        relations: 1,
        blobs: 1,
        blob_bytes: 90_000,
        offered_at_ms: 1_700_000_000_000,
    }
}

/// Every device in these tests shares one roster and one store contents.
/// Only the selection differs, which is the point: filtering is local.
fn selection(local_device: &str, offers_enabled: bool) -> SyncSelection {
    let facets: Vec<String> = if offers_enabled {
        vec![TRANSFER_OFFER_FACET.to_string()]
    } else {
        Vec::new()
    };
    SyncSelection::default()
        .with_facets(facets)
        .with_synthetic_addresses([transfer_offer_rule()])
        .with_local_device(local_device)
}

fn replica(selection: SyncSelection) -> PersonalGraphReplica<MemoryBackend> {
    let subject = *SigningKey::from_bytes(&SOURCE_SEED)
        .verifying_key()
        .as_bytes();
    PersonalGraphReplica::new(
        MemoryBackend::new(),
        GRAPH,
        SOURCE_SEED,
        SyncRoster::new([subject]),
        selection,
    )
}

#[tokio::test]
async fn an_offer_reaches_its_destination_and_its_source_but_not_a_bystander() {
    let mut source = replica(selection("laptop", true));
    let operation = source
        .author(offer_events(&offer()).unwrap())
        .await
        .unwrap();

    let destination = replica(selection("phone", true));
    let bystander = replica(selection("tablet", true));
    for device in [&destination, &bystander] {
        assert!(
            device.accept(&operation).await.unwrap(),
            "every admitted device stores the operation; only projection differs"
        );
    }

    let addressed = offers_in(&destination.projection_with(&IncipitFold).await.unwrap());
    assert_eq!(addressed.len(), 1);
    assert_eq!(addressed[0], offer());

    assert_eq!(
        offers_in(&source.projection_with(&IncipitFold).await.unwrap()).len(),
        1,
        "a source projects the offers it sent"
    );
    let seen = bystander.projection_with(&IncipitFold).await.unwrap();
    assert!(offers_in(&seen).is_empty());
    assert!(
        seen.graph.get_node_by_id(OFFER).is_none(),
        "the carrier node is filtered with its facet, not left titled and empty"
    );
}

#[tokio::test]
async fn a_device_without_the_offer_facet_projects_neither_carrier_nor_facet() {
    let mut source = replica(selection("laptop", true));
    let operation = source
        .author(offer_events(&offer()).unwrap())
        .await
        .unwrap();

    let opted_out = replica(selection("phone", false));
    opted_out.accept(&operation).await.unwrap();
    let projection = opted_out.projection_with(&IncipitFold).await.unwrap();

    assert!(offers_in(&projection).is_empty());
    assert!(projection.graph.get_node_by_id(OFFER).is_none());
    assert_eq!(
        projection
            .graph
            .facets()
            .get(&OFFER, &FacetId::new(TRANSFER_OFFER_FACET)),
        None,
        "a dropped carrier takes its facet with it, leaving no orphan"
    );
}

#[tokio::test]
async fn a_device_that_does_not_know_its_own_key_projects_every_offer() {
    let mut source = replica(selection("laptop", true));
    let operation = source
        .author(offer_events(&offer()).unwrap())
        .await
        .unwrap();

    let unconfigured = replica(
        SyncSelection::default()
            .with_facets([TRANSFER_OFFER_FACET])
            .with_synthetic_addresses([transfer_offer_rule()]),
    );
    unconfigured.accept(&operation).await.unwrap();
    assert_eq!(
        offers_in(&unconfigured.projection_with(&IncipitFold).await.unwrap()).len(),
        1,
        "over-showing is the honest failure; silently hiding offers is not"
    );
}
