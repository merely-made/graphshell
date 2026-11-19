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
use crate::temporal;

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
fn compare_key(
    key: &SortKey,
    a: Option<&ProjectionValue>,
    b: Option<&ProjectionValue>,
) -> Ordering {
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

/// The sort after a Shift+click on `field`'s header (table D26): the field
/// added as the next key, ascending, or flipped where it already is a key.
pub fn header_add(current: &[SortKey], field: &str) -> Vec<SortKey> {
    let mut keys = current.to_vec();
    match keys.iter_mut().find(|key| key.field == field) {
        Some(key) => key.descending = !key.descending,
        None => keys.push(SortKey::ascending(field)),
    }
    keys
}

/// A row's value under one key, with a date or date-time parsed once before
/// sorting rather than on every comparison. Orders as [`compare_values`].
struct Prepared<'a> {
    value: Option<&'a ProjectionValue>,
    parsed: Parsed,
}

enum Parsed {
    No,
    Date(Option<temporal::Date>),
    Instant(Option<temporal::Instant>),
}

impl<'a> Prepared<'a> {
    fn of(value: Option<&'a ProjectionValue>) -> Self {
        let parsed = match value {
            Some(ProjectionValue::Date(text)) => Parsed::Date(temporal::parse_date(text).ok()),
            Some(ProjectionValue::DateTime(text)) => {
                Parsed::Instant(temporal::parse_date_time(text).ok())
            },
            _ => Parsed::No,
        };
        Self { value, parsed }
    }

    /// Well-formed values in their parsed order, then malformed ones by text.
    fn order(&self, other: &Self) -> Ordering {
        fn parsed<T: Ord>(
            a: &Option<T>,
            b: &Option<T>,
            text: impl FnOnce() -> Ordering,
        ) -> Ordering {
            match (a, b) {
                (Some(x), Some(y)) => x.cmp(y),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => text(),
            }
        }
        let text = || match (self.value, other.value) {
            (
                Some(ProjectionValue::Date(a) | ProjectionValue::DateTime(a)),
                Some(ProjectionValue::Date(b) | ProjectionValue::DateTime(b)),
            ) => a.cmp(b),
            _ => Ordering::Equal,
        };
        match (&self.parsed, &other.parsed) {
            (Parsed::Date(a), Parsed::Date(b)) => parsed(a, b, text),
            (Parsed::Instant(a), Parsed::Instant(b)) => parsed(a, b, text),
            _ => compare_values(self.value, other.value),
        }
    }
}

/// [`compare_key`] over prepared values; a missing value stays last.
fn compare_prepared(key: &SortKey, a: &Prepared<'_>, b: &Prepared<'_>) -> Ordering {
    match (a.value.is_some(), b.value.is_some()) {
        (true, true) if key.descending => b.order(a),
        (true, true) => a.order(b),
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        (false, false) => Ordering::Equal,
    }
}

/// The permutation that sorts `rows` by `keys`, as indices into `rows`.
/// Stable: rows that tie keep their given order. Each row's keyed values
/// are looked up, and its dates parsed, once, not once per comparison.
pub fn order<'a>(
    keys: &[SortKey],
    rows: impl IntoIterator<Item = &'a BTreeMap<String, ProjectionValue>>,
) -> Vec<usize> {
    let keyed: Vec<Vec<Prepared<'a>>> = rows
        .into_iter()
        .map(|values| {
            keys.iter()
                .map(|key| Prepared::of(values.get(&key.field)))
                .collect()
        })
        .collect();
    let mut indices: Vec<usize> = (0..keyed.len()).collect();
    indices.sort_by(|&l, &r| {
        keys.iter()
            .enumerate()
            .map(|(at, key)| compare_prepared(key, &keyed[l][at], &keyed[r][at]))
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
    fn a_shift_click_adds_the_next_key_or_flips_one_held() {
        let one = header_add(&[], "score");
        assert_eq!(one, [SortKey::ascending("score")]);
        let two = header_add(&one, "name");
        assert_eq!(
            two,
            [SortKey::ascending("score"), SortKey::ascending("name")]
        );
        assert_eq!(
            header_add(&two, "score"),
            [SortKey::descending("score"), SortKey::ascending("name")]
        );
        // A plain click still replaces the whole sort.
        assert_eq!(header_click(&two, "name"), [SortKey::ascending("name")]);
    }

    #[test]
    fn dates_parsed_once_order_as_compare_values_does() {
        let dated = |kind: fn(String) -> ProjectionValue, text: &str| {
            BTreeMap::from([("at".to_owned(), kind(text.to_owned()))])
        };
        let rows = [
            dated(ProjectionValue::DateTime, "2024-01-01T13:00:00+01:00"),
            dated(ProjectionValue::DateTime, "not a time"),
            dated(ProjectionValue::DateTime, "2023-06-01T00:00:00Z"),
            dated(ProjectionValue::DateTime, "2024-01-01T12:00:00Z"),
            dated(ProjectionValue::Date, "2020-02-02"),
            dated(ProjectionValue::DateTime, "also not"),
            BTreeMap::new(),
        ];
        for keys in [[SortKey::ascending("at")], [SortKey::descending("at")]] {
            let mut by_compare: Vec<usize> = (0..rows.len()).collect();
            by_compare.sort_by(|&l, &r| compare(&keys, &rows[l], &rows[r]));
            assert_eq!(order(&keys, rows.iter()), by_compare, "{keys:?}");
        }
    }

    #[test]
    fn an_ascending_key_writes_only_its_field() {
        let json = serde_json::to_string(&SortKey::ascending("score")).unwrap();
        assert_eq!(json, r#"{"field":"score"}"#);
        let back: SortKey = serde_json::from_str(r#"{"field":"score","descending":true}"#).unwrap();
        assert_eq!(back, SortKey::descending("score"));
    }
}
