// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Resume from a client's acknowledged revision (engine binding E3).

use serde::{Deserialize, Serialize};

use crate::{
    EngineRevision, ProjectionDiff, ProjectionSession, ProjectionSnapshot, Revision, SceneEpoch,
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
}

impl ResumeRequest {
    /// A resume from `revision` within `epoch`. Build requests here rather
    /// than by struct literal: the engine revision joins as a field next
    /// (engine binding D5), and a literal would have to name it.
    pub fn new(session: ProjectionSession, epoch: SceneEpoch, revision: Revision) -> Self {
        Self {
            session,
            epoch,
            revision,
        }
    }

    /// This request, naming the engine revision the client holds. Until that
    /// field lands (engine binding D5) it names none.
    pub fn with_engine(self, _engine_revision: Option<EngineRevision>) -> Self {
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ResumeReply {
    Current(ProjectionAck),
    Diffs(Vec<ProjectionDiff>),
    Snapshot(Box<ProjectionSnapshot>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SceneDiff;

    #[test]
    fn the_constructors_build_what_a_literal_did() {
        let session = ProjectionSession("fixture:d5".into());
        assert_eq!(
            ResumeRequest::new(session.clone(), SceneEpoch(2), Revision(5)).with_engine(None),
            ResumeRequest {
                session,
                epoch: SceneEpoch(2),
                revision: Revision(5),
            }
        );
        let diff = SceneDiff::new(SceneEpoch(2), Revision(5), Revision(6), Vec::new());
        assert_eq!(
            diff,
            SceneDiff {
                epoch: SceneEpoch(2),
                base: Revision(5),
                revision: Revision(6),
                operations: Vec::new(),
            }
        );
    }
}
