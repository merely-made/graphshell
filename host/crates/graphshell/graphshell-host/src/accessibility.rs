// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Host envelopes for the client accessibility view of a mounted scene.

use chirograph::{CapabilityProfile, ProjectionSession};
use graphshell_client::{AccessibilityTree, ClientState, LocalActions, ResolutionError};
use sceno::SourceRef;
use scenotime::Revision;
use uxtree::envelope::{Address, Envelope};

pub type SceneItemEnvelope = Envelope<ProjectionSession, Revision, SourceRef>;

/// A client tree and envelopes in the same reading order as its visible items.
pub struct SceneAccessibility {
    pub tree: AccessibilityTree,
    pub items: Vec<SceneItemEnvelope>,
}

/// Take the tree and its scope from the same client observation. Session and
/// item identify the address, while the epoch bounds index reuse and the
/// revision records the observation used for action admission.
pub fn scene_accessibility(
    client: &ClientState,
    session: &ProjectionSession,
    profile: &CapabilityProfile,
    local: &dyn LocalActions,
) -> Result<SceneAccessibility, ResolutionError> {
    let scene = &client
        .mounted(session)
        .ok_or(ResolutionError::UnknownSession)?
        .scene;
    let tree = client.accessibility_tree_with(session, profile, local)?;
    let items = tree
        .children
        .iter()
        .map(|item| {
            let address = Address::new(
                session.clone(),
                accesskit::NodeId(u64::from(item.instance.0)),
            );
            let mut envelope = Envelope::scoped(address, scene.epoch.0, scene.revision);
            envelope.domain_ref = scene
                .active_item(item.instance)
                .and_then(|item| scene.tables.sources.get(item.source.0 as usize))
                .and_then(Option::as_ref)
                .cloned();
            envelope.authored_key = envelope.domain_ref.as_ref().map(|source| source.id.clone());
            for action in item.actions.iter().chain(&item.local_actions) {
                if !envelope.action_ids.contains(&action.intent.0) {
                    envelope.action_ids.push(action.intent.0.clone());
                }
            }
            envelope
        })
        .collect();
    Ok(SceneAccessibility { tree, items })
}
