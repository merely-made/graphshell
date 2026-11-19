// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! A reading's grouping (table projection plan, T5): fields in priority
//! order, one heading level each (table D24), saved as a sort is.
//!
//! Groups follow the field's own order, ascending or descending, and the rows
//! with no value form a last group named "No value" (D27). A list groups by
//! its whole value and a date by its exact value, so each row is in one group
//! (D28). Within a group the rows keep the sort's order: grouping is the group
//! keys leading the sort keys, then a heading wherever a group starts.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::SortKey;
use crate::dataset::{ProjectionValue, compare_values, value_words};

/// What the group of rows with no value is called (D27).
pub const NO_VALUE: &str = "No value";

/// One level of a group's identity: the field and the value its rows share,
/// absent for the "No value" group.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupStep {
    pub field: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<ProjectionValue>,
}

/// A group named by its values from the outermost level in, so a collapsed
/// group is found again after a sort or a filter.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GroupPath(pub Vec<GroupStep>);

impl GroupPath {
    /// The same group: the same fields, and values the dataset orders as
    /// equal at every level.
    pub fn same(&self, other: &Self) -> bool {
        self.0.len() == other.0.len()
            && self.0.iter().zip(&other.0).all(|(a, b)| {
                a.field == b.field
                    && compare_values(a.value.as_ref(), b.value.as_ref()) == Ordering::Equal
            })
    }

    /// Whether `self` is `other` or lies inside it.
    pub fn within(&self, other: &Self) -> bool {
        self.0.len() >= other.0.len() && GroupPath(self.0[..other.0.len()].to_vec()).same(other)
    }
}

/// One heading: its level, where its rows start in the grouped order, how
/// many rows it holds and which group it is.
#[derive(Clone, Debug, PartialEq)]
pub struct GroupBand {
    /// Zero for the outermost key.
    pub level: usize,
    /// The rank of the group's first row.
    pub start: usize,
    pub count: usize,
    pub path: GroupPath,
}

impl GroupBand {
    pub fn value(&self) -> Option<&ProjectionValue> {
        self.path.0.last().and_then(|step| step.value.as_ref())
    }

    /// The group's value in words: "No value" when it has none.
    pub fn value_words(&self) -> String {
        match self.value() {
            None => NO_VALUE.to_owned(),
            Some(value) => {
                let words = value_words(value);
                if words.is_empty() {
                    "empty".to_owned()
                } else {
                    words
                }
            },
        }
    }

    /// The heading's words, with the field called `name`: "Shelf: Fiction".
    pub fn label(&self, name: &str) -> String {
        format!("{name}: {}", self.value_words())
    }
}

/// The sort a grouped reading orders by: the group keys, then the sort's.
/// A field in both is ordered by its group key.
pub fn order_keys(group: &[SortKey], sort: &[SortKey]) -> Vec<SortKey> {
    group
        .iter()
        .chain(
            sort.iter()
                .filter(|key| !group.iter().any(|g| g.field == key.field)),
        )
        .cloned()
        .collect()
}

/// The headings of `rows`, which are already in [`order_keys`] order: one
/// band per group per level, in the order they are read, outer before inner.
pub fn bands<'a>(
    keys: &[SortKey],
    rows: impl IntoIterator<Item = &'a BTreeMap<String, ProjectionValue>>,
) -> Vec<GroupBand> {
    let mut bands: Vec<GroupBand> = Vec::new();
    // The open band at each level, as an index into `bands`.
    let mut open: Vec<usize> = Vec::with_capacity(keys.len());
    let mut previous: Option<&BTreeMap<String, ProjectionValue>> = None;
    for (rank, values) in rows.into_iter().enumerate() {
        let first_change = match previous {
            None => 0,
            Some(before) => keys
                .iter()
                .position(|key| {
                    compare_values(before.get(&key.field), values.get(&key.field)).is_ne()
                })
                .unwrap_or(keys.len()),
        };
        open.truncate(first_change);
        for (level, key) in keys.iter().enumerate().skip(first_change) {
            let mut path = match open.last() {
                Some(&outer) => bands[outer].path.clone(),
                None => GroupPath::default(),
            };
            let field = &key.field;
            path.0.push(GroupStep {
                field: field.clone(),
                value: values.get(field).cloned(),
            });
            open.push(bands.len());
            bands.push(GroupBand {
                level,
                start: rank,
                count: 0,
                path,
            });
        }
        for &band in &open {
            bands[band].count += 1;
        }
        previous = Some(values);
    }
    bands
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(shelf: Option<&str>, year: Option<f64>) -> BTreeMap<String, ProjectionValue> {
        let mut values = BTreeMap::new();
        if let Some(shelf) = shelf {
            values.insert("shelf".into(), ProjectionValue::Text(shelf.into()));
        }
        if let Some(year) = year {
            values.insert("year".into(), ProjectionValue::Number(year));
        }
        values
    }

    #[test]
    fn two_levels_draw_outer_then_inner_headings_with_counts() {
        let rows = [
            row(Some("art"), Some(1.0)),
            row(Some("art"), Some(1.0)),
            row(Some("art"), Some(2.0)),
            row(Some("sea"), Some(2.0)),
            row(None, Some(2.0)),
        ];
        let keys = [SortKey::ascending("shelf"), SortKey::ascending("year")];
        let found = bands(&keys, rows.iter());
        let summary: Vec<(usize, usize, usize, String)> = found
            .iter()
            .map(|band| (band.level, band.start, band.count, band.value_words()))
            .collect();
        assert_eq!(
            summary,
            [
                (0, 0, 3, "art".into()),
                (1, 0, 2, "1".into()),
                (1, 2, 1, "2".into()),
                (0, 3, 1, "sea".into()),
                (1, 3, 1, "2".into()),
                (0, 4, 1, NO_VALUE.into()),
                (1, 4, 1, "2".into()),
            ]
        );
        assert_eq!(found[5].label("Shelf"), "Shelf: No value");
        assert!(found[6].path.within(&found[5].path));
        assert!(!found[4].path.within(&found[5].path));
    }

    #[test]
    fn group_keys_lead_and_a_field_in_both_is_ordered_once() {
        let keys = order_keys(
            &[SortKey::descending("shelf")],
            &[SortKey::ascending("shelf"), SortKey::ascending("title")],
        );
        assert_eq!(
            keys,
            [SortKey::descending("shelf"), SortKey::ascending("title")]
        );
    }

    #[test]
    fn a_path_is_saved_as_field_and_value_and_matched_by_the_dataset_order() {
        let path = GroupPath(vec![GroupStep {
            field: "at".into(),
            value: Some(ProjectionValue::DateTime("2024-01-01T12:00:00Z".into())),
        }]);
        let json = serde_json::to_value(&path).unwrap();
        assert_eq!(
            json,
            serde_json::json!([{ "field": "at", "value": { "kind": "date_time", "value": "2024-01-01T12:00:00Z" } }])
        );
        let same_instant = GroupPath(vec![GroupStep {
            field: "at".into(),
            value: Some(ProjectionValue::DateTime(
                "2024-01-01T13:00:00+01:00".into(),
            )),
        }]);
        assert!(path.same(&same_instant));
    }
}
