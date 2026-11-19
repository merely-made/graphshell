// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! A reading's sort (table projection plan, T4): field keys in priority
//! order, each ascending or descending.
//!
//! The sort is a reading parameter and never changes the source. A missing
//! value sorts last in either direction, so flipping a column does not bring
//! its empty cells to the top.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::dataset::{ProjectionValue, compare_values};

/// One sort key: a dataset field and its direction.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SortKey {
    /// The field's path, the dataset key.
    pub field: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub descending: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}

impl SortKey {
    pub fn ascending(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            descending: false,
        }
    }

    pub fn descending(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            descending: true,
        }
    }
}

/// The sort after a click on `field`'s header: that field alone, ascending,
/// or flipped when it already leads the sort.
pub fn header_click(current: &[SortKey], field: &str) -> Vec<SortKey> {
    let descending = current
        .first()
        .is_some_and(|key| key.field == field && !key.descending);
    vec![SortKey {
        field: field.to_owned(),
        descending,
    }]
}

/// Order two values under one key; a missing value stays last.
fn compare_key(key: &SortKey, a: Option<&ProjectionValue>, b: Option<&ProjectionValue>) -> Ordering {
    match (a, b) {
        (Some(_), Some(_)) if key.descending => compare_values(b, a),
        _ => compare_values(a, b),
    }
}

/// Order two value maps by `keys`, in the dataset's per-type order.
pub fn compare(
    keys: &[SortKey],
    left: &BTreeMap<String, ProjectionValue>,
    right: &BTreeMap<String, ProjectionValue>,
) -> Ordering {
    keys.iter()
        .map(|key| compare_key(key, left.get(&key.field), right.get(&key.field)))
        .find(|order| order.is_ne())
        .unwrap_or(Ordering::Equal)
}

/// The permutation that sorts `rows` by `keys`, as indices into `rows`.
/// Stable: rows that tie keep their given order. Each row's keyed values
/// are looked up once, not once per comparison.
pub fn order<'a>(
    keys: &[SortKey],
    rows: impl IntoIterator<Item = &'a BTreeMap<String, ProjectionValue>>,
) -> Vec<usize> {
    let keyed: Vec<Vec<Option<&ProjectionValue>>> = rows
        .into_iter()
        .map(|values| keys.iter().map(|key| values.get(&key.field)).collect())
        .collect();
    let mut indices: Vec<usize> = (0..keyed.len()).collect();
    indices.sort_by(|&l, &r| {
        keys.iter()
            .enumerate()
            .map(|(at, key)| compare_key(key, keyed[l][at], keyed[r][at]))
            .find(|order| order.is_ne())
            .unwrap_or(Ordering::Equal)
    });
    indices
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(score: Option<f64>, name: &str) -> BTreeMap<String, ProjectionValue> {
        let mut values = BTreeMap::from([("name".into(), ProjectionValue::Text(name.into()))]);
        if let Some(score) = score {
            values.insert("score".into(), ProjectionValue::Number(score));
        }
        values
    }

    #[test]
    fn a_header_click_sorts_by_that_field_then_flips_it() {
        let first = header_click(&[], "score");
        assert_eq!(first, [SortKey::ascending("score")]);
        let second = header_click(&first, "score");
        assert_eq!(second, [SortKey::descending("score")]);
        assert_eq!(header_click(&second, "score"), first);
        assert_eq!(header_click(&second, "name"), [SortKey::ascending("name")]);
    }

    #[test]
    fn missing_values_sort_last_both_ways_and_later_keys_break_ties() {
        let rows = [
            row(Some(2.0), "b"),
            row(None, "z"),
            row(Some(2.0), "a"),
            row(Some(5.0), "c"),
        ];
        let order = |keys: &[SortKey]| {
            let mut indices: Vec<usize> = (0..rows.len()).collect();
            indices.sort_by(|&l, &r| compare(keys, &rows[l], &rows[r]));
            indices
        };
        let by_name = SortKey::ascending("name");
        assert_eq!(
            order(&[SortKey::ascending("score"), by_name.clone()]),
            [2, 0, 3, 1]
        );
        assert_eq!(
            order(&[SortKey::descending("score"), by_name]),
            [3, 2, 0, 1]
        );
    }

    #[test]
    fn order_agrees_with_compare_and_is_stable() {
        let rows = [
            row(Some(2.0), "b"),
            row(None, "z"),
            row(Some(2.0), "a"),
            row(Some(5.0), "c"),
        ];
        let keys = [SortKey::descending("score")];
        // The two 2.0 rows tie and keep their given order.
        assert_eq!(order(&keys, rows.iter()), [3, 0, 2, 1]);
        let mut by_compare: Vec<usize> = (0..rows.len()).collect();
        by_compare.sort_by(|&l, &r| compare(&keys, &rows[l], &rows[r]));
        assert_eq!(order(&keys, rows.iter()), by_compare);
    }

    #[test]
    fn an_ascending_key_writes_only_its_field() {
        let json = serde_json::to_string(&SortKey::ascending("score")).unwrap();
        assert_eq!(json, r#"{"field":"score"}"#);
        let back: SortKey = serde_json::from_str(r#"{"field":"score","descending":true}"#).unwrap();
        assert_eq!(back, SortKey::descending("score"));
    }
}
