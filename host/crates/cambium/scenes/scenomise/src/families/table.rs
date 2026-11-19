// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The table family: each item in reading order takes the next row, and the
//! row's rectangle is its assigned footprint.

use sceno::{Footprint, ScoreItem, Table, Vec2};

use super::Shape;

pub(crate) fn shape(config: &Table, items: &[&ScoreItem]) -> Shape {
    let mut shape = Shape::unplaced(items.len());
    for rank in 0..items.len() {
        let row = config.row_rect(rank);
        // Footprints are centred on the item's position.
        shape.positions[rank] = Vec2::new(
            row.origin.x + row.size.w / 2.0,
            row.origin.y + row.size.h / 2.0,
        );
        shape.footprints[rank] = Some(Footprint::Rect { size: row.size });
    }
    shape
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::families::tests::{card, items};
    use sceno::Size2;

    #[test]
    fn rows_stack_in_reading_order_at_the_full_width() {
        let owned: Vec<_> = (0..3).map(card).collect();
        let config = Table {
            origin: Vec2::new(10.0, 0.0),
            columns: vec![100.0, 60.0],
            row_height: 20.0,
            header: 30.0,
        };
        let shape = shape(&config, &items(&owned));
        assert_eq!(
            shape.positions,
            [
                Vec2::new(90.0, 40.0),
                Vec2::new(90.0, 60.0),
                Vec2::new(90.0, 80.0)
            ]
        );
        let row = Footprint::Rect {
            size: Size2::new(160.0, 20.0),
        };
        assert!(shape.footprints.iter().all(|f| f.as_ref() == Some(&row)));
        assert!(shape.regions.is_empty());
    }
}
