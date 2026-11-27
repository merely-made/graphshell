// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! A host view bound to an engine revision (engine split D179, D187, D188).
//!
//! The host may hold one [`PinnedView`]. While it does, scenes are built from
//! the pin's graph; otherwise from the live graph, as before. Either way the
//! snapshot names the engine revision it was built from, and an intent is
//! stale when that revision or the projection counter has moved since.
use crate::PinnedView;
use super::*;
use pandect::{GraphRevision, ResourceDigest};
use scenotime::EngineRevision;

/// The engine revision a scene was stamped with, and the projection counter
/// it was stamped at.
#[derive(Clone, Debug)]
pub(super) struct SceneEngine {
    counter: u64,
    revision: GraphRevision,
}

fn engine_revision(revision: &GraphRevision) -> EngineRevision {
    EngineRevision {
        session: revision.session.as_uuid().to_string(),
        cursor: revision.cursor.0,
        digest: revision.digest.clone(),
    }
}

impl<B: Backend> MereHost<B> {
    /// Show `pin`: scenes are built from its graph until
    /// [`follow_live`](Self::follow_live). Returns the pin shown before, if
    /// any; dropping a pin is its holder's cost (engine split D166).
    ///
    /// # Errors
    ///
    /// [`MereHostError::WrongSession`] when the pin is of another session.
    pub fn show_pinned(&mut self, pin: PinnedView) -> Result<Option<PinnedView>, MereHostError> {
        if pin.session() != self.graph_session.id() {
            return Err(MereHostError::WrongSession);
        }
        self.reset_served_registry();
        Ok(self.pin.replace(pin))
    }

    /// Drop back to the live graph, handing back the pin that was shown.
    pub fn follow_live(&mut self) -> Option<PinnedView> {
        let pin = self.pin.take();
        if pin.is_some() {
            self.reset_served_registry();
        }
        pin
    }

    /// The pin scenes are built from, if one is shown.
    pub fn pinned(&self) -> Option<&PinnedView> {
        self.pin.as_ref()
    }

    /// The shown pin's Resource digest, with its algorithm id and version
    /// (engine binding D1, D2). The pin computes it on the first ask and keeps
    /// it, so the asker pays once per pin (mere:engine-split D191); snapshots
    /// never carry it.
    ///
    /// # Errors
    ///
    /// [`MereHostError::NotPinned`] while the live graph is shown, and
    /// [`MereHostError::ResourceDigest`] when the pin's Resource stratum holds
    /// a blank node.
    pub fn pinned_resource_digest(&self) -> Result<&ResourceDigest, MereHostError> {
        let pin = self.pin.as_ref().ok_or(MereHostError::NotPinned)?;
        Ok(pin.resource_digest()?)
    }

    /// The graph scenes are built from: the pin's when one is shown.
    pub(super) fn scene_graph(&self) -> &Graph {
        self.pin
            .as_ref()
            .map_or_else(|| self.graph_session.graph(), PinnedView::graph)
    }

    /// The engine revision a scene built now would carry.
    ///
    /// # Errors
    ///
    /// A session error when the live journal's digest cannot be read.
    pub fn scene_engine_revision(&self) -> Result<EngineRevision, MereHostError> {
        Ok(engine_revision(&self.scene_revision()?))
    }

    /// The live engine revision, which an intent must have been made against.
    ///
    /// # Errors
    ///
    /// A session error when the live journal's digest cannot be read.
    pub fn live_engine_revision(&self) -> Result<EngineRevision, MereHostError> {
        Ok(engine_revision(&self.graph_session.live_revision()?))
    }

    fn scene_revision(&self) -> Result<GraphRevision, MereHostError> {
        match &self.pin {
            Some(pin) => Ok(pin.revision_identity()),
            None => Ok(self.graph_session.live_revision()?),
        }
    }

    /// Stamp the scene about to be built. The counter moves when the engine
    /// revision did and nothing host-local moved it already, so one counter
    /// value always names one engine revision.
    pub(super) fn stamp_scene(&mut self) -> Result<EngineRevision, MereHostError> {
        let revision = self.scene_revision()?;
        if self.scene_engine.as_ref().is_some_and(|stamped| {
            stamped.counter == self.projection_revision && stamped.revision != revision
        }) {
            self.projection_revision = self.projection_revision.wrapping_add(1);
        }
        let stamped = engine_revision(&revision);
        self.scene_engine = Some(SceneEngine {
            counter: self.projection_revision,
            revision,
        });
        Ok(stamped)
    }

    /// Whether the engine has moved past the scene last served, which an
    /// intent observing the current counter was made against.
    /// A stale reply naming the scene counter and the live engine revision
    /// an intent would now apply to (engine split D193).
    pub(super) fn stale_reply(&self) -> Result<IntentResult, MereHostError> {
        Ok(IntentResult::stale_at_engine(
            SceneEpoch(self.projection_epoch),
            Revision(self.projection_revision),
            Some(self.live_engine_revision()?),
        ))
    }

    pub(super) fn engine_moved(&self) -> Result<bool, MereHostError> {
        let Some(stamped) = &self.scene_engine else {
            return Ok(false);
        };
        Ok(stamped.revision != self.graph_session.live_revision()?)
    }

    /// The live node a disclosed instance target names. A pinned scene's keys
    /// belong to the pin's graph, so they are resolved through node ids.
    pub(super) fn live_target(&self, target: NodeKey) -> Option<NodeKey> {
        match &self.pin {
            Some(pin) => {
                let id = pin.graph().get_node(target)?.id;
                self.graph().get_node_key_by_id(id)
            },
            None => Some(target),
        }
    }

    fn reset_served_registry(&mut self) {
        // One registry serves one graph.
        *self
            .served_registry
            .get_mut()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) =
            cartography::canvas::ChannelRegistry::new();
    }
}

#[cfg(test)]
#[path = "mere_host_pin_tests.rs"]
mod tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "mere_host_arrangement_tests.rs"]
mod arrangement_tests;

#[cfg(test)]
#[path = "mere_host_digest_tests.rs"]
mod digest_tests;

#[cfg(test)]
#[path = "mere_host_recovery_tests.rs"]
mod recovery_tests;
