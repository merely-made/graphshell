/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

//! The data grid (Sprigging's flagship arrangement widget): a sticky
//! header over virtualized, absolutely-placed rows.
//!
//! Geometry comes from [`sprigging::GridSpec`]; row *content* comes from the
//! caller's cell function (any view — text, a `custom_leaf` sparkline, a
//! button), so the grid owns arrangement and nothing else. Only the
//! [`VirtualWindow`](sprigging::VirtualWindow) rows exist as DOM; scrolling is
//! caller state (wire [`on_wheel`](crate::on_wheel) around the grid, clamping
//! with [`GridSpec::max_scroll`]), so the header is "sticky" by construction —
//! it simply never scrolls. Header cells fire `on_header_click(col)` on click,
//! Enter, or Space for sort-by-column, which is likewise caller state: the grid
//! re-renders whatever order the cell function exposes.
//!
//! The root, headers, rows, and cells expose the ARIA grid roles and indices.
//! Only materialized rows exist in the DOM, while `aria-rowcount` reports the
//! full model size, headings included.
//!
//! [`data_grid_banded`] also draws group headings: a body row the caller
//! names a heading is one header cell spanning every column, with its count
//! and `aria-expanded`, and a press on it goes to the caller (table D25).
//!
//! Theming rides classes (`grid`, `grid-header`, `grid-header-cell`,
//! `grid-body`, `grid-row`, `grid-row-even` / `-odd`, `grid-cell`); inline
//! styles carry only geometry.

use sprigging::{GridSpec, Placement};

use crate::pod::GenetElement;
use crate::{
    AnyView, GenetCtx, Key, KeyEvent, Keyed, Modifiers, NamedKey, PointerClick,
    arrangement::placed_with, el, on_click, on_key, request_focus,
};

mod heading;
use heading::heading_row;
pub use heading::{GridBand, GridHeading, GridRowKey};

/// The erased view type grid cells (and the grid itself) use.
pub type GridView<State, Action> = Box<dyn AnyView<State, Action, GenetCtx, GenetElement>>;

/// Where keyboard focus sits in a grid: a header cell (`row: None`) or a body
/// cell. Only that cell is a tab stop; arrows move it (the ARIA grid pattern).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GridFocus {
    pub row: Option<usize>,
    pub col: usize,
    /// Move platform focus onto the cell when it changes.
    pub requested: bool,
}

/// What a grid shows beyond geometry and content.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GridOptions {
    /// The sorted column and whether it runs descending, for `aria-sort`.
    pub sorted: Option<(usize, bool)>,
    /// Every sort key's column and direction in priority order. With more
    /// than one, each sorted header shows its arrow and its priority number.
    pub sort_keys: Vec<(usize, bool)>,
    /// The columns the rows are grouped by, outermost first, and each
    /// group's direction. Such a header shows the group's arrow and a group
    /// mark instead of a sort rank (table D35).
    pub group_keys: Vec<(usize, bool)>,
    /// The roving focus; `None` keeps every header a tab stop and no cell one.
    pub focus: Option<GridFocus>,
    /// The grid's accessible name.
    pub label: Option<String>,
}

/// Build a data grid. `cell(row, col)` supplies each materialized cell's view;
/// `scroll` is the caller's scroll offset into the body (device px);
/// `on_header_click` receives the clicked column index (sort state lives with
/// the caller).
pub fn data_grid<State, Action>(
    spec: &GridSpec,
    total_rows: usize,
    viewport_height: f32,
    scroll: f32,
    cell: impl Fn(usize, usize) -> GridView<State, Action>,
    on_header_click: impl Fn(&mut State, usize) + Clone + 'static,
    row_class: impl Fn(usize) -> Option<String>,
) -> GridView<State, Action>
where
    State: 'static,
    Action: 'static,
{
    data_grid_with(
        spec,
        total_rows,
        viewport_height,
        scroll,
        cell,
        on_header_click,
        row_class,
        &GridOptions::default(),
        |_: &mut State, _: KeyEvent| {},
    )
}

