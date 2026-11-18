// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Search boundary events at minimum size first, then grow leaves where room remains.
use super::*;

const BEAM_WIDTH: usize = 8;
const CANDIDATES_PER_STATE: usize = 12;

#[derive(Clone, Copy)]
pub(super) enum Side {
    Top,
    Right,
    Bottom,
    Left,
}
pub(super) const SIDES: [Side; 4] = [Side::Top, Side::Right, Side::Bottom, Side::Left];

impl Side {
    fn along(self, r: Rect) -> (f32, f32) {
        match self {
            Self::Top | Self::Bottom => (r.origin.x, right(r)),
            Self::Left | Self::Right => (r.origin.y, bottom(r)),
        }
    }

    fn span(self, size: Size2) -> f32 {
        match self {
            Self::Top | Self::Bottom => size.w,
            Self::Left | Self::Right => size.h,
        }
    }

    fn at(self, nucleus: Rect, size: Size2, along: f32) -> Rect {
        let origin = match self {
            Self::Top => Vec2::new(along, nucleus.origin.y - size.h),
            Self::Right => Vec2::new(right(nucleus), along),
            Self::Bottom => Vec2::new(along, bottom(nucleus)),
            Self::Left => Vec2::new(nucleus.origin.x - size.w, along),
        };
        Rect::new(origin, size)
    }

    fn interval(self, nucleus: Rect, size: Size2, border: f32) -> (f32, f32) {
        let (start, end) = self.along(nucleus);
        (start + border - self.span(size), end - border)
    }

    pub(super) fn nearest(self, nucleus: Rect, size: Size2, target: Vec2, border: f32) -> Rect {
        let (start, end) = self.interval(nucleus, size, border);
        let toward = match self {
            Self::Top | Self::Bottom => target.x,
            Self::Left | Self::Right => target.y,
        };
        self.at(
            nucleus,
            size,
            (toward - self.span(size) / 2.0).clamp(start, end),
        )
    }
}

#[derive(Clone)]
struct Candidate {
    tiles: Vec<Option<Rect>>,
    attached: usize,
    borders: usize,
    travel: f64,
}

impl Candidate {
    fn best_first(a: &Self, b: &Self) -> std::cmp::Ordering {
        b.attached
            .cmp(&a.attached)
            .then_with(|| b.borders.cmp(&a.borders))
            .then_with(|| a.travel.total_cmp(&b.travel))
    }
}

