// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! What a person is looking at, as opposed to what they arranged: camera,
//! focus and mirror mode. It is held in a device-local store and is never
//! written to the journal or the view log, never replicated (engine split
//! D133, contract C7). Camera and mirror mode have no production path yet;
//! they are part of the type's shape.

use serde::{Deserialize, Serialize};

/// 2D affine snapshot for serialisation. Stores the six kurbo
/// `Affine` coefficients verbatim — round-trips identity, pan,
/// zoom, and rotation.
///
/// Wire-shape kept compact (a single `[f64; 6]`) so the JSON
/// representation reads clearly and diffs cleanly:
///
/// ```text
/// "camera": { "coefficients": [1.0, 0.0, 0.0, 1.0, 100.0, 50.0] }
/// ```
///
/// (That value is a pure pan of (100, 50) with identity scale +
/// no rotation.)
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CameraSnapshot {
    /// kurbo `Affine::as_coeffs()` order: `[a, b, c, d, e, f]` for
    /// the affine `[[a, c, e], [b, d, f]]`.
    pub coefficients: [f64; 6],
}

impl CameraSnapshot {
    /// Identity transform — no pan, no zoom, no rotation.
    pub const IDENTITY: Self = Self {
        coefficients: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
    };

    /// True when this snapshot equals identity. The save path can
    /// skip persisting an identity camera (no user gesture worth
    /// recording).
    pub fn is_identity(&self) -> bool {
        *self == Self::IDENTITY
    }
}

impl Default for CameraSnapshot {
    fn default() -> Self {
        Self::IDENTITY
    }
}

/// One view's attention on this device.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AttentionState {
    /// Substrate camera snapshot (pan + zoom); `None` for an unmoved camera.
    #[serde(default)]
    pub camera: Option<CameraSnapshot>,
    /// URL of the node this pane is focused on, so a reload re-opens its
    /// media. `None` when nothing is focused.
    #[serde(default)]
    pub focus: Option<String>,
    /// Whether the pane is in live workbench-mirror mode.
    #[serde(default)]
    pub mirror_tiles: bool,
}

impl AttentionState {
    /// True when nothing in this state is worth keeping.
    pub fn is_empty(&self) -> bool {
        self.camera.is_none() && self.focus.is_none() && !self.mirror_tiles
    }
}
