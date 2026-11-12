// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

use std::{collections::HashSet, time::Duration};

use emblem::{Palette, Rgba, ViewBox};
use kernel::graph::NodeKey;
use paint_list_api::{ColorF, LayoutTransform, PaintCmd};

use pictograph::backdrop::{CLEAR, LATTICE, MOTIF, OTHER, PALETTE_ENTRIES, ViewDigest};

use super::{Canvas, territory::TIER_ALPHA};

mod cadence;
pub(super) use cadence::WashPresentation;

const WASH_ALPHA: u8 = (TIER_ALPHA[2] * 255.0) as u8;
const MOTIF_ALPHA: u8 = WASH_ALPHA / 2;
// sRGB's linearization slope is at most 2.275, so limiting each channel
// to this interval caps relative luminance distance at 0.183.
const CHANNEL_DISTANCE: f32 = 0.08;

impl Canvas {
    /// Paint the held view's content wash beneath territory. Off by default.
    pub fn set_content_backdrop(&mut self, enabled: bool) {
        if !enabled && self.content_backdrop {
            self.wash = WashPresentation::default();
        }
        self.content_backdrop = enabled;
    }

    /// Whether the content-responsive wash is enabled.
    pub fn content_backdrop_enabled(&self) -> bool {
        self.content_backdrop
    }

    /// Physical pixels per logical canvas pixel, used for wash detail selection.
    pub fn set_content_backdrop_device_scale(&mut self, scale: f32) {
        self.content_backdrop_scale = if scale.is_finite() && scale > 0.0 {
            scale
        } else {
            1.0
        };
    }

    /// Export the selected wash arm in top-down world proportions with the active theme.
    /// An absent or outdated scope has no export until the next frame.
    ///
    /// # Errors
    ///
    /// Returns the Emblem SVG error if the retained wash cannot be serialized.
    #[cfg(feature = "backdrop-svg")]
    pub fn content_backdrop_svg(&self) -> Result<Option<String>, emblem::svg::SvgError> {
        if !self.content_backdrop
            || !self.wash.matches_visibility(
                self.scope
                    .as_ref()
                    .map(|scope| scope.iter().copied().collect()),
                self.fold_hidden_keys(),
            )
            || self.backdrop_digest().is_none()
        {
            return Ok(None);
        }
        self.wash.svg(&self.content_backdrop_palette())
    }

    pub(super) fn content_backdrop_commands(
        &mut self,
        dt: Duration,
        scope: Option<HashSet<NodeKey>>,
        folded: HashSet<NodeKey>,
    ) -> (Vec<PaintCmd>, bool) {
        if !self.content_backdrop || !self.camera.is_valid() {
            return (Vec::new(), false);
        }
        let digest = self.backdrop_digest();
        let palette = self.content_backdrop_palette();
        let visibility_changed = self.wash.update_visibility(scope, folded);
        self.wash.present(
            digest,
            visibility_changed,
            dt,
            self.camera.ground_transform(),
            self.content_backdrop_scale,
            &palette,
        )
    }

    fn content_backdrop_palette(&self) -> Palette {
        let edge = self.style.edge_color;
        let neutral = [edge.r, edge.g, edge.b];
        let mut colors = [neutral; PALETTE_ENTRIES];
        for (index, color) in colors[..usize::from(OTHER)].iter_mut().enumerate() {
            let (r, g, b) = self.category_rgb(index);
            *color = [r, g, b];
        }
        wash_palette(self.backdrop, colors)
    }
}

fn wash_world_transform(digest: &ViewDigest) -> LayoutTransform {
    let [left, top, right, bottom] = digest.bounds();
    let lattice = f64::from(LATTICE);
    let width = ((f64::from(right) - f64::from(left)) * lattice) as f32;
    let height = ((f64::from(bottom) - f64::from(top)) * lattice) as f32;
    let view = ViewBox::default();
    let sx = width / (view.max_x - view.min_x);
    let sy = height / (view.max_y - view.min_y);
    let x = (f64::from(left) * lattice) as f32 - view.min_x * sx;
    let y = (f64::from(top) * lattice) as f32 - view.min_y * sy;
    LayoutTransform::scale(sx, sy, 1.0).then(&LayoutTransform::translation(x, y, 0.0))
}

fn wash_height(transform: &LayoutTransform) -> f32 {
    // Project the view box's vertical side to get its raster height, including
    // the ground plane's yaw and foreshortening.
    let view = ViewBox::default();
    transform.m21.hypot(transform.m22) * (view.max_y - view.min_y)
}

fn wash_palette(ground: ColorF, colors: [[f32; 3]; PALETTE_ENTRIES]) -> Palette {
    let ground = [ground.r, ground.g, ground.b].map(|channel| channel.max(0.0).min(1.0));
    let mut palette = Palette::default();
    for (index, color) in colors.into_iter().enumerate() {
        let alpha = match index as u8 {
            MOTIF => MOTIF_ALPHA,
            CLEAR => 0,
            _ => WASH_ALPHA,
        };
        let a = f32::from(alpha);
        // Clamp in the premultiplied byte domain so quantization also respects
        // the interval. A transparent slot resolves to [0, 0, 0, 0].
        let channel = |i: usize| {
            let low = ((ground[i] - CHANNEL_DISTANCE).max(0.0) * a).ceil();
            let high = ((ground[i] + CHANNEL_DISTANCE).min(1.0) * a).floor();
            (color[i] * a).round().clamp(low, high) as u8
        };
        palette.0[index] = Rgba::new(channel(0), channel(1), channel(2), alpha);
    }
    palette
}

#[cfg(test)]
mod tests;
