// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! A reading's filter (table projection plan, T5): conditions that all hold
//! (table D23).
//!
//! Each condition is a field, an operator and a value. Every kind takes is,
//! is not, is empty and is not empty; numbers, dates and date-times take the
//! ordered operators, by the dataset's own order; text takes contains and
//! starts with, ignoring case by simple lowercasing, while is compares exact
//! bytes (table D31); a list takes contains an item. The saved filter is a
//! list of conditions, so an `any` block can join it later as another kind
//! of entry without changing what is saved now.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use sceno::SourceRef;
use serde::{Deserialize, Serialize};

use crate::dataset::{ProjectionFieldType, ProjectionValue, compare_values, value_words};
use crate::temporal;

/// One condition: a dataset field, an operator and, for most operators, a
/// value.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Condition {
    /// The field's path, the dataset key.
    pub field: String,
    pub op: FilterOp,
    /// Absent for is empty and is not empty; two values for between.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<FilterValue>,
}

impl Condition {
    pub fn new(field: impl Into<String>, op: FilterOp, value: Option<FilterValue>) -> Self {
        Self {
            field: field.into(),
            op,
            value,
        }
    }

    /// The condition in words, with the field called `name`.
    pub fn words(&self, name: &str) -> String {
        let value = self
            .value
            .as_ref()
            .map(FilterValue::words)
            .unwrap_or_default();
        match self.op {
            FilterOp::IsEmpty | FilterOp::IsNotEmpty => format!("{name} {}", self.op.words()),
            FilterOp::Between => match &self.value {
                Some(FilterValue::Many(pair)) if pair.len() == 2 => format!(
                    "{name} is between {} and {}",
                    pair[0].words(),
                    pair[1].words()
                ),
                _ => format!("{name} is between {value}"),
            },
            _ => format!("{name} {} {value}", self.op.words()),
        }
    }
}

/// What a condition tests.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterOp {
    Is,
    IsNot,
    IsEmpty,
    IsNotEmpty,
    Lt,
    Le,
    Gt,
    Ge,
    Between,
    Contains,
    StartsWith,
    ContainsItem,
}

impl FilterOp {
    /// Whether a field disclosed as `kind` takes this operator.
    pub fn takes(self, kind: ProjectionFieldType) -> bool {
        use FilterOp as Op;
        use ProjectionFieldType as T;
        match self {
            Op::Is | Op::IsNot | Op::IsEmpty | Op::IsNotEmpty => true,
            Op::Lt | Op::Le | Op::Gt | Op::Ge | Op::Between => {
                matches!(kind, T::Number | T::Date | T::DateTime)
            },
            Op::Contains | Op::StartsWith => kind == T::Text,
            Op::ContainsItem => matches!(kind, T::List(_)),
        }
    }

    fn words(self) -> &'static str {
        match self {
            Self::Is => "is",
            Self::IsNot => "is not",
            Self::IsEmpty => "is empty",
            Self::IsNotEmpty => "is not empty",
            Self::Lt => "is less than",
            Self::Le => "is at most",
            Self::Gt => "is greater than",
            Self::Ge => "is at least",
            Self::Between => "is between",
            Self::Contains => "contains",
            Self::StartsWith => "starts with",
            Self::ContainsItem => "has the item",
        }
    }
}

/// A condition's value as written. Its meaning comes from the field's
/// disclosed kind: text names a date for a date field, a table with an
/// adapter and an id names a reference, and a list holds a list's items or a
/// range's two ends.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FilterValue {
    Boolean(bool),
    Number(f64),
    Text(String),
    // Before the reference: a struct also reads from a two-item list.
    Many(Vec<FilterValue>),
    Reference(SourceRef),
}

