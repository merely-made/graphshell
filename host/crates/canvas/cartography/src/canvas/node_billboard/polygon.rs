// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

use euclid::default::{Point2D, Vector2D};

// Silhouette::outline supplies convex, consistently wound unit-box polygons.
fn edges(outline: &[(f32, f32)]) -> impl Iterator<Item = (Point2D<f64>, Vector2D<f64>)> + '_ {
    outline
        .iter()
        .zip(outline.iter().cycle().skip(1))
        .map(|(&(x, y), &(next_x, next_y))| {
            let start = Point2D::new(f64::from(x), f64::from(y));
            let end = Point2D::new(f64::from(next_x), f64::from(next_y));
            (start, end - start)
        })
}

fn local(point: Point2D<f32>, center: Point2D<f32>, side: f32) -> Point2D<f64> {
    let side = f64::from(side);
    Point2D::new(
        (f64::from(point.x) - f64::from(center.x)) / side,
        (f64::from(point.y) - f64::from(center.y)) / side,
    )
}

pub(super) fn contains(
    outline: &[(f32, f32)],
    center: Point2D<f32>,
    side: f32,
    point: Point2D<f32>,
) -> bool {
    let point = local(point, center, side);
    edges(outline).all(|(start, edge)| edge.cross(point - start) >= 0.0)
}

pub(super) fn signed_distance(
    outline: &[(f32, f32)],
    center: Point2D<f32>,
    side: f32,
    point: Point2D<f32>,
) -> Option<f32> {
    let point = local(point, center, side);
    let mut inside = true;
    let mut nearest = f64::INFINITY;
    for (start, edge) in edges(outline) {
        let offset = point - start;
        inside &= edge.cross(offset) >= 0.0;
        let along = (offset.dot(edge) / edge.square_length()).clamp(0.0, 1.0);
        nearest = nearest.min((offset - edge * along).square_length());
    }
    let pixels = (nearest.sqrt() * f64::from(side)) as f32;
    pixels
        .is_finite()
        .then_some(if inside { -pixels } else { pixels })
}

pub(super) fn segment_interval(
    outline: &[(f32, f32)],
    center: Point2D<f32>,
    side: f32,
    a: Point2D<f32>,
    b: Point2D<f32>,
) -> Option<(f32, f32)> {
    let a = local(a, center, side);
    let b = local(b, center, side);
    let mut enter: f64 = 0.0;
    let mut exit: f64 = 1.0;
    for (start, edge) in edges(outline) {
        let from = edge.cross(a - start);
        let to = edge.cross(b - start);
        match (from < 0.0, to < 0.0) {
            (true, true) => return None,
            (true, false) => enter = enter.max(from / (from - to)),
            (false, true) => exit = exit.min(from / (from - to)),
            (false, false) => {},
        }
        if enter > exit {
            return None;
        }
    }
    Some((enter as f32, exit as f32))
}
