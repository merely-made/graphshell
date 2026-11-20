// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

/// A producer's common world-to-content mapping for its semantic rectangles.
///
/// The host retains the rectangles already published at an anchor mapping.
/// An equal revision asserts that only `scale` and `origin` have changed.
/// Revisions must never be reused for different non-transform semantics;
/// `u64::MAX` disables retention after a saturating revision counter exhausts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProducerSemanticGeometry {
    pub revision: u64,
    pub scale: f64,
    pub origin: [f64; 2],
}

impl ProducerSemanticGeometry {
    pub fn is_valid(self) -> bool {
        self.revision != u64::MAX
            && self.scale.is_finite()
            && self.scale > 0.0
            && self.origin.iter().all(|value| value.is_finite())
    }

    /// Map the CSS rectangles retained at `anchor` into this publication.
    /// Content offsets are relative to the producer's border box, in logical
    /// pixels. Host scales convert logical pixels to browser CSS pixels.
    /// Always compare with the original anchor to avoid cumulative drift.
    pub fn transform_from(
        self,
        anchor: Self,
        anchor_host_scale: f64,
        host_scale: f64,
        anchor_content_offset: [f64; 2],
        content_offset: [f64; 2],
    ) -> Option<[f64; 3]> {
        if !self.is_valid()
            || !anchor.is_valid()
            || self.revision != anchor.revision
            || !anchor_host_scale.is_finite()
            || anchor_host_scale <= 0.0
            || !host_scale.is_finite()
            || host_scale <= 0.0
            || !anchor_content_offset.iter().all(|value| value.is_finite())
            || !content_offset.iter().all(|value| value.is_finite())
        {
            return None;
        }
        let ratio = self.scale / anchor.scale;
        let scale = host_scale / anchor_host_scale * ratio;
        let translation = |axis: usize| {
            host_scale
                * (content_offset[axis] + self.origin[axis]
                    - ratio * anchor.origin[axis]
                    - ratio * anchor_content_offset[axis])
        };
        let result = [scale, translation(0), translation(1)];
        (scale > 0.0 && result.iter().all(|value| value.is_finite())).then_some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retained_transform_matches_fresh_rectangles_with_padding_and_host_zoom() {
        let anchor = ProducerSemanticGeometry {
            revision: 42,
            scale: 0.25,
            origin: [120.0, -40.0],
        };
        let current = ProducerSemanticGeometry {
            revision: 42,
            scale: 0.75,
            origin: [-82.0, 160.0],
        };
        let [scale, tx, ty] = current
            .transform_from(anchor, 1.25, 0.8, [8.0, 12.0], [20.0, 5.0])
            .unwrap();
        for [x, y, width, height] in [[-20.0, 400.0, 120.0, 80.0], [10000.0, -5000.0, 20.0, 40.0]] {
            let old = [
                1.25 * (8.0 + anchor.origin[0] + anchor.scale * x),
                1.25 * (12.0 + anchor.origin[1] + anchor.scale * y),
                1.25 * anchor.scale * width,
                1.25 * anchor.scale * height,
            ];
            let transformed = [
                old[0] * scale + tx,
                old[1] * scale + ty,
                old[2] * scale,
                old[3] * scale,
            ];
            let fresh = [
                0.8 * (20.0 + current.origin[0] + current.scale * x),
                0.8 * (5.0 + current.origin[1] + current.scale * y),
                0.8 * current.scale * width,
                0.8 * current.scale * height,
            ];
            for (actual, expected) in transformed.into_iter().zip(fresh) {
                assert!((actual - expected).abs() < 1e-9, "{actual} != {expected}");
            }
        }
    }

    #[test]
    fn invalid_or_changed_contributions_cannot_retain_rectangles() {
        let valid = ProducerSemanticGeometry {
            revision: 1,
            scale: 1.0,
            origin: [0.0, 0.0],
        };
        let map = |current: ProducerSemanticGeometry,
                   anchor,
                   old_scale,
                   new_scale,
                   old_offset,
                   new_offset| {
            current.transform_from(anchor, old_scale, new_scale, old_offset, new_offset)
        };
        for bad in [
            ProducerSemanticGeometry {
                revision: 2,
                ..valid
            },
            ProducerSemanticGeometry {
                revision: u64::MAX,
                ..valid
            },
            ProducerSemanticGeometry {
                scale: 0.0,
                ..valid
            },
            ProducerSemanticGeometry {
                scale: -1.0,
                ..valid
            },
            ProducerSemanticGeometry {
                scale: f64::NAN,
                ..valid
            },
            ProducerSemanticGeometry {
                origin: [f64::INFINITY, 0.0],
                ..valid
            },
        ] {
            assert!(map(bad, valid, 1.0, 1.0, [0.0; 2], [0.0; 2]).is_none());
            assert!(map(valid, bad, 1.0, 1.0, [0.0; 2], [0.0; 2]).is_none());
        }
        for bad in [0.0, -1.0, f64::INFINITY, f64::NAN] {
            assert!(map(valid, valid, bad, 1.0, [0.0; 2], [0.0; 2]).is_none());
            assert!(map(valid, valid, 1.0, bad, [0.0; 2], [0.0; 2]).is_none());
            assert!(map(valid, valid, 1.0, 1.0, [f64::NAN, 0.0], [0.0; 2]).is_none());
        }
        let tiny = ProducerSemanticGeometry {
            scale: f64::MIN_POSITIVE,
            ..valid
        };
        let huge = ProducerSemanticGeometry {
            scale: f64::MAX,
            ..valid
        };
        assert!(map(huge, tiny, 1.0, 1.0, [0.0; 2], [0.0; 2]).is_none());
    }
}
