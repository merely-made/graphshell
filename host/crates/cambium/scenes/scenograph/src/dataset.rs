// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The resolved dataset a host hands the projection compiler.
//!
//! A product adapter reads its own authority and resolves it into this generic
//! table: typed fields, and occurrences carrying values and an exact source
//! reference. It is the in-host input to compilation and never part of the
//! remote projection protocol, where endpoints compile locally and send
//! scores. Its shape is product-free, so a new product adds rows, not types.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use sceno::SourceRef;
use serde::{Deserialize, Serialize};

use crate::{PublicSourceRevision, SourceBinding, temporal};

/// The type a product disclosed for a field in one resolved dataset.
///
/// The first three spell as before (`"text"`, `"number"`, `"boolean"`), so
/// saved datasets read unchanged. A list spells `{"list": "<item>"}`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionFieldType {
    Text,
    Number,
    Boolean,
    /// Another entity, named by its source identity.
    Reference,
    /// An `xsd:date`.
    Date,
    /// An `xsd:dateTime` with its offset (RFC 3339).
    DateTime,
    /// An ordered list of one item type, possibly empty.
    List(ProjectionItemType),
}

impl ProjectionFieldType {
    /// Every type a field can disclose, lists of each item type included.
    pub const ALL: [Self; 12] = [
        Self::Text,
        Self::Number,
        Self::Boolean,
        Self::Reference,
        Self::Date,
        Self::DateTime,
        Self::List(ProjectionItemType::Text),
        Self::List(ProjectionItemType::Number),
        Self::List(ProjectionItemType::Boolean),
        Self::List(ProjectionItemType::Reference),
        Self::List(ProjectionItemType::Date),
        Self::List(ProjectionItemType::DateTime),
    ];
}

/// The type of a list's items. A list is not one of them, so a list of
/// lists cannot be disclosed.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionItemType {
    Text,
    Number,
    Boolean,
    Reference,
    Date,
    DateTime,
}

impl From<ProjectionItemType> for ProjectionFieldType {
    fn from(item: ProjectionItemType) -> Self {
        match item {
            ProjectionItemType::Text => Self::Text,
            ProjectionItemType::Number => Self::Number,
            ProjectionItemType::Boolean => Self::Boolean,
            ProjectionItemType::Reference => Self::Reference,
            ProjectionItemType::Date => Self::Date,
            ProjectionItemType::DateTime => Self::DateTime,
        }
    }
}

/// One source value, kept small enough to make an adapter's disclosure plain.
///
/// A missing value is an absent key in an occurrence's `values`; there is no
/// null. Dates keep the lexical string they were disclosed as.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum ProjectionValue {
    Text(String),
    Number(f64),
    Boolean(bool),
    Reference(SourceRef),
    Date(String),
    DateTime(String),
    List(Vec<ProjectionValue>),
}

impl ProjectionValue {
    /// The type this value is, where the value alone says. An empty list, or
    /// one whose items are not all one item type, says nothing.
    pub fn field_type(&self) -> Option<ProjectionFieldType> {
        Some(match self {
            Self::Text(_) => ProjectionFieldType::Text,
            Self::Number(_) => ProjectionFieldType::Number,
            Self::Boolean(_) => ProjectionFieldType::Boolean,
            Self::Reference(_) => ProjectionFieldType::Reference,
            Self::Date(_) => ProjectionFieldType::Date,
            Self::DateTime(_) => ProjectionFieldType::DateTime,
            Self::List(items) => {
                let first = item_type(items.first()?)?;
                if items.iter().any(|item| item_type(item) != Some(first)) {
                    return None;
                }
                ProjectionFieldType::List(first)
            },
        })
    }

    /// Why this value cannot stand for a field disclosed as `field_type`, or
    /// `None` when it can.
    pub fn refusal(&self, field_type: ProjectionFieldType) -> Option<String> {
        match (self, field_type) {
            (Self::List(items), ProjectionFieldType::List(item)) => {
                items.iter().enumerate().find_map(|(index, value)| {
                    if matches!(value, Self::List(_)) {
                        return Some(format!(
                            "item {index} is a list; a list of lists is refused"
                        ));
                    }
                    value
                        .refusal(item.into())
                        .map(|why| format!("item {index}: {why}"))
                })
            },
            (Self::Reference(source), ProjectionFieldType::Reference) => {
                (source.adapter.trim().is_empty() || source.id.trim().is_empty())
                    .then(|| "a reference needs an adapter and an id".to_owned())
            },
            (Self::Date(text), ProjectionFieldType::Date) => temporal::parse_date(text).err(),
            (Self::DateTime(text), ProjectionFieldType::DateTime) => {
                temporal::parse_date_time(text).err()
            },
            (Self::Text(_), ProjectionFieldType::Text)
            | (Self::Number(_), ProjectionFieldType::Number)
            | (Self::Boolean(_), ProjectionFieldType::Boolean) => None,
            _ => Some("value does not match the disclosed field type".to_owned()),
        }
    }

    pub fn reference(&self) -> Option<&SourceRef> {
        match self {
            Self::Reference(source) => Some(source),
            _ => None,
        }
    }

    pub fn list(&self) -> Option<&[ProjectionValue]> {
        match self {
            Self::List(items) => Some(items),
            _ => None,
        }
    }

    pub fn text(&self) -> Option<&str> {
        match self {
            Self::Text(value) => Some(value),
            _ => None,
        }
    }

    pub fn number(&self) -> Option<f64> {
        match self {
            Self::Number(value) => Some(*value),
            _ => None,
        }
    }
}

