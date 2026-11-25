// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Resume, and the deltas a producer could retain to answer it (engine
//! binding E3).
//!
//! A host that missed a delta resumes from its acknowledged identity and
//! epoch (engine split D181). Today every gap is answered with a fresh
//! keyframe, [`keyframe_reply`]: the served layout moves every node at each
//! edit, so a retained delta outweighs the keyframe it would spare (D206).
//! [`DeltaRetention`], which replays a verified suffix under D201's bound,
//! stays for when placement changes; nothing constructs it.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use crate::{
    EngineRevision, ProjectionDiff, ProjectionSession, ProjectionSnapshot, Revision, SceneEpoch,
    validate_presentation,
};

/// The last revision a client has applied in memory: the session, epoch and
/// revision of its mounted scene. It says nothing about what the client has
/// stored.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectionAck {
    pub session: ProjectionSession,
    pub epoch: SceneEpoch,
    pub revision: Revision,
}

/// Reconnect from a client's last acknowledged epoch and revision.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResumeRequest {
    pub session: ProjectionSession,
    pub epoch: SceneEpoch,
    pub revision: Revision,
    /// The engine revision of the scene the client holds, when its scene
    /// names one (engine binding E3). Absent from the wire when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine_revision: Option<EngineRevision>,
}

impl ResumeRequest {
    /// A request that names no engine revision.
    pub fn new(session: ProjectionSession, epoch: SceneEpoch, revision: Revision) -> Self {
        Self {
            session,
            epoch,
            revision,
            engine_revision: None,
        }
    }

