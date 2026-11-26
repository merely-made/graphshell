// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Address-access facts over incipit-graph. The records, their schema and
//! their Eidetic lane are pandect's and re-exported; the three reads and
//! writes of a node's access history are copied from pandect's `access.rs`
//! (Mere 36a13570b) over this crate's graph.

pub use pandect::access::*;

use eidetic::PrivacyClass;
use kernel::graph::apply::{GraphDelta, GraphDeltaResult, apply_graph_delta};
use kernel::graph::{Graph, NodeKey};
use portolan::FacetId;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyAccessRecord {
    persona: String,
    device: String,
    at_ms: u64,
    handler: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyAccessHistory {
    records: Vec<LegacyAccessRecord>,
}

/// Read one node's access history. An absent facet is an empty history.
pub fn access_history(graph: &Graph, key: NodeKey) -> Result<AccessHistory, AccessError> {
    let node = graph.get_node(key).ok_or(AccessError::UnknownNode)?;
    let Some(value) = graph
        .facets()
        .get(&node.id, &FacetId::new(ACCESS_HISTORY_FACET))
    else {
        return Ok(AccessHistory::default());
    };
    if let Ok(history) = serde_json::from_value::<AccessHistory>(value.clone()) {
        return Ok(history);
    }
    let legacy: LegacyAccessHistory =
        serde_json::from_value(value.clone()).map_err(AccessError::InvalidFacet)?;
    Ok(AccessHistory {
        records: legacy
            .records
            .into_iter()
            .enumerate()
            .map(|(index, legacy)| {
                let identity = format!(
                    "{}\u{0}{}\u{0}{}\u{0}{}\u{0}{}",
                    node.id, legacy.persona, legacy.device, legacy.at_ms, index
                );
                AccessRecord {
                    record_id: Uuid::new_v5(&Uuid::NAMESPACE_URL, identity.as_bytes()),
                    container_id: node.id,
                    address: node.url().to_string(),
                    action: AccessAction::Open,
                    persona: legacy.persona,
                    device: legacy.device,
                    application: legacy.handler.clone(),
                    at_ms: legacy.at_ms,
                    handler: legacy.handler,
                    dwell_ms: None,
                    referring_container_id: None,
                    referring_address: None,
                    transition: AccessTransition::Unknown,
                    capture_source: "graphshell.legacy-facet".to_string(),
                    source_event_id: None,
                    privacy: PrivacyClass::LocalOnly,
                }
            })
            .collect(),
    })
}

/// Append an access through the host extension gate.
pub fn record_access(
    graph: &mut Graph,
    key: NodeKey,
    context: &AccessContext,
    handler: &str,
) -> Result<(), AccessError> {
    record_observation(
        graph,
        key,
        &AccessObservation {
            record_id: Uuid::new_v4(),
            action: AccessAction::Open,
            persona: context.persona.clone(),
            device: context.device.clone(),
            application: handler.to_string(),
            handler: handler.to_string(),
            at_ms: context.at_ms,
            dwell_ms: None,
            referring_container_id: None,
            referring_address: None,
            transition: AccessTransition::Unknown,
            capture_source: "graphshell.intent".to_string(),
            source_event_id: None,
            privacy: PrivacyClass::LocalOnly,
        },
    )
    .map(|_| ())
}

/// Append one complete observation to the derived node cache. Duplicate stable
/// record ids are idempotent. The caller separately saves the same record as
/// the append-only Eidetic authority.
pub fn record_observation(
    graph: &mut Graph,
    key: NodeKey,
    observation: &AccessObservation,
) -> Result<(AccessRecord, bool), AccessError> {
    let node_id = graph
        .get_node(key)
        .map(|node| node.id)
        .ok_or(AccessError::UnknownNode)?;
    let address = graph
        .get_node(key)
        .map(|node| node.url().to_string())
        .ok_or(AccessError::UnknownNode)?;
    let mut history = access_history(graph, key)?;
    let record = AccessRecord {
        record_id: observation.record_id,
        container_id: node_id,
        address,
        action: observation.action,
        persona: observation.persona.clone(),
        device: observation.device.clone(),
        application: observation.application.clone(),
        at_ms: observation.at_ms,
        handler: observation.handler.clone(),
        dwell_ms: observation.dwell_ms,
        referring_container_id: observation.referring_container_id,
        referring_address: observation.referring_address.clone(),
        transition: observation.transition,
        capture_source: observation.capture_source.clone(),
        source_event_id: observation.source_event_id.clone(),
        privacy: observation.privacy,
    };
    if history
        .records
        .iter()
        .any(|existing| existing.record_id == record.record_id)
    {
        return Ok((record, false));
    }
    history.records.push(record.clone());
    let value = serde_json::to_value(history).expect("AccessHistory always serializes");
    let updated = apply_graph_delta(
        graph,
        GraphDelta::SetNodeFacet {
            key,
            facet: ACCESS_HISTORY_FACET.to_string(),
            value,
        },
    );
    debug_assert_eq!(updated, GraphDeltaResult::NodeMetadataUpdated(true));
    Ok((record, true))
}