// Numbers compare by their bits, so a saved NaN equals itself and `Eq` holds.
impl PartialEq for FilterValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Boolean(a), Self::Boolean(b)) => a == b,
            (Self::Number(a), Self::Number(b)) => a.to_bits() == b.to_bits(),
            (Self::Text(a), Self::Text(b)) => a == b,
            (Self::Reference(a), Self::Reference(b)) => a == b,
            (Self::Many(a), Self::Many(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for FilterValue {}

impl FilterValue {
    fn words(&self) -> String {
        match self {
            Self::Boolean(true) => "yes".into(),
            Self::Boolean(false) => "no".into(),
            Self::Number(n) => value_words(&ProjectionValue::Number(*n)),
            Self::Text(text) => format!("\u{201c}{text}\u{201d}"),
            Self::Reference(source) => source.id.clone(),
            Self::Many(items) => items.iter().map(Self::words).collect::<Vec<_>>().join(", "),
        }
    }

    /// This value as a value of `kind`, or why it cannot be one.
    fn literal(&self, kind: ProjectionFieldType) -> Result<ProjectionValue, String> {
        use ProjectionFieldType as T;
        let value = match (self, kind) {
            (Self::Boolean(b), T::Boolean) => ProjectionValue::Boolean(*b),
            (Self::Number(n), T::Number) => ProjectionValue::Number(*n),
            (Self::Text(t), T::Text) => ProjectionValue::Text(t.clone()),
            (Self::Text(t), T::Date) => {
                temporal::parse_date(t)?;
                ProjectionValue::Date(t.clone())
            },
            (Self::Text(t), T::DateTime) => {
                temporal::parse_date_time(t)?;
                ProjectionValue::DateTime(t.clone())
            },
            (Self::Reference(source), T::Reference) => ProjectionValue::Reference(source.clone()),
            (Self::Many(items), T::List(item)) => ProjectionValue::List(
                items
                    .iter()
                    .map(|value| value.literal(item.into()))
                    .collect::<Result<_, _>>()?,
            ),
            _ => return Err(format!("{} is not a {}", self.words(), kind_words(kind))),
        };
        Ok(value)
    }
}

fn kind_words(kind: ProjectionFieldType) -> String {
    use ProjectionFieldType as T;
    match kind {
        T::Text => "text".into(),
        T::Number => "number".into(),
        T::Boolean => "true or false".into(),
        T::Reference => "reference".into(),
        T::Date => "date".into(),
        T::DateTime => "date and time".into(),
        T::List(item) => format!("list of {}", kind_words(item.into())),
    }
}

/// One condition made ready to test rows.
#[derive(Clone, Debug)]
enum Test {
    Is(ProjectionValue),
    IsNot(ProjectionValue),
    IsEmpty,
    IsNotEmpty,
    /// Holds when the row's value orders as one of `accept` against `value`.
    Order {
        value: ProjectionValue,
        accept: [Option<Ordering>; 2],
    },
    Between(ProjectionValue, ProjectionValue),
    /// Lowercased once.
    Contains(String),
    StartsWith(String),
    ContainsItem(ProjectionValue),
}

/// A filter checked against a dataset's fields and ready to test rows.
#[derive(Clone, Debug, Default)]
pub struct Matcher {
    tests: Vec<(String, Test)>,
}

impl Matcher {
    /// Check `conditions` against the disclosed `fields`.
    ///
    /// # Errors
    ///
    /// Each refused condition's index and why, naming the field: a field
    /// not disclosed, an operator its kind does not take, or a value of the
    /// wrong kind.
    pub fn new(
        conditions: &[Condition],
        fields: &BTreeMap<String, ProjectionFieldType>,
    ) -> Result<Self, Vec<(usize, String)>> {
        let mut tests = Vec::with_capacity(conditions.len());
        let mut refusals = Vec::new();
        for (index, condition) in conditions.iter().enumerate() {
            match test(condition, fields) {
                Ok(test) => tests.push((condition.field.clone(), test)),
                Err(why) => refusals.push((index, why)),
            }
        }
        if refusals.is_empty() {
            Ok(Self { tests })
        } else {
            Err(refusals)
        }
    }

    pub fn is_empty(&self) -> bool {
        self.tests.is_empty()
    }

    /// Whether every condition holds for a row's values.
    pub fn matches(&self, values: &BTreeMap<String, ProjectionValue>) -> bool {
        self.tests
            .iter()
            .all(|(field, test)| holds(test, values.get(field)))
    }
}

fn test(
    condition: &Condition,
    fields: &BTreeMap<String, ProjectionFieldType>,
) -> Result<Test, String> {
    let field = &condition.field;
    let Some(&kind) = fields.get(field) else {
        return Err(format!(
            "the filter reads {field:?}, which this dataset does not disclose"
        ));
    };
    let op = condition.op;
    if !op.takes(kind) {
        return Err(format!(
            "the filter asks whether {field:?} {}, which a {} field cannot answer",
            op.words(),
            kind_words(kind)
        ));
    }
    let wrong = |why: String| format!("the filter on {field:?}: {why}");
    let one = |value: &Option<FilterValue>, kind: ProjectionFieldType| match value {
        None => Err(wrong(format!("{} needs a value", op.words()))),
        Some(value) => value.literal(kind).map_err(wrong),
    };
    let value = &condition.value;
    Ok(match op {
        FilterOp::IsEmpty | FilterOp::IsNotEmpty => {
            if value.is_some() {
                return Err(wrong(format!("{} takes no value", op.words())));
            }
            if op == FilterOp::IsEmpty {
                Test::IsEmpty
            } else {
                Test::IsNotEmpty
            }
        },
        FilterOp::Is => Test::Is(one(value, kind)?),
        FilterOp::IsNot => Test::IsNot(one(value, kind)?),
        FilterOp::Lt | FilterOp::Le | FilterOp::Gt | FilterOp::Ge => {
            use Ordering::{Equal, Greater, Less};
            let accept = match op {
                FilterOp::Lt => [Some(Less), None],
                FilterOp::Le => [Some(Less), Some(Equal)],
                FilterOp::Gt => [Some(Greater), None],
                _ => [Some(Greater), Some(Equal)],
            };
            Test::Order {
                value: one(value, kind)?,
                accept,
            }
        },
        FilterOp::Between => match value {
            Some(FilterValue::Many(pair)) if pair.len() == 2 => Test::Between(
                pair[0].literal(kind).map_err(wrong)?,
                pair[1].literal(kind).map_err(wrong)?,
            ),
            _ => return Err(wrong("between needs two values, low and high".into())),
        },
        FilterOp::Contains | FilterOp::StartsWith => {
            let Some(FilterValue::Text(text)) = value else {
                return Err(wrong(format!("{} needs text", op.words())));
            };
            let needle = text.to_lowercase();
            if op == FilterOp::Contains {
                Test::Contains(needle)
            } else {
                Test::StartsWith(needle)
            }
        },
        FilterOp::ContainsItem => {
            let ProjectionFieldType::List(item) = kind else {
                unreachable!("only a list takes contains an item");
            };
            Test::ContainsItem(one(value, item.into())?)
        },
    })
}

/// A missing value, an empty text or an empty list.
fn empty(value: Option<&ProjectionValue>) -> bool {
    match value {
        None => true,
        Some(ProjectionValue::Text(text)) => text.is_empty(),
        Some(ProjectionValue::List(items)) => items.is_empty(),
        Some(_) => false,
    }
}

fn same(a: &ProjectionValue, b: &ProjectionValue) -> bool {
    compare_values(Some(a), Some(b)) == Ordering::Equal
}

fn holds(test: &Test, value: Option<&ProjectionValue>) -> bool {
    match test {
        Test::IsEmpty => empty(value),
        Test::IsNotEmpty => !empty(value),
        // A row with no value is not the value, so it passes is not.
        Test::IsNot(want) => value.is_none_or(|have| !same(have, want)),
        Test::Is(want) => value.is_some_and(|have| same(have, want)),
        Test::Order {
            value: want,
            accept,
        } => value.is_some_and(|have| {
            let order = compare_values(Some(have), Some(want));
            accept.contains(&Some(order))
        }),
        Test::Between(low, high) => value.is_some_and(|have| {
            compare_values(Some(have), Some(low)).is_ge()
                && compare_values(Some(have), Some(high)).is_le()
        }),
        Test::Contains(needle) => value
            .and_then(ProjectionValue::text)
            .is_some_and(|text| text.to_lowercase().contains(needle.as_str())),
        Test::StartsWith(needle) => value
            .and_then(ProjectionValue::text)
            .is_some_and(|text| text.to_lowercase().starts_with(needle.as_str())),
        Test::ContainsItem(want) => value
            .and_then(ProjectionValue::list)
            .is_some_and(|items| items.iter().any(|item| same(item, want))),
    }
}

#[cfg(test)]
#[path = "filter_tests.rs"]
mod tests;
