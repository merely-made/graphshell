// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The table arrangement (table projection plan, T3): one row per item in
//! reading order, one column per field. A row is the instance; a cell is a
//! row and a column, never an instance of its own (table D1).

use serde::{Deserialize, Serialize};

use crate::{Rect, Size2, Vec2};

/// Rows stacked top to bottom under a header band, every row the full width
/// of its columns. Rows touch: the pitch is the row height.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Table {
    /// The header band's top-left corner.
    pub origin: Vec2,
    /// Column widths in the fields' order.
    pub columns: Vec<f32>,
    pub row_height: f32,
    /// The header band's height; zero leaves no band.
    pub header: f32,
}

impl Default for Table {
    fn default() -> Self {
        Self {
            origin: Vec2::ZERO,
            columns: vec![160.0],
            row_height: 32.0,
            header: 32.0,
        }
    }
}

impl Table {
    /// The width of every row.
    pub fn width(&self) -> f32 {
        self.columns.iter().sum()
    }

    /// Column `index`'s left edge, measured from the row's left edge.
    pub fn column_x(&self, index: usize) -> f32 {
        self.columns.iter().take(index).sum()
    }

    /// The column under `x`, measured from the row's left edge.
    pub fn column_at(&self, x: f32) -> Option<usize> {
        if !x.is_finite() || x < 0.0 {
            return None;
        }
        let mut left = 0.0;
        for (index, width) in self.columns.iter().enumerate() {
            if x < left + width {
                return Some(index);
            }
            left += width;
        }
        None
    }

    /// The header band.
    pub fn header_rect(&self) -> Rect {
        Rect::new(self.origin, Size2::new(self.width(), self.header))
    }

    /// The rectangle of the row at reading position `rank`.
    pub fn row_rect(&self, rank: usize) -> Rect {
        Rect::new(
            Vec2::new(
                self.origin.x,
                self.origin.y + self.header + rank as f32 * self.row_height,
            ),
            Size2::new(self.width(), self.row_height),
        )
    }

    /// The rectangle of `column` in a row rectangle.
    pub fn cell_rect(&self, row: Rect, column: usize) -> Option<Rect> {
        let width = *self.columns.get(column)?;
        Some(Rect::new(
            Vec2::new(row.origin.x + self.column_x(column), row.origin.y),
            Size2::new(width, row.size.h),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> Table {
        Table {
            origin: Vec2::new(10.0, 20.0),
            columns: vec![100.0, 50.0, 30.0],
            row_height: 24.0,
            header: 28.0,
        }
    }

    #[test]
    fn columns_are_prefix_sums_of_their_widths() {
        let table = table();
        assert_eq!(table.width(), 180.0);
        assert_eq!(table.column_x(2), 150.0);
        assert_eq!(table.column_at(0.0), Some(0));
        assert_eq!(table.column_at(99.9), Some(0));
        assert_eq!(table.column_at(100.0), Some(1));
        assert_eq!(table.column_at(179.0), Some(2));
        assert_eq!(table.column_at(180.0), None);
        assert_eq!(table.column_at(-1.0), None);
    }

    #[test]
    fn rows_stack_under_the_header_band() {
        let table = table();
        assert_eq!(
            table.header_rect(),
            Rect::new(Vec2::new(10.0, 20.0), Size2::new(180.0, 28.0))
        );
        assert_eq!(
            table.row_rect(2),
            Rect::new(Vec2::new(10.0, 96.0), Size2::new(180.0, 24.0))
        );
        assert_eq!(
            table.cell_rect(table.row_rect(0), 1),
            Some(Rect::new(Vec2::new(110.0, 48.0), Size2::new(50.0, 24.0)))
        );
        assert_eq!(table.cell_rect(table.row_rect(0), 3), None);
    }
}