impl Layout {
    pub(super) fn pack(&self, nucleus: usize, anchor: Option<(usize, Rect)>) -> Option<Vec<Rect>> {
        let border_min = self.smallest / 2.0;
        let main = self.tiles[nucleus];
        let anchored = anchor.map(|(tile, _)| tile);
        let mut movers: Vec<_> = self.linked[nucleus]
            .iter()
            .copied()
            .filter(|tile| !self.pins.contains(tile) && Some(*tile) != anchored)
            .collect();
        // With a moved anchor, place displaced occupants before optional leaves.
        movers.sort_by_key(|&tile| {
            let displaced = anchor
                .is_some_and(|(other, rect)| !self.separated(tile, self.tiles[tile], other, rect));
            (!displaced, std::cmp::Reverse(self.linked[tile].len()), tile)
        });
        let mut slots: Vec<_> = self.tiles.iter().copied().map(Some).collect();
        for &tile in &movers {
            slots[tile] = None;
        }
        if let Some((tile, placed)) = anchor {
            if !self.fits(&slots, tile, placed) || border(main, placed, border_min).is_none() {
                return None;
            }
            slots[tile] = Some(placed);
        }
        let mut beam = vec![Candidate {
            tiles: slots,
            attached: 0,
            borders: 0,
            travel: 0.0,
        }];
        let min_size = Size2::new(self.smallest, self.smallest);
        for &tile in &movers {
            let mut next = Vec::new();
            for state in &beam {
                let old = self.tiles[tile];
                let from = center(old);
                let mut places = self.places(&state.tiles, tile, main, min_size, from);
                places.retain(|&rect| self.fits(&state.tiles, tile, rect));
                let mut places: Vec<_> = places
                    .into_iter()
                    .map(|rect| {
                        (
                            rect,
                            self.contacts(&state.tiles, tile, rect),
                            distance(rect, from),
                        )
                    })
                    .collect();
                places.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.2.total_cmp(&b.2)));
                places.truncate(CANDIDATES_PER_STATE);
                // The exact old rectangle is always a separate option. It is
                // the only placement an overflow leaf may end up with.
                if self.fits(&state.tiles, tile, old) {
                    places.push((old, self.contacts(&state.tiles, tile, old), 0.0));
                }
                for (rect, touches, travel) in places {
                    let mut candidate = state.clone();
                    candidate.tiles[tile] = Some(rect);
                    candidate.attached += usize::from(border(main, rect, border_min).is_some());
                    candidate.borders += touches;
                    candidate.travel += travel;
                    next.push(candidate);
                }
            }
            next.sort_by(Candidate::best_first);
            next.dedup_by(|a, b| a.tiles == b.tiles);
            next.truncate(BEAM_WIDTH);
            beam = next;
        }
        let mut best = beam.into_iter().next()?;
        if anchor.is_none() {
            // A bounded search can prune a path. Keep prior geometry if its
            // neighborhood already had more borders than the result.
            let before = movers
                .iter()
                .filter(|&&i| border(main, self.tiles[i], border_min).is_some())
                .count();
            let moving: BTreeSet<_> = movers.iter().copied().collect();
            let before_borders: usize = movers
                .iter()
                .map(|&tile| {
                    self.linked[tile]
                        .iter()
                        .filter(|&&other| {
                            (!moving.contains(&other) || tile < other)
                                && border(self.tiles[tile], self.tiles[other], border_min).is_some()
                        })
                        .count()
                })
                .sum();
            if (best.attached, best.borders) < (before, before_borders) {
                return Some(self.tiles.clone());
            }
        }
        for tile in movers {
            if best.tiles[tile] != Some(self.tiles[tile]) {
                self.grow(&mut best.tiles, tile, main);
            }
        }
        best.tiles.into_iter().collect()
    }

    fn fits(&self, slots: &[Option<Rect>], tile: usize, rect: Rect) -> bool {
        slots.iter().enumerate().all(|(other, slot)| {
            other == tile || slot.is_none_or(|obstacle| self.separated(tile, rect, other, obstacle))
        })
    }

    fn contacts(&self, slots: &[Option<Rect>], tile: usize, rect: Rect) -> usize {
        self.linked[tile]
            .iter()
            .filter(|&&other| {
                slots[other]
                    .is_some_and(|obstacle| border(rect, obstacle, self.smallest / 2.0).is_some())
            })
            .count()
    }

    fn places(
        &self,
        slots: &[Option<Rect>],
        tile: usize,
        main: Rect,
        size: Size2,
        from: Vec2,
    ) -> Vec<Rect> {
        let min = self.smallest / 2.0;
        let mut places = Vec::new();
        for side in SIDES {
            let (first, last) = side.interval(main, size, min);
            let span = side.span(size);
            let near = side.nearest(main, size, from, min);
            let mut events = vec![first, last, (first + last) / 2.0, side.along(near).0];
            for (index, obstacle) in slots.iter().enumerate() {
                let Some(obstacle) = obstacle else { continue };
                if index == tile {
                    continue;
                }
                let gap = if self.linked[tile].contains(&index) {
                    0.0
                } else {
                    CLEARANCE
                };
                let overlaps_slab = match side {
                    Side::Top | Side::Bottom => {
                        bottom(*obstacle) + gap >= near.origin.y
                            && obstacle.origin.y - gap <= bottom(near)
                    },
                    Side::Left | Side::Right => {
                        right(*obstacle) + gap >= near.origin.x
                            && obstacle.origin.x - gap <= right(near)
                    },
                };
                if overlaps_slab {
                    let (a, b) = side.along(*obstacle);
                    events.extend([a - span - gap, b + gap]);
                    if gap == 0.0 {
                        events.extend([a, b - span]);
                    }
                }
            }
            events.retain(|event| *event >= first && *event <= last);
            events.sort_by(f32::total_cmp);
            events.dedup_by(|a, b| (*a - *b).abs() <= EPSILON);
            places.extend(events.into_iter().map(|at| side.at(main, size, at)));
        }
        places
    }

    fn grow(&self, slots: &mut [Option<Rect>], tile: usize, main: Rect) {
        let Some(before) = slots[tile] else { return };
        let min = self.smallest / 2.0;
        let at = center(before);
        let Some(side) = SIDES.into_iter().find(|side| {
            distance(side.nearest(main, before.size, at, min), at) < f64::from(EPSILON)
                && border(main, before, min).is_some()
        }) else {
            return;
        };
        for fraction in [1.0, 0.75, 0.5, 0.25] {
            let size = Size2::new(
                before.size.w + (self.preferred.w.max(before.size.w) - before.size.w) * fraction,
                before.size.h + (self.preferred.h.max(before.size.h) - before.size.h) * fraction,
            );
            let bigger = side.nearest(main, size, at, min);
            if self.fits(slots, tile, bigger)
                && self.linked[tile].iter().all(|&other| {
                    slots[other].is_none_or(|rect| {
                        border(before, rect, min).is_none() || border(bigger, rect, min).is_some()
                    })
                })
            {
                slots[tile] = Some(bigger);
                return;
            }
        }
    }
}
