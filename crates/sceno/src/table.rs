// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The table arrangement (table projection plan, T3): one row per item in
//! reading order, one column per field. A row is the instance; a cell is a
//! row and a column, never an instance of its own (table D1).
//!
//! A grouped table also carries its group headings as bands (table D25):
//! each heading takes one row's height before the first row of its group,
//! and the rows below it move down past it. A heading is not an instance.

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
    /// Group headings in reading order, outer before inner where several
    /// start at one row; empty for an ungrouped table.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub headings: Vec<TableHeading>,
}

/// One group heading: a band one row high before the group's first row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableHeading {
    /// The reading position of the group's first row.
    pub position: usize,
    /// Zero for the outermost grouping key.
    #[serde(default)]
    pub level: usize,
    pub label: String,
    /// How many rows the group holds.
    pub count: usize,
}

/// What a band of the table is: a row, by reading position, or a heading,
/// by its index in [`Table::headings`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableBand {
    Row(usize),
    Heading(usize),
}

impl Default for Table {
    fn default() -> Self {
        Self {
            origin: Vec2::ZERO,
            columns: vec![160.0],
            row_height: 32.0,
            header: 32.0,
            headings: Vec::new(),
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

    /// The band slot under the header, counting headings and rows alike.
    fn slot_rect(&self, slot: usize) -> Rect {
        Rect::new(
            Vec2::new(
                self.origin.x,
                self.origin.y + self.header + slot as f32 * self.row_height,
            ),
            Size2::new(self.width(), self.row_height),
        )
    }

    /// The rectangle of the row at reading position `rank`, below every
    /// heading that starts at or before it.
    pub fn row_rect(&self, rank: usize) -> Rect {
        let above = self.headings.partition_point(|h| h.position <= rank);
        self.slot_rect(rank + above)
    }

    /// The rectangle of heading `index`.
    pub fn heading_rect(&self, index: usize) -> Option<Rect> {
        let heading = self.headings.get(index)?;
        Some(self.slot_rect(heading.position + index))
    }

    /// How many bands lie under the header: every row and every heading.
    pub fn band_count(&self, rows: usize) -> usize {
        rows + self.headings.len()
    }

    /// The band in slot `slot` under the header, given the table's `rows`.
    pub fn band(&self, slot: usize, rows: usize) -> Option<TableBand> {
        // The rows before `slot` are `slot` less the headings before them;
        // headings sit before the row at their position.
        let (mut low, mut high) = (0, self.headings.len());
        while low < high {
            let mid = (low + high) / 2;
            if self.headings[mid].position + mid < slot {
                low = mid + 1;
            } else {
                high = mid;
            }
        }
        if let Some(heading) = self.headings.get(low)
            && heading.position + low == slot
        {
            return Some(TableBand::Heading(low));
        }
        let rank = slot - low;
        (rank < rows).then_some(TableBand::Row(rank))
    }

    /// The band under `y`, in the table's own space, or `None` above the
    /// rows or past the last.
    pub fn band_at(&self, y: f32, rows: usize) -> Option<TableBand> {
        let below = y - self.origin.y - self.header;
        if !below.is_finite() || below < 0.0 || self.row_height <= 0.0 {
            return None;
        }
        self.band((below / self.row_height) as usize, rows)
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
            headings: Vec::new(),
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

    fn heading(position: usize, level: usize) -> TableHeading {
        TableHeading {
            position,
            level,
            label: format!("group at {position}"),
            count: 1,
        }
    }

    #[test]
    fn rows_skip_past_the_headings_of_their_groups() {
        // Two levels at row 0, one inner heading at row 2, an outer at 3.
        let table = Table {
            headings: vec![heading(0, 0), heading(0, 1), heading(2, 1), heading(3, 0)],
            ..table()
        };
        let slot_y = |slot: usize| 20.0 + 28.0 + slot as f32 * 24.0;
        assert_eq!(table.heading_rect(1).unwrap().origin.y, slot_y(1));
        assert_eq!(table.row_rect(0).origin.y, slot_y(2));
        assert_eq!(table.row_rect(1).origin.y, slot_y(3));
        assert_eq!(table.heading_rect(2).unwrap().origin.y, slot_y(4));
        assert_eq!(table.row_rect(2).origin.y, slot_y(5));
        assert_eq!(table.heading_rect(3).unwrap().origin.y, slot_y(6));
        assert_eq!(table.row_rect(3).origin.y, slot_y(7));
        let bands: Vec<_> = (0..table.band_count(4))
            .map(|slot| table.band(slot, 4).unwrap())
            .collect();
        use TableBand::{Heading, Row};
        assert_eq!(
            bands,
            [
                Heading(0),
                Heading(1),
                Row(0),
                Row(1),
                Heading(2),
                Row(2),
                Heading(3),
                Row(3)
            ]
        );
        assert_eq!(table.band(8, 4), None);
        assert_eq!(table.band_at(slot_y(4) + 1.0, 4), Some(Heading(2)));
        assert_eq!(table.band_at(slot_y(0) - 1.0, 4), None);
        // Each band's rectangle is where the band lookup finds it.
        for (slot, band) in bands.iter().enumerate() {
            let rect = match band {
                Row(rank) => table.row_rect(*rank),
                Heading(index) => table.heading_rect(*index).unwrap(),
            };
            assert_eq!(rect.origin.y, slot_y(slot));
        }
    }
}