/// [`data_grid`] with sort state, a roving focus and a grid-level key handler
/// (`on_key_event` sees every key that reaches the grid; it owns navigation).
#[allow(clippy::too_many_arguments)]
pub fn data_grid_with<State, Action>(
    spec: &GridSpec,
    total_rows: usize,
    viewport_height: f32,
    scroll: f32,
    cell: impl Fn(usize, usize) -> GridView<State, Action>,
    on_header_click: impl Fn(&mut State, usize) + Clone + 'static,
    row_class: impl Fn(usize) -> Option<String>,
    options: &GridOptions,
    on_key_event: impl Fn(&mut State, KeyEvent) + 'static,
) -> GridView<State, Action>
where
    State: 'static,
    Action: 'static,
{
    data_grid_banded(
        spec,
        total_rows,
        viewport_height,
        scroll,
        cell,
        GridBand::Row,
        move |s: &mut State, col, _: Modifiers| on_header_click(s, col),
        |_: &mut State, _| {},
        row_class,
        options,
        on_key_event,
    )
}

/// [`data_grid_with`] with rows kept by the caller's key (table D42): a
/// caller that knows each row's identity passes `row_key(row)`, so a reorder
/// moves a row's node rather than rewriting it. [`data_grid`] and
/// [`data_grid_with`] keep the row index.
#[allow(clippy::too_many_arguments)]
pub fn data_grid_keyed<State, Action>(
    spec: &GridSpec,
    total_rows: usize,
    viewport_height: f32,
    scroll: f32,
    cell: impl Fn(usize, usize) -> GridView<State, Action>,
    row_key: impl Fn(usize) -> GridRowKey,
    on_header_click: impl Fn(&mut State, usize) + Clone + 'static,
    row_class: impl Fn(usize) -> Option<String>,
    options: &GridOptions,
    on_key_event: impl Fn(&mut State, KeyEvent) + 'static,
) -> GridView<State, Action>
where
    State: 'static,
    Action: 'static,
{
    data_grid_banded(
        spec,
        total_rows,
        viewport_height,
        scroll,
        cell,
        // A plain grid draws no headings; a heading key is kept as an id.
        |row| match row_key(row) {
            GridRowKey::Row(index) => GridBand::Row(index),
            GridRowKey::Id(id) | GridRowKey::Heading(id) => GridBand::Id(id),
        },
        move |s: &mut State, col, _: Modifiers| on_header_click(s, col),
        |_: &mut State, _| {},
        row_class,
        options,
        on_key_event,
    )
}