/// A value in plain words: a number without a trailing `.0` when whole, true
/// and false as Yes and No, a reference by its source id, a list's items
/// joined by commas.
pub fn value_words(value: &ProjectionValue) -> String {
    match value {
        ProjectionValue::Text(text)
        | ProjectionValue::Date(text)
        | ProjectionValue::DateTime(text) => text.clone(),
        ProjectionValue::Number(number) => {
            if number.is_finite() && number.fract() == 0.0 && number.abs() < 1e15 {
                format!("{}", *number as i64)
            } else {
                format!("{number}")
            }
        },
        ProjectionValue::Boolean(true) => "Yes".into(),
        ProjectionValue::Boolean(false) => "No".into(),
        ProjectionValue::Reference(source) => source.id.clone(),
        ProjectionValue::List(items) => {
            items.iter().map(value_words).collect::<Vec<_>>().join(", ")
        },
    }
}

fn item_type(value: &ProjectionValue) -> Option<ProjectionItemType> {
    Some(match value {
        ProjectionValue::Text(_) => ProjectionItemType::Text,
        ProjectionValue::Number(_) => ProjectionItemType::Number,
        ProjectionValue::Boolean(_) => ProjectionItemType::Boolean,
        ProjectionValue::Reference(_) => ProjectionItemType::Reference,
        ProjectionValue::Date(_) => ProjectionItemType::Date,
        ProjectionValue::DateTime(_) => ProjectionItemType::DateTime,
        ProjectionValue::List(_) => return None,
    })
}

/// The sort order of one field's values, ascending, with a missing value
/// last. Each type has its own order:
///
/// - text by its bytes; numbers by `f64::total_cmp`; false before true;
/// - a reference by adapter, then id: source identity, not a label;
/// - a date by calendar day; a date-time by the instant it names, so
///   `T12:00:00Z` and `T13:00:00+01:00` are equal;
/// - a list item by item, a shorter list first when one is a prefix of the
///   other, so the empty list comes first.
///
/// A malformed date or date-time sorts after every well-formed one, by its
/// text. Values of different types, which validation refuses in one field,
/// still get an order (by type, as declared) so the order stays total.
pub fn compare_values(left: Option<&ProjectionValue>, right: Option<&ProjectionValue>) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => compare_present(left, right),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn compare_present(left: &ProjectionValue, right: &ProjectionValue) -> Ordering {
    use ProjectionValue as V;
    match (left, right) {
        (V::Text(a), V::Text(b)) => a.cmp(b),
        (V::Number(a), V::Number(b)) => a.total_cmp(b),
        (V::Boolean(a), V::Boolean(b)) => a.cmp(b),
        (V::Reference(a), V::Reference(b)) => (&a.adapter, &a.id).cmp(&(&b.adapter, &b.id)),
        (V::Date(a), V::Date(b)) => parsed_cmp(a, b, temporal::parse_date),
        (V::DateTime(a), V::DateTime(b)) => parsed_cmp(a, b, temporal::parse_date_time),
        (V::List(a), V::List(b)) => a
            .iter()
            .zip(b)
            .map(|(a, b)| compare_present(a, b))
            .find(|order| order.is_ne())
            .unwrap_or_else(|| a.len().cmp(&b.len())),
        _ => rank(left).cmp(&rank(right)),
    }
}

/// Well-formed values in their parsed order, then malformed ones by text.
fn parsed_cmp<T: Ord, E>(a: &str, b: &str, parse: impl Fn(&str) -> Result<T, E>) -> Ordering {
    match (parse(a).ok(), parse(b).ok()) {
        (Some(x), Some(y)) => x.cmp(&y),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => a.cmp(b),
    }
}

fn rank(value: &ProjectionValue) -> u8 {
    match value {
        ProjectionValue::Text(_) => 0,
        ProjectionValue::Number(_) => 1,
        ProjectionValue::Boolean(_) => 2,
        ProjectionValue::Reference(_) => 3,
        ProjectionValue::Date(_) => 4,
        ProjectionValue::DateTime(_) => 5,
        ProjectionValue::List(_) => 6,
    }
}

/// One occurrence in a product's resolved reading.
///
/// `occurrence_id` is the host's selection and persistence identity. `source`
/// remains source truth identity, so two occurrences may intentionally name the
/// same source.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProjectionOccurrence {
    pub occurrence_id: String,
    pub source: SourceRef,
    pub values: BTreeMap<String, ProjectionValue>,
    /// The class labels the source discloses for this occurrence.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub classes: Vec<String>,
}

impl ProjectionOccurrence {
    /// Build a row without naming every field, so a field added later does
    /// not break the caller.
    pub fn new(
        occurrence_id: impl Into<String>,
        source: SourceRef,
        values: BTreeMap<String, ProjectionValue>,
    ) -> Self {
        Self {
            occurrence_id: occurrence_id.into(),
            source,
            values,
            classes: Vec::new(),
        }
    }

    pub fn with_classes(mut self, classes: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.classes = classes.into_iter().map(Into::into).collect();
        self
    }
}

/// A product-resolved dataset, supplied to the compiler without a product
/// dependency.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProjectionDataset {
    pub source: SourceBinding,
    pub revision: PublicSourceRevision,
    pub fields: BTreeMap<String, ProjectionFieldType>,
    pub occurrences: Vec<ProjectionOccurrence>,
}

#[cfg(test)]
#[path = "dataset_tests.rs"]
mod tests;
