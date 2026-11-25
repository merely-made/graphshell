// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The coordinate domain a recipe asks an arrangement to use.

use crate::{Rect, Transform2, Vec2};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrameKind {
    Unbounded,
    Bounded,
    Nested,
    Flow,
}
impl FrameKind {
    pub fn id(self) -> &'static str {
        match self {
            Self::Unbounded => "unbounded",
            Self::Bounded => "bounded",
            Self::Nested => "nested",
            Self::Flow => "flow",
        }
    }
}

/// An explicit arrangement domain. Omitting the slot retains a legacy
/// arrangement's bounds/origin; an explicit frame is authoritative.
///
/// Nested bounds are local to a Space. The host supplies the transform from
/// that node/region's coordinates into the parent scene; its source identity
/// remains the host's. Flow fixes page width and lets content determine height.
/// Grid caps its column count to that width; authored cell coordinates must fit.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Frame {
    Unbounded { origin: Vec2 },
    Bounded { bounds: Rect },
    Nested { bounds: Rect, transform: Transform2 },
    Flow { origin: Vec2, width: f32 },
}

impl Frame {
    pub fn kind(&self) -> FrameKind {
        match self {
            Self::Unbounded { .. } => FrameKind::Unbounded,
            Self::Bounded { .. } => FrameKind::Bounded,
            Self::Nested { .. } => FrameKind::Nested,
            Self::Flow { .. } => FrameKind::Flow,
        }
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        fn point(p: Vec2) -> bool {
            p.x.is_finite() && p.y.is_finite()
        }
        fn rect(b: Rect) -> bool {
            point(b.origin)
                && b.size.w.is_finite()
                && b.size.h.is_finite()
                && b.size.w > 0.0
                && b.size.h > 0.0
                && point(Vec2::new(b.origin.x + b.size.w, b.origin.y + b.size.h))
        }
        let valid = match self {
            Self::Unbounded { origin } => point(*origin),
            Self::Bounded { bounds } => rect(*bounds),
            Self::Nested { bounds, transform } => {
                rect(*bounds)
                    && point(transform.translate)
                    && transform.rotate.is_finite()
                    && transform.scale.is_finite()
                    && transform.scale != 0.0
                    && [
                        (0.0, 0.0),
                        (bounds.size.w, 0.0),
                        (0.0, bounds.size.h),
                        (bounds.size.w, bounds.size.h),
                    ]
                    .into_iter()
                    .all(|(x, y)| {
                        point(transform.apply(Vec2::new(bounds.origin.x + x, bounds.origin.y + y)))
                    })
            },
            Self::Flow { origin, width } => {
                point(*origin)
                    && width.is_finite()
                    && *width > 0.0
                    && (origin.x + width).is_finite()
            },
        };
        if valid {
            Ok(())
        } else {
            Err(
                "frame coordinates must be finite, extents positive, and a nested transform invertible",
            )
        }
    }
}

/// The instantiated frame in a Scene. The spec records the authored choice;
/// its transform seeds a nested Space. Current placement and bounds use
/// that Space's live transform, including later group movement.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneFrame {
    pub spec: Frame,
    pub space: crate::SpaceId,
}

impl SceneFrame {
    /// The frame's rectangle in current world coordinates. Unbounded frames
    /// impose no rectangle; flow frames contribute width with no fixed height.
    pub fn world_bounds(&self, scene: &crate::Scene) -> Option<Rect> {
        let bounds = match &self.spec {
            Frame::Unbounded { .. } => return None,
            Frame::Bounded { bounds } | Frame::Nested { bounds, .. } => *bounds,
            Frame::Flow { origin, width } => Rect::new(*origin, crate::Size2::new(*width, 0.0)),
        };
        let transform = scene.to_world(self.space)?;
        [
            (0.0, 0.0),
            (bounds.size.w, 0.0),
            (0.0, bounds.size.h),
            (bounds.size.w, bounds.size.h),
        ]
        .into_iter()
        .map(|(x, y)| {
            Rect::new(
                transform.apply(Vec2::new(bounds.origin.x + x, bounds.origin.y + y)),
                crate::Size2::default(),
            )
        })
        .reduce(Rect::union)
    }
}
