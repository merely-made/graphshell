// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! A table's selection, saved as row and field (table projection plan, T3).
//!
//! A row is named by its occurrence id and a column by its field's path, never
//! by position, so a selection survives a sort, a filter that keeps the row,
//! and a reordered fields list. A cell is a row and a field (table D1).

use serde::{Deserialize, Serialize};

/// One selected row, or one cell of it when `field` is set.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TableTarget {
    /// The row's occurrence id.
    pub row: String,
    /// The cell's field path; absent selects the whole row.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}

impl TableTarget {
    pub fn row(row: impl Into<String>) -> Self {
        Self {
            row: row.into(),
            field: None,
        }
    }

    pub fn cell(row: impl Into<String>, field: impl Into<String>) -> Self {
        Self {
            row: row.into(),
            field: Some(field.into()),
        }
    }

    pub fn is_row(&self) -> bool {
        self.field.is_none()
    }
}

/// Selected rows and cells, in the order they were selected. Empty is a
/// cleared selection.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TableSelection {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub targets: Vec<TableTarget>,
}

impl TableSelection {
    pub fn one(target: TableTarget) -> Self {
        Self {
            targets: vec![target],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.targets.is_empty()
    }

    pub fn contains(&self, target: &TableTarget) -> bool {
        self.targets.contains(target)
    }

    /// Whether the row is selected, as a whole or through one of its cells.
    pub fn touches_row(&self, row: &str) -> bool {
        self.targets.iter().any(|target| target.row == row)
    }

    /// Add `target`, or take it out when it is already selected.
    pub fn toggle(&mut self, target: TableTarget) {
        match self.targets.iter().position(|held| *held == target) {
            Some(index) => {
                self.targets.remove(index);
            },
            None => self.targets.push(target),
        }
    }

    /// Keep only targets whose row is still read and whose field is still
    /// listed, returning what was dropped so a host can say so.
    pub fn retain_present(
        &mut self,
        row_present: impl Fn(&str) -> bool,
        field_listed: impl Fn(&str) -> bool,
    ) -> Vec<TableTarget> {
        let (kept, dropped) =
            std::mem::take(&mut self.targets)
                .into_iter()
                .partition(|target: &TableTarget| {
                    row_present(&target.row) && target.field.as_deref().is_none_or(&field_listed)
                });
        self.targets = kept;
        dropped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_selection_is_saved_as_row_and_field() {
        let mut selection = TableSelection::one(TableTarget::row("a"));
        selection.toggle(TableTarget::cell("b", "title"));
        let json = serde_json::to_value(&selection).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "targets": [
                { "row": "a" },
                { "row": "b", "field": "title" },
            ]})
        );
        let back: TableSelection = serde_json::from_value(json).unwrap();
        assert_eq!(back, selection);
        assert_eq!(
            serde_json::to_value(TableSelection::default()).unwrap(),
            serde_json::json!({})
        );
    }

    #[test]
    fn toggling_a_held_target_takes_it_out() {
        let mut selection = TableSelection::default();
        selection.toggle(TableTarget::cell("a", "x"));
        selection.toggle(TableTarget::row("a"));
        assert!(selection.touches_row("a"));
        selection.toggle(TableTarget::cell("a", "x"));
        assert_eq!(selection.targets, [TableTarget::row("a")]);
    }

    #[test]
    fn rows_no_longer_read_and_fields_no_longer_listed_are_dropped() {
        let mut selection = TableSelection {
            targets: vec![
                TableTarget::row("a"),
                TableTarget::cell("a", "gone"),
                TableTarget::cell("b", "x"),
                TableTarget::row("c"),
            ],
        };
        let dropped = selection.retain_present(|row| row != "c", |field| field == "x");
        assert_eq!(
            selection.targets,
            [TableTarget::row("a"), TableTarget::cell("b", "x")]
        );
        assert_eq!(
            dropped,
            [TableTarget::cell("a", "gone"), TableTarget::row("c")]
        );
    }
}
