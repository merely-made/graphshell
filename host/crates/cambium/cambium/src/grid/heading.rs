// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! A group heading in the data grid (table D25): a body row drawn as one
//! header cell across every column, with its count and `aria-expanded`.

use sprigging::{GridSpec, Placement};

use super::GridView;
use crate::{
    KeyEvent, PointerClick, arrangement::placed_with, el, on_click, on_key, request_focus,
};

/// What one body row of a banded grid is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GridBand {
    /// A row, with the key it is kept by across renders.
    Row(usize),
    Heading(GridHeading),
}

/// The key a body row is kept by: a row's key, or a heading's group.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GridRowKey {
    Row(usize),
    Heading(String),
}

/// A body row drawn as a group heading: one header cell across every column.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GridHeading {
    /// What the heading is kept by: its group, never its position.
    pub key: String,
    /// "Shelf: Fiction".
    pub label: String,
    pub count: usize,
    /// Zero for the outermost group; inner levels are indented.
    pub level: usize,
    pub expanded: bool,
}

/// A heading band: a row with one header cell across every column, its
/// label and count, indented by level, and a drawn open or closed mark.
pub(super) fn heading_row<State: 'static, Action: 'static>(
    spec: &GridSpec,
    r: usize,
    band: GridHeading,
    stop: Option<(bool, bool)>,
    on_heading: impl Fn(&mut State, usize) + 'static,
) -> GridView<State, Action> {
    let width = spec.total_width();
    let rows = if band.count == 1 {
        "1 row".to_owned()
    } else {
        format!("{} rows", band.count)
    };
    let mark = el::<_, State, Action>("span", ())
        .attr(
            "class",
            if band.expanded {
                "grid-heading-mark grid-heading-open"
            } else {
                "grid-heading-mark grid-heading-closed"
            },
        )
        .attr("aria-hidden", "true");
    let cell = placed_with(
        Placement::new(0.0, 0.0),
        format!(
            "width: {width}px; height: {}px; padding-left: {}px;",
            spec.row_height,
            8 + 16 * band.level
        ),
        (
            mark,
            el::<_, State, Action>("span", band.label).attr("class", "grid-heading-label"),
            el::<_, State, Action>("span", rows).attr("class", "grid-heading-count"),
        ),
    )
    .attr("class", "grid-heading-cell")
    .attr("role", "rowheader")
    .attr("aria-colindex", "1")
    .attr("aria-colspan", spec.columns.len().to_string());
    let cell: GridView<State, Action> = match stop {
        Some((here, requested)) => Box::new(request_focus(
            on_key(
                cell.attr("tabindex", if here { "0" } else { "-1" }),
                |_: &mut State, _: KeyEvent| {},
            )
            .focusable(here),
            requested,
        )),
        None => Box::new(cell),
    };
    Box::new(on_click(
        placed_with(
            Placement::new(0.0, r as f32 * spec.row_height),
            format!("width: {width}px; height: {}px;", spec.row_height),
            cell,
        )
        .attr(
            "class",
            format!("grid-row grid-heading grid-heading-level-{}", band.level),
        )
        .attr("role", "row")
        .attr("aria-rowindex", (r + 2).to_string())
        .attr("aria-expanded", band.expanded.to_string())
        .attr("data-row", r.to_string()),
        move |s: &mut State, _: PointerClick| on_heading(s, r),
    ))
}