    /// This request, naming the engine revision the client holds.
    pub fn with_engine(mut self, engine_revision: Option<EngineRevision>) -> Self {
        self.engine_revision = engine_revision;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ResumeReply {
    /// Plain canonical CBOR: it carries no scene (D219).
    Current(ProjectionAck),
    /// A sequence of diff envelopes, each bound alone.
    Diffs(#[serde(with = "crate::envelope::diffs")] Vec<ProjectionDiff>),
    /// A keyframe envelope.
    Snapshot(#[serde(with = "crate::envelope::keyframe")] Box<ProjectionSnapshot>),
}

/// Answer a resume with nothing retained: current when the client holds the
/// keyframe's epoch, revision and engine revision, and otherwise the keyframe
/// (engine split D206). A client whose counter matches under another engine
/// revision has diverged and gets the keyframe too.
pub fn keyframe_reply(request: &ResumeRequest, keyframe: ProjectionSnapshot) -> ResumeReply {
    if request.epoch == keyframe.scene.epoch
        && request.revision == keyframe.scene.revision
        && request.engine_revision == keyframe.scene.engine_revision
    {
        ResumeReply::Current(ProjectionAck {
            session: keyframe.session,
            epoch: keyframe.scene.epoch,
            revision: keyframe.scene.revision,
        })
    } else {
        ResumeReply::Snapshot(Box::new(keyframe))
    }
}

/// How many deltas a producer retains at most (engine split D201).
pub const RETAINED_DELTAS: usize = 64;

/// How a keyframe and a delta are counted against each other. The default is
/// the envelope each travels in.
#[derive(Clone, Copy, Debug)]
pub struct WireSize {
    pub keyframe: fn(&ProjectionSnapshot) -> usize,
    pub delta: fn(&ProjectionDiff) -> usize,
}

impl Default for WireSize {
    fn default() -> Self {
        Self {
            keyframe: |snapshot| crate::envelope::seal(snapshot).map_or(usize::MAX, |b| b.len()),
            delta: |diff| crate::envelope::seal(diff).map_or(usize::MAX, |b| b.len()),
        }
    }
}

/// What [`DeltaRetention::advance`] did with a build.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Advance {
    /// The build is the scene already held.
    Unchanged,
    /// The delta to the build was retained and applied to the held scene.
    Retained,
    /// The build became the held scene and nothing before it is retained:
    /// the first build, another epoch or session, or a change no delta says.
    Restarted,
}

#[derive(Clone, Debug)]
struct Retained {
    diff: ProjectionDiff,
    bytes: usize,
}

/// A producer's retained deltas and the scene its clients hold at their head.
///
/// The producer builds whole snapshots; each build is diffed against the held
/// scene, and the held scene advances by applying that diff in place, so it
/// is exactly what a client that applied every delta holds (a shorter build
/// leaves tombstones there). The held scene is the keyframe it serves.
#[derive(Clone, Debug)]
pub struct DeltaRetention {
    held: Option<ProjectionSnapshot>,
    deltas: VecDeque<Retained>,
    bytes: usize,
    keyframe_bytes: usize,
    capacity: usize,
    size: WireSize,
}

impl Default for DeltaRetention {
    fn default() -> Self {
        Self::new(RETAINED_DELTAS, WireSize::default())
    }
}

impl DeltaRetention {
    /// Retain at most `capacity` deltas, counted against a keyframe by `size`.
    pub fn new(capacity: usize, size: WireSize) -> Self {
        Self {
            held: None,
            deltas: VecDeque::new(),
            bytes: 0,
            keyframe_bytes: 0,
            capacity,
            size,
        }
    }

    /// The scene a client holds after every retained delta: the keyframe.
    pub fn keyframe(&self) -> Option<&ProjectionSnapshot> {
        self.held.as_ref()
    }

    /// The retained deltas, oldest first.
    pub fn retained(&self) -> impl Iterator<Item = &ProjectionDiff> {
        self.deltas.iter().map(|retained| &retained.diff)
    }

    pub fn len(&self) -> usize {
        self.deltas.len()
    }

    pub fn is_empty(&self) -> bool {
        self.deltas.is_empty()
    }

    /// The retained deltas' bytes, and the held keyframe's, as `size` counts.
    pub fn bytes(&self) -> (usize, usize) {
        (self.bytes, self.keyframe_bytes)
    }

    /// Take a freshly built snapshot: retain the delta from the held scene
    /// to it, or restart from it when no delta says the change.
    pub fn advance(&mut self, built: ProjectionSnapshot) -> Advance {
        let Some(held) = self.held.as_mut() else {
            return self.restart(built);
        };
        if built.version != held.version
            || built.session != held.session
            || built.cache_policy != held.cache_policy
            || built.scene.epoch != held.scene.epoch
        {
            return self.restart(built);
        }
        let presentation = held.presentation.changes_to(&built.presentation);
        if built.scene.revision == held.scene.revision
            && built.scene.engine_revision == held.scene.engine_revision
            && presentation.is_empty()
            && held
                .scene
                .operations_to(&built.scene)
                .is_some_and(|ops| ops.is_empty())
        {
            return Advance::Unchanged;
        }
        let Some(scene) = held.scene.diff_to(&built.scene) else {
            return self.restart(built);
        };
        let diff = ProjectionDiff {
            version: held.version,
            session: held.session.clone(),
            scene,
            presentation,
            status: None,
        };
        let applied = held.scene.apply_diff_staged(&diff.scene).is_ok() && {
            held.presentation.apply_changes(&diff.presentation);
            validate_presentation(&held.scene, &held.presentation).is_ok()
        };
        if !applied {
            return self.restart(built);
        }
        self.keyframe_bytes = (self.size.keyframe)(held);
        let bytes = (self.size.delta)(&diff);
        self.bytes = self.bytes.saturating_add(bytes);
        self.deltas.push_back(Retained { diff, bytes });
        while self.deltas.len() > self.capacity || self.bytes > self.keyframe_bytes {
            let Some(evicted) = self.deltas.pop_front() else {
                break;
            };
            self.bytes -= evicted.bytes;
        }
        Advance::Retained
    }

    fn restart(&mut self, built: ProjectionSnapshot) -> Advance {
        self.keyframe_bytes = (self.size.keyframe)(&built);
        self.held = Some(built);
        self.deltas.clear();
        self.bytes = 0;
        Advance::Restarted
    }

    /// Answer a resume against the held scene: current when the client holds
    /// it, the retained suffix from the client's base when one verifies, and
    /// otherwise the keyframe. `None` when nothing is held for that session.
    pub fn resume(&self, request: &ResumeRequest) -> Option<ResumeReply> {
        let held = self.held.as_ref()?;
        if request.session != held.session {
            return None;
        }
        let at_head = request.epoch == held.scene.epoch
            && request.revision == held.scene.revision
            && request.engine_revision == held.scene.engine_revision;
        if at_head {
            return Some(ResumeReply::Current(ProjectionAck {
                session: held.session.clone(),
                epoch: held.scene.epoch,
                revision: held.scene.revision,
            }));
        }
        Some(match self.suffix(request, held) {
            Some(diffs) => ResumeReply::Diffs(diffs),
            None => ResumeReply::Snapshot(Box::new(held.clone())),
        })
    }

    /// The retained deltas from the client's base to the head, when they
    /// begin at the identity the client names, run contiguously, and end at
    /// the held scene.
    fn suffix(&self, request: &ResumeRequest, held: &ProjectionSnapshot) -> Option<Vec<ProjectionDiff>> {
        if request.epoch != held.scene.epoch {
            return None;
        }
        let start = self
            .deltas
            .iter()
            .position(|retained| retained.diff.scene.base == request.revision)?;
        let mut revision = request.revision;
        let mut engine = &request.engine_revision;
        for retained in self.deltas.range(start..) {
            let scene = &retained.diff.scene;
            if scene.epoch != request.epoch
                || scene.base != revision
                || &scene.base_engine != engine
                || scene.revision <= scene.base
            {
                return None;
            }
            revision = scene.revision;
            engine = &scene.engine_revision;
        }
        (revision == held.scene.revision && engine == &held.scene.engine_revision).then(|| {
            self.deltas
                .range(start..)
                .map(|retained| retained.diff.clone())
                .collect()
        })
    }
}

#[cfg(test)]
#[path = "resume_tests.rs"]
mod tests;
