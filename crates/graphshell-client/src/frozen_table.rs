// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! A table projection frozen (table projection plan, T4): a captioned HTML
//! table with one column header per field and one row per occurrence.
//!
//! The first cell of each row is its row header, so a reader moving down a
//! column hears which row each value belongs to, with no extra column. The
//! sorted column says so with `aria-sort`. An empty value is written as words
//! ("no value") by whoever fills the cells, never left blank.

use sceno::SourceRef;
use serde::{Deserialize, Serialize};

use crate::frozen::escape;

/// One row: the occurrence's source identity and its cells in column order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FrozenTableRow {
    pub source: SourceRef,
    pub cells: Vec<String>,
}

/// A table: a caption, the fields' names, and the rows in reading order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FrozenTable {
    pub caption: String,
    pub columns: Vec<String>,
    /// The column the sort leads with, and whether it runs descending.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sorted: Option<(usize, bool)>,
    pub rows: Vec<FrozenTableRow>,
}

/// Why a table cannot be frozen as it stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FrozenTableError {
    /// The caption is blank.
    Unlabelled,
    /// No columns, or a column with a blank name.
    BlankColumn { column: usize },
    /// A row does not have one cell per column.
    Ragged { row: usize, cells: usize },
    /// A cell is blank.
    BlankCell { row: usize, column: usize },
}

impl FrozenTable {
    /// Every name present, every row one cell per column, no blank cell.
    pub fn check(&self) -> Result<(), FrozenTableError> {
        if self.caption.trim().is_empty() {
            return Err(FrozenTableError::Unlabelled);
        }
        if self.columns.is_empty() {
            return Err(FrozenTableError::BlankColumn { column: 0 });
        }
        if let Some(column) = self.columns.iter().position(|name| name.trim().is_empty()) {
            return Err(FrozenTableError::BlankColumn { column });
        }
        for (row, cells) in self.rows.iter().map(|row| &row.cells).enumerate() {
            if cells.len() != self.columns.len() {
                return Err(FrozenTableError::Ragged {
                    row,
                    cells: cells.len(),
                });
            }
            if let Some(column) = cells.iter().position(|cell| cell.trim().is_empty()) {
                return Err(FrozenTableError::BlankCell { row, column });
            }
        }
        Ok(())
    }

    /// The table as markup a screen reader announces: the caption, a
    /// `scope="col"` header per field, and each row's first cell as its
    /// `scope="row"` header. Rows carry their source identity.
    pub fn to_html(&self, id_prefix: &str) -> String {
        let mut html = format!(
            "<table class=\"frozen-table\"><caption id=\"{}-caption\">{}</caption><thead><tr>",
            escape(id_prefix),
            escape(&self.caption)
        );
        for (index, name) in self.columns.iter().enumerate() {
            let sort = match self.sorted {
                Some((column, true)) if column == index => " aria-sort=\"descending\"",
                Some((column, false)) if column == index => " aria-sort=\"ascending\"",
                _ => "",
            };
            html.push_str(&format!("<th scope=\"col\"{sort}>{}</th>", escape(name)));
        }
        html.push_str("</tr></thead><tbody>");
        for row in &self.rows {
            html.push_str(&format!(
                "<tr data-source-adapter=\"{}\" data-source-id=\"{}\">",
                escape(&row.source.adapter),
                escape(&row.source.id)
            ));
            for (index, cell) in row.cells.iter().enumerate() {
                if index == 0 {
                    html.push_str(&format!("<th scope=\"row\">{}</th>", escape(cell)));
                } else {
                    html.push_str(&format!("<td>{}</td>", escape(cell)));
                }
            }
            html.push_str("</tr>");
        }
        html.push_str("</tbody></table>");
        html
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> FrozenTable {
        FrozenTable {
            caption: "Reading list".into(),
            columns: vec!["Title".into(), "Pages".into()],
            sorted: Some((1, true)),
            rows: vec![FrozenTableRow {
                source: SourceRef::new("books", "b<1>"),
                cells: vec!["Dune & more".into(), "412".into()],
            }],
        }
    }

    #[test]
    fn a_frozen_table_has_a_caption_and_one_column_header_per_field() {
        let html = table().to_html("t");
        assert!(html.contains("<caption id=\"t-caption\">Reading list</caption>"));
        assert_eq!(html.matches("scope=\"col\"").count(), 2);
        assert!(html.contains("<th scope=\"col\" aria-sort=\"descending\">Pages</th>"));
        assert!(html.contains("<th scope=\"row\">Dune &amp; more</th><td>412</td>"));
        assert!(html.contains("data-source-id=\"b&lt;1&gt;\""));
        assert_eq!(table().check(), Ok(()));
    }

    #[test]
    fn a_blank_or_ragged_table_is_refused() {
        let mut blank = table();
        blank.caption = " ".into();
        assert_eq!(blank.check(), Err(FrozenTableError::Unlabelled));
        let mut ragged = table();
        ragged.rows[0].cells.pop();
        assert_eq!(
            ragged.check(),
            Err(FrozenTableError::Ragged { row: 0, cells: 1 })
        );
        let mut empty = table();
        empty.rows[0].cells[1] = String::new();
        assert_eq!(
            empty.check(),
            Err(FrozenTableError::BlankCell { row: 0, column: 1 })
        );
    }
}
