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

use crate::group::GroupPath;

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

/// Which groups are collapsed (table D29): view state saved beside the
/// selection, keyed by group value, never in the recipe and never an undo
/// step. A collapsed group keeps its heading and hides its rows and the
/// groups inside it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CollapsedGroups {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<GroupPath>,
}

impl CollapsedGroups {
    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }

    pub fn is_collapsed(&self, path: &GroupPath) -> bool {
        self.groups.iter().any(|held| held.same(path))
    }

    /// Whether a group lies inside a collapsed one, so its heading is hidden.
    pub fn hides(&self, path: &GroupPath) -> bool {
        self.groups
            .iter()
            .any(|held| held.0.len() < path.0.len() && path.within(held))
    }

    /// Collapse a group, or expand it when it is collapsed.
    pub fn toggle(&mut self, path: GroupPath) {
        match self.groups.iter().position(|held| held.same(&path)) {
            Some(index) => {
                self.groups.remove(index);
            },
            None => self.groups.push(path),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataset::ProjectionValue;
    use crate::group::GroupStep;

    #[test]
    fn collapsed_groups_are_saved_by_value_and_hide_what_they_hold() {
        let step = |field: &str, value: Option<&str>| GroupStep {
            field: field.into(),
            value: value.map(|v| ProjectionValue::Text(v.into())),
        };
        let shelf = GroupPath(vec![step("shelf", Some("art"))]);
        let inner = GroupPath(vec![step("shelf", Some("art")), step("year", None)]);
        let mut collapsed = CollapsedGroups::default();
        collapsed.toggle(shelf.clone());
        assert!(collapsed.is_collapsed(&shelf) && !collapsed.is_collapsed(&inner));
        assert!(collapsed.hides(&inner) && !collapsed.hides(&shelf));
        let json = serde_json::to_string(&collapsed).unwrap();
        assert_eq!(
            json,
            r#"{"groups":[[{"field":"shelf","value":{"kind":"text","value":"art"}}]]}"#
        );
        let back: CollapsedGroups = serde_json::from_str(&json).unwrap();
        assert_eq!(back, collapsed);
        collapsed.toggle(shelf);
        assert!(collapsed.is_empty());
        assert_eq!(serde_json::to_string(&collapsed).unwrap(), "{}");
    }

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
