// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Pin and slide rectangular tiles using the disclosed one-step neighborhood.
use std::collections::BTreeSet;

use sceno::{Rect, Size2, Vec2};

mod packing;
mod scene;

pub(crate) use scene::initialize;
pub use scene::{CrystalEdit, edit};
#[cfg(test)]
mod tests;

const EPSILON: f32 = 0.0001;
pub(crate) const CLEARANCE: f32 = 8.0;

struct Layout {
    tiles: Vec<Rect>,
    linked: Vec<BTreeSet<usize>>,
    pins: BTreeSet<usize>,
    smallest: f32,
    preferred: Size2,
}

impl Layout {
    fn new(
        tiles: Vec<Rect>,
        links: impl IntoIterator<Item = (usize, usize)>,
        pins: BTreeSet<usize>,
        smallest: f32,
        preferred: Size2,
    ) -> Option<Self> {
        if !smallest.is_finite()
            || smallest <= 0.0
            || tiles.iter().any(|tile| {
                ![tile.origin.x, tile.origin.y, right(*tile), bottom(*tile)]
                    .into_iter()
                    .all(f32::is_finite)
                    || tile.size.w < smallest
                    || tile.size.h < smallest
            })
        {
            return None;
        }
        let mut linked = vec![BTreeSet::new(); tiles.len()];
        for (a, b) in links {
            if a < tiles.len() && b < tiles.len() && a != b {
                linked[a].insert(b);
                linked[b].insert(a);
            }
        }
        Some(Self {
            tiles,
            linked,
            pins,
            smallest,
            preferred,
        })
    }

    fn pin(&mut self, tile: usize) {
        if self.pins.insert(tile)
            && let Some(packed) = self.pack(tile, None)
        {
            self.tiles = packed;
        }
    }

    fn unpin(&mut self, tile: usize) {
        self.pins.remove(&tile);
    }

    fn drop_at(&mut self, tile: usize, target: Vec2) -> bool {
        if self.pins.contains(&tile) || !target.x.is_finite() || !target.y.is_finite() {
            return false;
        }
        let around = self.tiles[tile];
        if let Some(&nucleus) = self.linked[tile].iter().find(|&&other| {
            self.pins.contains(&other)
                && border(around, self.tiles[other], self.smallest / 2.0).is_some()
        }) {
            return self.slide(tile, nucleus, target);
        }

        let next = centered(target, around.size);
        if self
            .tiles
            .iter()
            .enumerate()
            .all(|(other, &obstacle)| other == tile || self.separated(tile, next, other, obstacle))
        {
            self.tiles[tile] = next;
            true
        } else {
            false
        }
    }

    fn slide(&mut self, tile: usize, nucleus: usize, target: Vec2) -> bool {
        let size = self.tiles[tile].size;
        let mut anchors = packing::SIDES
            .map(|side| side.nearest(self.tiles[nucleus], size, target, self.smallest / 2.0));
        anchors.sort_by(|a, b| distance(*a, target).total_cmp(&distance(*b, target)));
        for anchor in anchors {
            if let Some(packed) = self.pack(nucleus, Some((tile, anchor))) {
                self.tiles = packed;
                return true;
            }
        }
        false
    }

    fn separated(&self, tile: usize, candidate: Rect, other: usize, obstacle: Rect) -> bool {
        let gap = if self.linked[tile].contains(&other) {
            0.0
        } else {
            CLEARANCE
        };
        candidate.origin.x.is_finite()
            && candidate.origin.y.is_finite()
            && (right(candidate) + gap <= obstacle.origin.x + EPSILON
                || right(obstacle) + gap <= candidate.origin.x + EPSILON
                || bottom(candidate) + gap <= obstacle.origin.y + EPSILON
                || bottom(obstacle) + gap <= candidate.origin.y + EPSILON)
    }
}

fn centered(at: Vec2, size: Size2) -> Rect {
    Rect::new(Vec2::new(at.x - size.w / 2.0, at.y - size.h / 2.0), size)
}

fn center(rect: Rect) -> Vec2 {
    Vec2::new(
        rect.origin.x + rect.size.w / 2.0,
        rect.origin.y + rect.size.h / 2.0,
    )
}

fn right(rect: Rect) -> f32 {
    rect.origin.x + rect.size.w
}

fn bottom(rect: Rect) -> f32 {
    rect.origin.y + rect.size.h
}

fn distance(rect: Rect, target: Vec2) -> f64 {
    let at = center(rect);
    (f64::from(at.x) - f64::from(target.x)).powi(2)
        + (f64::from(at.y) - f64::from(target.y)).powi(2)
}

/// The segment of a common edge that meets the recipe's threshold.
fn border(a: Rect, b: Rect, minimum: f32) -> Option<[Vec2; 2]> {
    let (top, foot) = (a.origin.y.max(b.origin.y), bottom(a).min(bottom(b)));
    if foot - top + EPSILON >= minimum {
        for (left, right) in [(a, b), (b, a)] {
            let x = right.origin.x;
            if (self::right(left) - x).abs() <= EPSILON {
                return Some([Vec2::new(x, top), Vec2::new(x, foot)]);
            }
        }
    }
    let (start, end) = (a.origin.x.max(b.origin.x), right(a).min(right(b)));
    if end - start + EPSILON >= minimum {
        for (upper, lower) in [(a, b), (b, a)] {
            let y = lower.origin.y;
            if (bottom(upper) - y).abs() <= EPSILON {
                return Some([Vec2::new(start, y), Vec2::new(end, y)]);
            }
        }
    }
    None
}
