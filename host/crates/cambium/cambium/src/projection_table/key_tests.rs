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
