// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! How a cell shows its value: a field's viewer (table D6, D9, D11).
//!
//! A viewer is a registered id. The built-in four are the plain default for
//! each value type, a chip, a bar in the cell and a check mark; a host adds
//! its own with [`TableViewers::with`]. A viewer that does not fit a value
//! (a bar over text) shows the plain default, so no value is ever hidden.

use std::collections::BTreeMap;

use scenograph::dataset::ProjectionValue;
use scenograph::fields::DEFAULT_VIEWER;

/// What a column's numbers span, for a bar.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ColumnFacts {
    pub min: f64,
    pub max: f64,
}

impl ColumnFacts {
    /// The span of every finite number in `values`.
    pub fn of<'a>(values: impl Iterator<Item = Option<&'a ProjectionValue>>) -> Self {
        let mut span: Option<(f64, f64)> = None;
        for number in values.flatten().filter_map(ProjectionValue::number) {
            if number.is_finite() {
                span = Some(span.map_or((number, number), |(lo, hi)| {
                    (lo.min(number), hi.max(number))
                }));
            }
        }
        let (min, max) = span.unwrap_or_default();
        Self { min, max }
    }

    /// Where `value` sits in the span, from 0 to 1; a single value is full.
    pub fn fraction(&self, value: f64) -> f32 {
        if self.max > self.min {
            ((value - self.min) / (self.max - self.min)).clamp(0.0, 1.0) as f32
        } else {
            1.0
        }
    }
}

/// A cell's content, before it becomes a view.
#[derive(Clone, Debug, PartialEq)]
pub enum CellContent {
    /// No value.
    Empty,
    Text(String),
    Chip(String),
    /// A bar filled to `fraction` of the column's span, with its value.
    Bar {
        fraction: f32,
        text: String,
    },
    Check(bool),
}

impl CellContent {
    /// The cell in words, for the frozen table and a reader.
    pub fn words(&self) -> String {
        match self {
            Self::Empty => "no value".into(),
            Self::Text(text) | Self::Chip(text) | Self::Bar { text, .. } => text.clone(),
            Self::Check(true) => "yes".into(),
            Self::Check(false) => "no".into(),
        }
    }
}

/// A viewer: a value and its column's facts to a cell.
pub type Viewer = fn(Option<&ProjectionValue>, &ColumnFacts) -> CellContent;

/// The viewers a host can draw, by id.
#[derive(Clone, Debug)]
pub struct TableViewers(BTreeMap<String, Viewer>);

impl TableViewers {
    /// Default, chip, bar and check: the first table's four (table D6).
    pub fn built_in() -> Self {
        Self(BTreeMap::from([
            (DEFAULT_VIEWER.to_owned(), plain as Viewer),
            ("chip".to_owned(), chip as Viewer),
            ("bar".to_owned(), bar as Viewer),
            ("check".to_owned(), check as Viewer),
        ]))
    }

    pub fn with(mut self, id: impl Into<String>, viewer: Viewer) -> Self {
        self.0.insert(id.into(), viewer);
        self
    }

    pub fn get(&self, id: &str) -> Option<Viewer> {
        self.0.get(id).copied()
    }

    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.0.keys().map(String::as_str)
    }
}

impl Default for TableViewers {
    fn default() -> Self {
        Self::built_in()
    }
}

/// A value in plain words: the default viewer for every type.
pub fn plain_text(value: &ProjectionValue) -> String {
    match value {
        ProjectionValue::Text(text) | ProjectionValue::Date(text) => text.clone(),
        ProjectionValue::DateTime(text) => text.clone(),
        ProjectionValue::Number(number) => number_text(*number),
        ProjectionValue::Boolean(true) => "Yes".into(),
        ProjectionValue::Boolean(false) => "No".into(),
        // The source identity until reference cells show the entity (T7).
        ProjectionValue::Reference(source) => source.id.clone(),
        ProjectionValue::List(items) => items.iter().map(plain_text).collect::<Vec<_>>().join(", "),
    }
}

/// A number without a trailing `.0` when it is whole.
fn number_text(number: f64) -> String {
    if number.is_finite() && number.fract() == 0.0 && number.abs() < 1e15 {
        format!("{}", number as i64)
    } else {
        format!("{number}")
    }
}

fn plain(value: Option<&ProjectionValue>, _: &ColumnFacts) -> CellContent {
    value.map_or(CellContent::Empty, |value| {
        CellContent::Text(plain_text(value))
    })
}

fn chip(value: Option<&ProjectionValue>, facts: &ColumnFacts) -> CellContent {
    match value {
        Some(value) if !matches!(value, ProjectionValue::List(items) if items.is_empty()) => {
            CellContent::Chip(plain_text(value))
        },
        _ => plain(value, facts),
    }
}

fn bar(value: Option<&ProjectionValue>, facts: &ColumnFacts) -> CellContent {
    match value.and_then(ProjectionValue::number) {
        Some(number) if number.is_finite() => CellContent::Bar {
            fraction: facts.fraction(number),
            text: number_text(number),
        },
        _ => plain(value, facts),
    }
}

fn check(value: Option<&ProjectionValue>, facts: &ColumnFacts) -> CellContent {
    match value {
        Some(ProjectionValue::Boolean(on)) => CellContent::Check(*on),
        _ => plain(value, facts),
    }
}
