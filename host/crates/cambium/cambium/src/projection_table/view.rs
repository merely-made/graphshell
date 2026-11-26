// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The table's view: a caption, the data grid over only the visible rows
//! and headings, and a status line that says how many rows are read, which
//! conditions keep them, how they group and how they sort.

use sprigging::{GridColumn, GridSpec};

use crate::grid::{GridBand, GridCell, GridHeading, GridOptions, GridView, data_grid_banded};
use crate::{KeyEvent, Modifiers, PointerClick, WheelEvent, el, on_click, on_wheel};

use super::{CellContent, ProjectionTable, TableBandView};

/// Rows drawn beyond the body on each side.
const OVERSCAN: usize = 2;

/// Structural styles for the table; colour belongs to the host's sheet.
pub const PROJECTION_TABLE_CSS: &str = "\
.projection-table { display: flex; flex-direction: column; gap: 6px; } \
.table-caption { font-weight: bold; } \
.grid-header-cell { padding: 4px 8px; box-sizing: border-box; overflow: hidden; } \
.grid-sort { display: inline-block; width: 0; height: 0; margin-left: 6px; } \
.grid-sort-asc { border-left: 4px solid transparent; border-right: 4px solid transparent; border-bottom: 6px solid currentColor; } \
.grid-sort-desc { border-left: 4px solid transparent; border-right: 4px solid transparent; border-top: 6px solid currentColor; } \
.grid-key-priority { margin-left: 3px; font-size: 11px; } \
.grid-group-on { display: inline-block; width: 8px; height: 2px; margin-left: 5px; border-top: 2px solid currentColor; border-bottom: 2px solid currentColor; } \
.grid-heading-cell { display: flex; align-items: center; gap: 6px; box-sizing: border-box; font-weight: bold; white-space: nowrap; overflow: hidden; } \
.grid-heading-count { font-weight: normal; } \
.grid-heading-mark { display: inline-block; width: 0; height: 0; } \
.grid-heading-open { border-left: 4px solid transparent; border-right: 4px solid transparent; border-top: 6px solid currentColor; } \
.grid-heading-closed { border-top: 4px solid transparent; border-bottom: 4px solid transparent; border-left: 6px solid currentColor; } \
.grid-cell { padding: 2px 8px; box-sizing: border-box; overflow: hidden; white-space: nowrap; } \
.cell-content { display: flex; align-items: center; gap: 6px; height: 100%; } \
.cell-chip { padding: 0 6px; border-radius: 8px; } \
.cell-bar { display: flex; align-items: center; gap: 6px; } \
.cell-bar-track { height: 8px; } \
.cell-bar-fill { height: 8px; } \
.cell-check { display: inline-block; width: 12px; height: 12px; box-sizing: border-box; }";

/// What one cell's view reads: equal inputs keep its row unbuilt (D82).
#[derive(Clone, Debug, PartialEq)]
struct TableCell {
    content: CellContent,
    width: f32,
    band: usize,
    col: usize,
}

impl<Action: 'static> GridCell<ProjectionTable, Action> for TableCell {
    fn view(&self) -> GridView<ProjectionTable, Action> {
        cell_view(self.content.clone(), self.width, self.band, self.col)
    }
}

fn cell_view<Action: 'static>(
    content: CellContent,
    width: f32,
    band: usize,
    col: usize,
) -> GridView<ProjectionTable, Action> {
    let inner: GridView<ProjectionTable, Action> = match content {
        CellContent::Empty => Box::new(el("span", ()).attr("class", "cell-empty")),
        CellContent::Text(text) => Box::new(el("span", text).attr("class", "cell-text")),
        CellContent::Chip(text) => Box::new(el("span", text).attr("class", "cell-chip")),
        CellContent::Bar { fraction, text } => {
            let track = (width * 0.5).max(8.0);
            Box::new(
                el(
                    "span",
                    (
                        el(
                            "span",
                            el("span", ()).attr("class", "cell-bar-fill").attr(
                                "style",
                                format!("display: block; width: {}px;", track * fraction),
                            ),
                        )
                        .attr("class", "cell-bar-track")
                        .attr("style", format!("display: block; width: {track}px;")),
                        el("span", text).attr("class", "cell-text"),
                    ),
                )
                .attr("class", "cell-bar"),
            )
        },
        CellContent::Check(on) => Box::new(
            el("span", ())
                .attr(
                    "class",
                    if on {
                        "cell-check cell-check-on"
                    } else {
                        "cell-check"
                    },
                )
                .attr("role", "img")
                .attr("aria-label", if on { "yes" } else { "no" }),
        ),
    };
    Box::new(on_click(
        el("div", inner).attr("class", "cell-content"),
        move |table: &mut ProjectionTable, _: PointerClick| table.focus_cell(Some(band), col),
    ))
}

