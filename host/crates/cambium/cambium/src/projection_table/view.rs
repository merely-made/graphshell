// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The table's view: a caption, the data grid over only the visible rows,
//! and a status line that says how many rows there are and how they sort.

use sprigging::{GridColumn, GridSpec};

use crate::grid::{GridOptions, GridView, data_grid_with};
use crate::{KeyEvent, PointerClick, WheelEvent, el, on_click, on_wheel};

use super::{CellContent, ProjectionTable};

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
.grid-cell { padding: 2px 8px; box-sizing: border-box; overflow: hidden; white-space: nowrap; } \
.cell-content { display: flex; align-items: center; gap: 6px; height: 100%; } \
.cell-chip { padding: 0 6px; border-radius: 8px; } \
.cell-bar { display: flex; align-items: center; gap: 6px; } \
.cell-bar-track { height: 8px; } \
.cell-bar-fill { height: 8px; } \
.cell-check { display: inline-block; width: 12px; height: 12px; box-sizing: border-box; }";

fn cell_view<Action: 'static>(
    content: CellContent,
    width: f32,
    rank: usize,
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
        move |table: &mut ProjectionTable, _: PointerClick| table.focus_cell(Some(rank), col),
    ))
}

/// What the status line says.
fn status(table: &ProjectionTable) -> String {
    if let Some(notice) = &table.notice {
        return notice.clone();
    }
    let rows = table.row_count();
    let count = if rows == 1 {
        "1 row".to_owned()
    } else {
        format!("{rows} rows")
    };
    match table.sorted() {
        Some((col, descending)) => format!(
            "{count}, sorted by {}, {}",
            table.columns()[col].name,
            if descending {
                "descending"
            } else {
                "ascending"
            }
        ),
        None => count,
    }
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
        focus: Some(table.focus()),
        label: Some(table.caption().to_owned()),
    };
    let grid = data_grid_with(
        &spec,
        table.row_count(),
        table.viewport_height,
        table.scroll,
        |rank, col| cell_view(table.cell(rank, col), widths[col], rank, col),
        // A sort never joins an undo run, so the clock does not matter here.
        |table: &mut ProjectionTable, col| table.click_header(col, 0),
        move |rank| (Some(rank) == focused_row).then(|| "grid-row-focus".to_owned()),
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
