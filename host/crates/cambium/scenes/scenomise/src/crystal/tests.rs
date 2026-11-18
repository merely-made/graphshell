// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

use super::*;

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::new(Vec2::new(x, y), Size2::new(w, h))
}

fn parked(count: usize) -> Vec<Rect> {
    let mut tiles = vec![rect(0.0, 0.0, 160.0, 100.0)];
    tiles.extend((1..count).map(|i| {
        rect(
            300.0 + (i % 8) as f32 * 180.0,
            (i / 8) as f32 * 120.0,
            160.0,
            100.0,
        )
    }));
    tiles
}

fn assert_clear(layout: &Layout) {
    for (i, &a) in layout.tiles.iter().enumerate() {
        assert!(a.size.w >= layout.smallest && a.size.h >= layout.smallest);
        for (j, &b) in layout.tiles.iter().enumerate().skip(i + 1) {
            assert!(
                layout.separated(i, a, j, b),
                "overlap or false adjacent pair {i} {j}"
            );
        }
    }
}

#[test]
fn dense_star_uses_the_corners_and_leaves_overflow_at_its_drop() {
    let initial = parked(31);
    let mut layout = Layout::new(
        initial.clone(),
        (1..31).map(|i| (0, i)),
        BTreeSet::new(),
        48.0,
        Size2::new(160.0, 100.0),
    )
    .unwrap();
    layout.pin(0);
    assert_clear(&layout);
    let mut borders = 0;
    for (i, &tile) in layout.tiles.iter().enumerate().skip(1) {
        if border(tile, layout.tiles[0], 24.0).is_some() {
            borders += 1;
        } else {
            assert_eq!(tile, initial[i]);
        }
    }
    assert_eq!(layout.tiles[0], initial[0]);
    assert_eq!(borders, 10, "the earlier greedy probe fit nine");
    eprintln!("48 px star, {borders} measured borders");
}

#[test]
fn pinning_grows_one_neighborhood_and_unpinning_keeps_geometry() {
    let initial = parked(4);
    let mut layout = Layout::new(
        initial.clone(),
        [(0, 1), (1, 2)],
        BTreeSet::new(),
        32.0,
        Size2::new(160.0, 100.0),
    )
    .unwrap();
    assert!(layout.drop_at(1, Vec2::new(-230.0, 50.0)));
    assert_eq!(center(layout.tiles[1]), Vec2::new(-230.0, 50.0));
    assert!(!layout.drop_at(3, Vec2::new(240.0, 50.0)));
    layout.pin(0);
    assert_eq!(layout.tiles[2..], initial[2..]);
    assert!(border(layout.tiles[0], layout.tiles[1], 16.0).is_some());
    let previous = layout.tiles.clone();
    layout.pin(1);
    assert_eq!(layout.tiles[0..2], previous[0..2]);
    assert_eq!(layout.tiles[3], initial[3]);
    assert!(border(layout.tiles[1], layout.tiles[2], 16.0).is_some());
    assert_clear(&layout);
    let before = layout.tiles.clone();
    layout.unpin(0);
    layout.unpin(1);
    assert_eq!(layout.tiles, before);
}

#[test]
fn sliding_can_replace_an_occupied_slot_and_repack_only_siblings() {
    let original = vec![
        rect(0.0, 0.0, 160.0, 100.0),
        rect(0.0, -32.0, 32.0, 32.0),
        rect(48.0, -32.0, 32.0, 32.0),
        rect(200.0, 200.0, 32.0, 32.0),
    ];
    let mut layout = Layout::new(
        original.clone(),
        [(0, 1), (0, 2), (2, 3)],
        BTreeSet::from([0]),
        32.0,
        Size2::new(32.0, 32.0),
    )
    .unwrap();
    assert!(layout.drop_at(1, Vec2::new(64.125, -16.0)));
    assert_eq!(center(layout.tiles[1]), Vec2::new(64.125, -16.0));
    assert_ne!(layout.tiles[2], original[2]);
    assert_eq!(layout.tiles[3], original[3]);
    assert!(border(layout.tiles[2], layout.tiles[0], 16.0).is_some());
    assert_clear(&layout);
}
