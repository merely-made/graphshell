// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Rows kept by occurrence id (table D41) and the status line under a
//! grouping (table D40).

use std::collections::HashMap;

use genet_scripted_dom::NodeId;
use layout_dom_api::{DomMutation, LayoutDom as _, LayoutDomMut as _};
use scenograph::SortKey;

use super::tests::{attr, find_role, open, runner, text};

/// Body rows by their text (a title is unique in the fixture), with nodes.
fn rows_by_text(dom: &crate::DomHandle, root: NodeId) -> HashMap<String, NodeId> {
    let d = dom.borrow();
    let mut rows = Vec::new();
    find_role(&d, root, "row", &mut rows);
    rows.into_iter()
        .filter(|&row| attr(&d, row, "aria-rowindex") != "1")
        .map(|row| (text(&d, row), row))
        .collect()
}

#[test]
fn the_status_line_names_only_the_sort_keys_that_order_rows_in_the_groups() {
    let (dom, mut runner) = runner(open(20));
    runner.update(|table| {
        table.apply_sort(
            vec![
                SortKey::ascending("read"),
                SortKey::ascending("pages"),
                SortKey::ascending("added"),
            ],
            0,
        );
        table.apply_group(vec![SortKey::descending("read")], 0);
    });
    let d = dom.borrow();
    let mut status = Vec::new();
    find_role(&d, runner.root(), "status", &mut status);
    assert_eq!(
        text(&d, status[0]),
        "20 rows, grouped by Read, descending, sorted by Pages, ascending, then Added, ascending"
    );
}

#[test]
fn a_resort_moves_rows_by_their_id_and_writes_no_text() {
    let mut table = open(20);
    table.viewport_height = 720.0;
    let (dom, mut runner) = runner(table);
    let root = runner.root();
    let before = rows_by_text(&dom, root);
    assert_eq!(before.len(), 20, "every row is drawn");
    // Every node inside a row; the status line's words do change.
    let mut inside = std::collections::HashSet::new();
    {
        let d = dom.borrow();
        let mut pending: Vec<NodeId> = before.values().copied().collect();
        while let Some(node) = pending.pop() {
            inside.insert(node);
            pending.extend(d.dom_children(node));
        }
    }
    dom.borrow_mut().drain_mutations(&mut Vec::new());
    runner.update(|table| table.apply_sort(vec![SortKey::descending("pages")], 0));
    let mut mutations = Vec::new();
    dom.borrow_mut().drain_mutations(&mut mutations);
    let after = rows_by_text(&dom, root);
    assert_eq!(after.len(), 20);
    for (row, node) in &after {
        assert_eq!(before.get(row), Some(node), "{row} kept its node");
    }
    let written = mutations
        .iter()
        .filter(
            |m| matches!(m, DomMutation::CharacterDataChanged { node } if inside.contains(node)),
        )
        .count();
    assert_eq!(written, 0, "no row's text is written again");
}

#[test]
fn a_one_row_scroll_builds_only_the_arriving_row() {
    let (dom, mut runner) = runner(open(10_000));
    runner.update(|table| table.scroll_by(2400.0));
    let root = runner.root();
    let before = rows_by_text(&dom, root);
    dom.borrow_mut().drain_mutations(&mut Vec::new());
    runner.update(|table| {
        let step = table.row_height();
        table.scroll_by(step);
    });
    let mut mutations = Vec::new();
    dom.borrow_mut().drain_mutations(&mut mutations);
    let after = rows_by_text(&dom, root);
    let kept = after.keys().filter(|row| before.contains_key(*row)).count();
    assert_eq!(kept, after.len() - 1, "one row arrives");
    for row in after.keys().filter(|row| before.contains_key(*row)) {
        assert_eq!(before[row], after[row]);
    }
    // For the receipts: the mutations a one-row shift costs.
    println!("one-row shift: {} mutations", mutations.len());
    assert!(mutations.len() < 120, "{}", mutations.len());
}

#[test]
fn scrolling_lands_on_whole_rows_and_the_body_holds_whole_rows() {
    let mut table = open(100);
    let row = table.row_height();
    table.viewport_height = row * 10.0 + 7.0;
    assert_eq!(table.body_height(), row * 10.0);
    // Fine wheel travel adds up to a row.
    table.scroll_by(row * 0.4);
    assert_eq!(table.scroll, 0.0);
    table.scroll_by(row * 0.4);
    table.scroll_by(row * 0.4);
    assert_eq!(table.scroll, row);
    table.scroll_by(row * 2.6);
    assert_eq!(table.scroll % row, 0.0);
    // The end shows the last row whole.
    table.scroll_by(1.0e9);
    assert_eq!(table.scroll, table.max_scroll());
    assert_eq!(table.scroll + table.body_height(), 100.0 * row);
    table.scroll_to(row * 3.3);
    assert_eq!(table.scroll, row * 3.0);
}

#[test]
fn the_focus_follows_its_row_through_a_sort_and_is_shown() {
    let mut table = open(200);
    table.focus_cell(Some(3), 1);
    let id = table.row_id(3).expect("a row").to_owned();
    table.apply_sort(vec![SortKey::descending("pages")], 0);
    let band = table.focus().row.expect("still on a row");
    assert_eq!(
        table.band_row(band).and_then(|rank| table.row_id(rank)),
        Some(id.as_str())
    );
    let top = band as f32 * table.row_height();
    assert!(top >= table.scroll && top + table.row_height() <= table.scroll + table.body_height());
    // Grouping puts headings between the rows; ungrouping takes them away.
    table.apply_group(vec![SortKey::ascending("read")], 0);
    let band = table.focus().row.expect("still on a row");
    assert_eq!(
        table.band_row(band).and_then(|rank| table.row_id(rank)),
        Some(id.as_str())
    );
    table.apply_group(Vec::new(), 0);
    let band = table.focus().row.expect("still on a row");
    assert_eq!(
        table.band_row(band).and_then(|rank| table.row_id(rank)),
        Some(id.as_str())
    );
}

#[test]
fn a_pressed_header_takes_the_focus_and_a_focused_heading_is_marked() {
    let mut table = open(20);
    table.viewport_height = 720.0;
    table.focus_cell(Some(2), 0);
    table.press_header(1, false, 0);
    let focus = table.focus();
    assert_eq!((focus.row, focus.col, focus.requested), (None, 1, true));
    table.apply_group(vec![SortKey::ascending("read")], 0);
    table.focus_cell(Some(0), 0);
    assert!(table.toggle_band(0), "band 0 is a heading");
    assert_eq!(
        table.focus().row,
        Some(0),
        "the toggled heading keeps the focus"
    );
    let (dom, runner) = runner(table);
    let d = dom.borrow();
    let mut rows = Vec::new();
    find_role(&d, runner.root(), "row", &mut rows);
    let heading = rows
        .into_iter()
        .find(|&row| attr(&d, row, "aria-rowindex") == "2")
        .expect("the first band");
    let class = attr(&d, heading, "class");
    assert!(
        class.contains("grid-heading") && class.contains("grid-row-focus"),
        "{class}"
    );
}