/// A field's name, by its column, else its path.
fn name_of<'a>(table: &'a ProjectionTable, field: &'a str) -> &'a str {
    table
        .columns()
        .iter()
        .find(|column| column.field == field)
        .map_or(field, |column| column.name.as_str())
}

/// Keys in words: "Read, descending, then Pages, ascending".
fn keys_words(table: &ProjectionTable, keys: &[scenograph::SortKey]) -> String {
    keys.iter()
        .map(|key| {
            let way = if key.descending {
                "descending"
            } else {
                "ascending"
            };
            format!("{}, {way}", name_of(table, &key.field))
        })
        .collect::<Vec<_>>()
        .join(", then ")
}

/// What the status line says.
fn status(table: &ProjectionTable) -> String {
    if let Some(notice) = &table.notice {
        return notice.clone();
    }
    let reading = &table.draft().reading;
    let rows = table.row_count();
    let noun = if rows == 1 { "row" } else { "rows" };
    let mut words = if reading.filter.is_empty() {
        format!("{rows} {noun}")
    } else {
        // Of how many, and the conditions that keep them.
        let conditions: Vec<String> = reading
            .filter
            .iter()
            .map(|condition| condition.words(name_of(table, &condition.field)))
            .collect();
        format!(
            "{rows} of {} {noun}, where {}",
            table.source_row_count(),
            conditions.join(" and ")
        )
    };
    if !reading.group.is_empty() {
        words.push_str(&format!(
            ", grouped by {}",
            keys_words(table, &reading.group)
        ));
    }
    // Under a grouping, only the keys that still order rows inside the
    // groups (table D40).
    let sorting: Vec<scenograph::SortKey> = reading
        .sort
        .iter()
        .filter(|key| !reading.group.iter().any(|g| g.field == key.field))
        .cloned()
        .collect();
    if !sorting.is_empty() {
        words.push_str(&format!(", sorted by {}", keys_words(table, &sorting)));
    }
    words
}

/// The table on the data grid. Embed it with a lens onto the host's
/// [`ProjectionTable`].
pub fn projection_table<Action: 'static>(
    table: &ProjectionTable,
) -> GridView<ProjectionTable, Action> {
    let sorted = table.sorted();
    let spec = GridSpec {
        columns: table
            .columns()
            .iter()
            .map(|column| GridColumn::new(column.name.clone(), column.width))
            .collect(),
        row_height: table.row_height(),
        header_height: table.header_height(),
        overscan: OVERSCAN,
    };
    let widths: Vec<f32> = table.columns().iter().map(|c| c.width).collect();
    let focused_row = table.focus().row;
    let options = GridOptions {
        sorted,
        sort_keys: table.sort_columns(),
        group_keys: table.group_columns(),
        focus: Some(table.focus()),
        label: Some(table.caption().to_owned()),
    };
    let grid = data_grid_banded(
        &spec,
        table.band_count(),
        table.body_height(),
        table.scroll,
        |band, col| {
            let rank = table.band_row(band).unwrap_or(usize::MAX);
            TableCell {
                content: table.cell(rank, col),
                width: widths[col],
                band,
                col,
            }
        },
        // A row is kept by its occurrence id, so a sort, a filter or a
        // collapse moves it rather than rewriting it (table D41).
        |band| match table.band(band) {
            Some(TableBandView::Heading(heading)) => GridBand::Heading(GridHeading {
                key: heading.key,
                label: heading.label,
                count: heading.count,
                level: heading.level,
                expanded: !heading.collapsed,
            }),
            Some(TableBandView::Row(rank)) => {
                GridBand::Id(table.row_id(rank).unwrap_or_default().to_owned())
            },
            None => GridBand::Row(band),
        },
        // A sort never joins an undo run, so the clock does not matter here.
        |table: &mut ProjectionTable, col, mods: Modifiers| table.press_header(col, mods.shift, 0),
        |table: &mut ProjectionTable, band| {
            table.focus_cell(Some(band), table.focus().col);
            table.toggle_band(band);
        },
        move |band| (Some(band) == focused_row).then(|| "grid-row-focus".to_owned()),
        &options,
        |table: &mut ProjectionTable, event: KeyEvent| {
            if table.navigate(&event.key, event.mods.ctrl) {
                event.prevent_default();
            }
        },
    );
    Box::new(
        el(
            "div",
            (
                el("div", table.caption().to_owned()).attr("class", "table-caption"),
                on_wheel(
                    el("div", grid).attr("class", "table-body"),
                    |table: &mut ProjectionTable, event: WheelEvent| {
                        table.scroll_by(event.delta.1);
                        event.prevent_default();
                    },
                ),
                el("div", status(table))
                    .attr("class", "table-status")
                    .attr("role", "status"),
            ),
        )
        .attr("class", "projection-table"),
    )
}