/// [`data_grid_with`] with group headings: `band(row)` says whether a body
/// row is a row or a heading, with the key it is kept by, and `on_heading(row)`
/// hears a press on a heading. A header press carries its modifier keys, so
/// Shift can add a sort key.
#[allow(clippy::too_many_arguments)]
pub fn data_grid_banded<State, Action>(
    spec: &GridSpec,
    total_rows: usize,
    viewport_height: f32,
    scroll: f32,
    cell: impl Fn(usize, usize) -> GridView<State, Action>,
    band: impl Fn(usize) -> GridBand,
    on_header: impl Fn(&mut State, usize, Modifiers) + Clone + 'static,
    on_heading: impl Fn(&mut State, usize) + Clone + 'static,
    row_class: impl Fn(usize) -> Option<String>,
    options: &GridOptions,
    on_key_event: impl Fn(&mut State, KeyEvent) + 'static,
) -> GridView<State, Action>
where
    State: 'static,
    Action: 'static,
{
    let width = spec.total_width();
    let focus = options.focus;
    // The tab stop and focus request for one cell, or `None` with no focus.
    let stop = |row: Option<usize>, col: usize| {
        focus.map(|focus| {
            let here = focus.row == row && focus.col == col;
            (if here { "0" } else { "-1" }, here && focus.requested)
        })
    };

    // Sticky header: one relative strip that never scrolls.
    let header_cells: Vec<GridView<State, Action>> = spec
        .columns
        .iter()
        .enumerate()
        .map(|(c, col)| {
            let click_handler = on_header.clone();
            let key_handler = on_header.clone();
            let sort = match options.sorted {
                Some((sorted, true)) if sorted == c => "descending",
                Some((sorted, false)) if sorted == c => "ascending",
                _ => "none",
            };
            // Each key of a several-key sort shows its own arrow and number;
            // `aria-sort` stays on the lead key, as ARIA allows one.
            let key = options
                .sort_keys
                .iter()
                .position(|(sorted, _)| *sorted == c)
                .map(|at| (at, options.sort_keys[at].1));
            let grouped = options
                .group_keys
                .iter()
                .find(|(grouped, _)| *grouped == c)
                .map(|(_, descending)| *descending);
            let drawn = match (sort, grouped, key) {
                (_, Some(true), _) | ("none", None, Some((_, true))) => "descending",
                (_, Some(false), _) | ("none", None, Some((_, false))) => "ascending",
                (sort, ..) => sort,
            };
            let priority = key
                .filter(|_| grouped.is_none() && options.sort_keys.len() > 1)
                .map(|(at, _)| (at + 1).to_string())
                .unwrap_or_default();
            let (tabindex, requested) = stop(None, c).unwrap_or(("0", false));
            // Only the tab stop takes Tab focus; the arrows reach the rest.
            let tab_stop = tabindex == "0";
            // The sort indicator is a drawn box, not a glyph a font may lack;
            // `aria-sort` stays the truth and the box is hidden from readers.
            let indicator = el::<_, State, Action>("span", ())
                .attr(
                    "class",
                    match drawn {
                        "descending" => "grid-sort grid-sort-desc",
                        "ascending" => "grid-sort grid-sort-asc",
                        _ => "grid-sort",
                    },
                )
                .attr("aria-hidden", "true");
            let priority = el::<_, State, Action>("span", priority)
                .attr("class", "grid-key-priority")
                .attr("aria-hidden", "true");
            let group_mark = el::<_, State, Action>("span", ())
                .attr(
                    "class",
                    if grouped.is_some() {
                        "grid-group grid-group-on"
                    } else {
                        "grid-group"
                    },
                )
                .attr("aria-hidden", "true");
            let header_cell = placed_with(
                Placement::new(spec.col_x(c), 0.0),
                format!("width: {}px; height: {}px;", col.width, spec.header_height),
                (
                    el::<_, State, Action>("span", col.title.clone())
                        .attr("class", "grid-header-title"),
                    indicator,
                    priority,
                    group_mark,
                ),
            )
            .attr("class", "grid-header-cell")
            .attr("role", "columnheader")
            .attr("aria-colindex", (c + 1).to_string())
            .attr("aria-sort", sort)
            .attr("tabindex", tabindex);
            Box::new(request_focus(
                on_key(
                    on_click(header_cell, move |s: &mut State, click: PointerClick| {
                        click_handler(s, c, click.modifiers)
                    }),
                    move |s: &mut State, event| {
                        if matches!(event.key, Key::Named(NamedKey::Enter | NamedKey::Space)) {
                            key_handler(s, c, event.mods);
                            event.prevent_default();
                        }
                    },
                )
                .focusable(tab_stop),
                requested,
            )) as GridView<State, Action>
        })
        .collect();
    let header = el::<_, State, Action>("div", header_cells)
        .attr("class", "grid-header")
        .attr("role", "row")
        .attr("aria-rowindex", "1")
        .attr(
            "style",
            format!(
                "position: relative; width: {width}px; height: {}px;",
                spec.header_height
            ),
        );

    // Virtualized body: only the window's rows exist, each at its own y in a
    // strip that the scroll offsets. A scroll that keeps the window moves the
    // strip alone, one style change rather than one per row. Rows are keyed by
    // the caller's key (a plain grid's is the row index), so a window shift
    // builds the arriving rows and drops the leaving ones; the rows in both
    // windows are not rewritten. A heading is kept by its group, so a
    // collapse or a regrouping moves it rather than building it again.
    let window = spec.window(total_rows, viewport_height, scroll);
    let rows: Keyed<GridRowKey, GridView<State, Action>> = window
        .range()
        .map(|r| {
            let key = match band(r) {
                GridBand::Heading(heading) => {
                    let stop = focus.map(|focus| {
                        let here = focus.row == Some(r);
                        (here, here && focus.requested)
                    });
                    let key = GridRowKey::Heading(heading.key.clone());
                    let extra = row_class(r);
                    return (
                        key,
                        heading_row(spec, r, heading, stop, extra, on_heading.clone()),
                    );
                },
                GridBand::Row(key) => GridRowKey::Row(key),
                GridBand::Id(id) => GridRowKey::Id(id),
            };
            let cells: Vec<GridView<State, Action>> = spec
                .columns
                .iter()
                .enumerate()
                .map(|(c, col)| {
                    let placed = placed_with(
                        Placement::new(spec.col_x(c), 0.0),
                        format!("width: {}px; height: {}px;", col.width, spec.row_height),
                        cell(r, c),
                    )
                    .attr("class", "grid-cell")
                    .attr("role", "gridcell")
                    .attr("aria-colindex", (c + 1).to_string());
                    match stop(Some(r), c) {
                        // A passive key listener makes the tab stop focusable;
                        // its keys bubble to the grid's handler.
                        Some((tabindex, requested)) => Box::new(request_focus(
                            on_key(
                                placed.attr("tabindex", tabindex),
                                |_: &mut State, _: KeyEvent| {},
                            )
                            .focusable(tabindex == "0"),
                            requested,
                        ))
                            as GridView<State, Action>,
                        None => Box::new(placed) as GridView<State, Action>,
                    }
                })
                .collect();
            let zebra = if r % 2 == 0 {
                "grid-row grid-row-even"
            } else {
                "grid-row grid-row-odd"
            };
            // Per-row class hook: selection / active / status styling the caller
            // owns (every real table needs it), appended to the zebra base.
            let class = match row_class(r) {
                Some(extra) => format!("{zebra} {extra}"),
                None => zebra.to_string(),
            };
            let row = Box::new(
                placed_with(
                    Placement::new(0.0, r as f32 * spec.row_height),
                    format!("width: {width}px; height: {}px;", spec.row_height),
                    cells,
                )
                .attr("class", class)
                .attr("role", "row")
                .attr("aria-rowindex", (r + 2).to_string())
                .attr("data-row", r.to_string()),
            ) as GridView<State, Action>;
            (key, row)
        })
        .collect();
    let strip = placed_with(
        Placement::new(0.0, -scroll),
        format!("width: {width}px; height: 0px;"),
        rows,
    )
    .attr("class", "grid-rows");
    let body = el::<_, State, Action>("div", strip).attr("class", "grid-body").attr(
        "style",
        format!(
            "position: relative; overflow-x: hidden; overflow-y: hidden; width: {width}px; height: {viewport_height}px;"
        ),
    );

    let grid = el::<_, State, Action>("div", (header, body))
        .attr("class", "grid")
        .attr("role", "grid")
        .attr("aria-rowcount", (total_rows + 1).to_string())
        .attr("aria-colcount", spec.columns.len().to_string())
        .attr("style", format!("display: block; width: {width}px;"));
    // The grid hears every key from its cells but is not a tab stop itself.
    let handler = move |s: &mut State, event: KeyEvent| on_key_event(s, event);
    match &options.label {
        Some(label) => {
            Box::new(on_key(grid.attr("aria-label", label.clone()), handler).focusable(false))
        },
        None => Box::new(on_key(grid, handler).focusable(false)),
    }
}

#[cfg(test)]
mod tests;
