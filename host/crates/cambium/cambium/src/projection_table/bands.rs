// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The table's bands: what the grid draws top to bottom, the group headings
//! and the rows, with a collapsed group's rows and inner headings left out
//! (table D25, D29). A heading is not an instance; collapsing one is view
//! state, saved beside the selection and never an undo step.

use scenograph::table::CollapsedGroups;

use super::ProjectionTable;

/// One band the grid draws, by index into the table's rows or groups.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Band {
    Row(u32),
    Heading(u32),
}

/// A heading as the grid draws it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableHeadingView {
    /// The group's index, for collapsing it.
    pub group: usize,
    /// The group's path as JSON: the same group across a collapse, a filter
    /// or a regrouping that keeps it.
    pub key: String,
    /// Zero for the outermost key.
    pub level: usize,
    /// "Shelf: Fiction".
    pub label: String,
    pub count: usize,
    pub collapsed: bool,
}

/// What lies at one band of the body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TableBandView {
    /// A row, by its rank in the rows read.
    Row(usize),
    Heading(TableHeadingView),
}

impl ProjectionTable {
    /// Lay the bands out from the rows, the groups and the collapsed set,
    /// and keep the focus on a band that still exists.
    pub(super) fn rebuild_bands(&mut self) {
        let mut bands = Vec::with_capacity(self.rows.len() + self.groups.len());
        // Rows before this rank are hidden by a collapsed group.
        let mut hidden_until = 0;
        let mut next_row = 0;
        let push_rows = |bands: &mut Vec<Band>, from: usize, to: usize, hidden: usize| {
            bands.extend((from.max(hidden)..to.max(hidden)).map(|rank| Band::Row(rank as u32)));
        };
        for (index, group) in self.groups.iter().enumerate() {
            push_rows(&mut bands, next_row, group.start, hidden_until);
            next_row = group.start;
            if group.start < hidden_until {
                continue;
            }
            bands.push(Band::Heading(index as u32));
            if self.collapsed.is_collapsed(&group.path) {
                hidden_until = group.start + group.count;
            }
        }
        push_rows(&mut bands, next_row, self.rows.len(), hidden_until);
        self.bands = bands;
        if self.focus.row.is_some_and(|row| row >= self.bands.len()) {
            self.focus.row = self.bands.len().checked_sub(1);
        }
        self.scroll = self.scroll.clamp(0.0, self.max_scroll());
    }

    /// How many bands the body holds: visible headings and rows.
    pub fn band_count(&self) -> usize {
        self.bands.len()
    }

    /// The band at `index` from the top of the body.
    pub fn band(&self, index: usize) -> Option<TableBandView> {
        Some(match *self.bands.get(index)? {
            Band::Row(rank) => TableBandView::Row(rank as usize),
            Band::Heading(group) => {
                let group = group as usize;
                let band = &self.groups[group];
                TableBandView::Heading(TableHeadingView {
                    group,
                    key: serde_json::to_string(&band.path).unwrap_or_default(),
                    level: band.level,
                    label: self.heading_label(group),
                    count: band.count,
                    collapsed: self.collapsed.is_collapsed(&band.path),
                })
            },
        })
    }

    /// The row rank at band `index`, when it is a row.
    pub fn band_row(&self, index: usize) -> Option<usize> {
        match self.bands.get(index)? {
            Band::Row(rank) => Some(*rank as usize),
            Band::Heading(_) => None,
        }
    }

    /// Group `group`'s heading in words, named by its field's column.
    pub fn heading_label(&self, group: usize) -> String {
        let Some(band) = self.groups.get(group) else {
            return String::new();
        };
        let field = band.path.0.last().map_or("", |step| step.field.as_str());
        let name = self
            .columns
            .iter()
            .find(|column| column.field == field)
            .map_or(field, |column| column.name.as_str());
        band.label(name)
    }

    /// How many groups the reading has, at every level.
    pub fn group_count(&self) -> usize {
        self.groups.len()
    }

    /// Collapse group `group`, or expand it. View state: the recipe and the
    /// undo history do not change.
    pub fn toggle_group(&mut self, group: usize) {
        if let Some(band) = self.groups.get(group) {
            self.collapsed.toggle(band.path.clone());
            self.rebuild_bands();
        }
    }

    /// Collapse or expand the heading at band `index`; `false` when the band
    /// is a row.
    pub fn toggle_band(&mut self, index: usize) -> bool {
        match self.bands.get(index) {
            Some(Band::Heading(group)) => {
                self.toggle_group(*group as usize);
                true
            },
            _ => false,
        }
    }

    /// The collapsed groups, to save beside the selection.
    pub fn collapsed(&self) -> &CollapsedGroups {
        &self.collapsed
    }

    /// Restore saved collapsed groups; a group no longer read is kept, so
    /// it collapses again if a later reading brings it back.
    pub fn set_collapsed(&mut self, collapsed: CollapsedGroups) {
        self.collapsed = collapsed;
        self.rebuild_bands();
    }
}
