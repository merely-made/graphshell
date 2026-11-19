// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Keyboard reach: every header and cell, by the ARIA grid pattern. One tab
//! stop; arrows move it, Home and End go to the row's ends (with Control, to
//! the table's corners), Page Up and Page Down move a body's height. The
//! header row sits above the first body row. The body scrolls to keep the
//! focused row in view.

use crate::grid::GridFocus;
use crate::{Key, NamedKey};

use super::ProjectionTable;

impl ProjectionTable {
    /// Move the focus for `key`; `false` when the key is not the grid's.
    pub fn navigate(&mut self, key: &Key, control: bool) -> bool {
        let Key::Named(named) = key else {
            return false;
        };
        let last_row = self.rows.len().checked_sub(1);
        let last_col = self.columns.len().saturating_sub(1);
        let page = ((self.viewport_height / self.row_height.max(1.0)).floor() as usize).max(1);
        let GridFocus { row, col, .. } = self.focus;
        let (row, col) = match named {
            NamedKey::ArrowLeft => (row, col.saturating_sub(1)),
            NamedKey::ArrowRight => (row, (col + 1).min(last_col)),
            NamedKey::ArrowUp => (row.and_then(|r| r.checked_sub(1)), col),
            NamedKey::ArrowDown => match (row, last_row) {
                (_, None) => (None, col),
                (None, Some(_)) => (Some(0), col),
                (Some(r), Some(last)) => (Some((r + 1).min(last)), col),
            },
            NamedKey::Home if control => (None, 0),
            NamedKey::End if control => (last_row, last_col),
            NamedKey::Home => (row, 0),
            NamedKey::End => (row, last_col),
            NamedKey::PageUp => (row.and_then(|r| r.checked_sub(page).or(Some(0))), col),
            NamedKey::PageDown => match (row, last_row) {
                (_, None) => (None, col),
                (r, Some(last)) => (Some(r.map_or(page - 1, |r| r + page).min(last)), col),
            },
            _ => return false,
        };
        self.focus = GridFocus {
            row,
            col,
            requested: true,
        };
        self.reveal_focus();
        true
    }

    /// Put the focus on a cell, as a click on it does.
    pub fn focus_cell(&mut self, row: Option<usize>, col: usize) {
        self.focus = GridFocus {
            row: row.filter(|r| *r < self.rows.len()),
            col: col.min(self.columns.len().saturating_sub(1)),
            requested: true,
        };
        self.reveal_focus();
    }

    /// Scroll the least that shows the focused row whole.
    fn reveal_focus(&mut self) {
        let Some(row) = self.focus.row else {
            return;
        };
        let top = row as f32 * self.row_height;
        let bottom = top + self.row_height;
        if top < self.scroll {
            self.scroll = top;
        } else if bottom > self.scroll + self.viewport_height {
            self.scroll = bottom - self.viewport_height;
        }
        self.scroll = self.scroll.clamp(0.0, self.max_scroll());
    }
}
