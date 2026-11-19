// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The data grid's tests: virtualization, keyed rows and header clicks.

use super::*;
use crate::tags::custom_leaf;
use crate::{DomHandle, GenetAppRunner};
use genet_scripted_dom::{NodeId, ScriptedDom};
use layout_dom_api::{LayoutDom, LocalName, Namespace};
use sprigging::GridColumn;
use std::cell::RefCell;
use std::rc::Rc;

fn spec() -> GridSpec {
    GridSpec {
        columns: vec![
            GridColumn::new("name", 120.0),
            GridColumn::new("value", 80.0),
            GridColumn::new("trend", 100.0),
        ],
        row_height: 24.0,
        header_height: 28.0,
        overscan: 2,
    }
}

struct GridState {
    scroll: f32,
    descending: bool,
    rows: usize,
}

fn view(state: &GridState) -> GridView<GridState, ()> {
    let rows = state.rows;
    let descending = state.descending;
    data_grid(
        &spec(),
        rows,
        300.0,
        state.scroll,
        move |r, c| {
            // Sort order is caller state: ascending shows r, descending
            // shows the mirrored index.
            let i = if descending { rows - 1 - r } else { r };
            if c == 2 {
                // Sparkline-in-cell shape: a custom leaf as a cell view.
                Box::new(custom_leaf::<GridState, ()>(1000 + i as u64, 96, 16))
            } else {
                Box::new(el::<_, GridState, ()>("span", format!("r{i}c{c}")))
            }
        },
        |s: &mut GridState, _col| s.descending = !s.descending,
        // Mark row 0 selected, to exercise the per-row class hook.
        |r| (r == 0).then(|| "grid-row-selected".to_string()),
    )
}

fn class_of(dom: &ScriptedDom, node: NodeId) -> String {
    dom.attribute(node, &Namespace::from(""), &LocalName::from("class"))
        .unwrap_or_default()
        .to_string()
}

fn find_all(dom: &ScriptedDom, node: NodeId, class: &str, out: &mut Vec<NodeId>) {
    if class_of(dom, node).split_whitespace().any(|c| c == class) {
        out.push(node);
    }
    for child in dom.dom_children(node) {
        find_all(dom, child, class, out);
    }
}

fn texts_under(dom: &ScriptedDom, node: NodeId, out: &mut String) {
    if let Some(t) = dom.text(node) {
        out.push_str(t);
    }
    for child in dom.dom_children(node) {
        texts_under(dom, child, out);
    }
}

/// The composed done conditions: a 10k-row grid materializes only the
/// window (sticky header always present), scrolling shifts rows without
/// growing the DOM, and a custom leaf rides as a cell view.
#[test]
fn grid_virtualizes_rows_under_a_sticky_header() {
    let dom: DomHandle = Rc::new(RefCell::new(ScriptedDom::new()));
    let mut runner = GenetAppRunner::<_, _, _, ()>::new(
        dom.clone(),
        view,
        GridState {
            scroll: 0.0,
            descending: false,
            rows: 10_000,
        },
    );
    let root = runner.root();
    {
        let d = dom.borrow();
        let mut headers = Vec::new();
        find_all(&d, root, "grid-header-cell", &mut headers);
        assert_eq!(headers.len(), 3, "one header cell per column");
        let mut rows = Vec::new();
        find_all(&d, root, "grid-row", &mut rows);
        assert!(
            rows.len() < 25,
            "10k rows materialize a window, got {}",
            rows.len()
        );
        // The per-row class hook rides on the row's class: row 0 is the
        // window's first row at scroll 0 and the view marks it selected.
        let mut selected = Vec::new();
        find_all(&d, root, "grid-row-selected", &mut selected);
        assert_eq!(selected.len(), 1, "row_class marks exactly row 0 selected");
        assert!(
            class_of(&d, selected[0]).contains("grid-row"),
            "hook appends to the zebra base"
        );
        let mut leaves = Vec::new();
        find_all(&d, root, "grid-cell", &mut leaves);
        assert_eq!(
            leaves.len(),
            rows.len() * 3,
            "three cells per materialized row"
        );
    }

    runner.update(|s| s.scroll = 4800.0);
    let d = dom.borrow();
    let mut rows = Vec::new();
    find_all(&d, root, "grid-row", &mut rows);
    assert!(rows.len() < 25, "window stays a sliver after scroll");
    let first_row_class = d
        .attribute(rows[0], &Namespace::from(""), &LocalName::from("data-row"))
        .unwrap_or_default();
    assert_eq!(
        first_row_class, "198",
        "window starts at scroll/24 minus overscan"
    );
    // The sparkline cell rides along: a custom leaf inside the row.
    let mut text = String::new();
    texts_under(&d, rows[0], &mut text);
    assert!(
        text.contains("r198c0"),
        "cell content reflects the row: {text}"
    );
}

/// A one-row window shift moves rows rather than rewriting them: the rows
/// in both windows keep their nodes and none of their attributes or text
/// is written; the DOM sees the arriving and leaving rows.
#[test]
fn a_window_shift_keeps_the_surviving_rows() {
    use layout_dom_api::{DomMutation, LayoutDomMut as _};
    let dom: DomHandle = Rc::new(RefCell::new(ScriptedDom::new()));
    let mut runner = GenetAppRunner::<_, _, _, ()>::new(
        dom.clone(),
        view,
        GridState {
            scroll: 240.0,
            descending: false,
            rows: 10_000,
        },
    );
    let root = runner.root();
    let rows_by_index = |d: &ScriptedDom| {
        let mut rows = Vec::new();
        find_all(d, root, "grid-row", &mut rows);
        rows.into_iter()
            .map(|node| {
                let index = d
                    .attribute(node, &Namespace::from(""), &LocalName::from("data-row"))
                    .unwrap_or_default()
                    .to_string();
                (index, node)
            })
            .collect::<std::collections::HashMap<_, _>>()
    };
    let before = rows_by_index(&dom.borrow());
    // Every node under the window's rows before the shift.
    let mut existing = std::collections::HashSet::new();
    {
        let d = dom.borrow();
        let mut pending: Vec<_> = before.values().copied().collect();
        while let Some(node) = pending.pop() {
            existing.insert(node);
            pending.extend(d.dom_children(node));
        }
    }
    dom.borrow_mut().drain_mutations(&mut Vec::new());
    runner.update(|s| s.scroll = 264.0);
    let mut mutations = Vec::new();
    dom.borrow_mut().drain_mutations(&mut mutations);
    let after = rows_by_index(&dom.borrow());
    let kept: Vec<_> = before.keys().filter(|k| after.contains_key(*k)).collect();
    assert_eq!(
        kept.len(),
        before.len() - 1,
        "one row leaves, the rest stay"
    );
    for index in kept {
        assert_eq!(before[index], after[index], "row {index} kept its node");
    }
    let rewrites = mutations
        .iter()
        .filter(|m| match m {
            DomMutation::AttributeChanged { node, .. }
            | DomMutation::CharacterDataChanged { node } => existing.contains(node),
            _ => false,
        })
        .count();
    assert_eq!(rewrites, 0, "no surviving row is rewritten: {mutations:?}");
}

/// Header click routes through the grid's on_click wiring into caller
/// state: the sort flips and the same materialized window shows the
/// mirrored order.
#[test]
fn header_click_flips_the_sort() {
    let dom: DomHandle = Rc::new(RefCell::new(ScriptedDom::new()));
    let mut runner = GenetAppRunner::<_, _, _, ()>::new(
        dom.clone(),
        view,
        GridState {
            scroll: 0.0,
            descending: false,
            rows: 100,
        },
    );
    let root = runner.root();
    let (header, first_row) = {
        let d = dom.borrow();
        let mut headers = Vec::new();
        find_all(&d, root, "grid-header-cell", &mut headers);
        let mut rows = Vec::new();
        find_all(&d, root, "grid-row", &mut rows);
        (headers[0], rows[0])
    };
    {
        let d = dom.borrow();
        let mut text = String::new();
        texts_under(&d, first_row, &mut text);
        assert!(text.contains("r0c0"), "ascending starts at row 0: {text}");
    }

    runner.dispatch_click(header, PointerClick::at((1.0, 1.0)));

    let d = dom.borrow();
    let mut rows = Vec::new();
    find_all(&d, root, "grid-row", &mut rows);
    let mut text = String::new();
    texts_under(&d, rows[0], &mut text);
    assert!(
        text.contains("r99c0"),
        "descending shows the mirrored index in the same window: {text}",
    );
}

/// A plain grid kept by the caller's keys: a reorder moves each row's node
/// and writes none of its text (table D42).
#[test]
fn a_keyed_plain_grid_keeps_row_nodes_across_a_reorder() {
    use layout_dom_api::{DomMutation, LayoutDomMut as _};
    struct Items {
        order: Vec<usize>,
    }
    fn keyed(state: &Items) -> GridView<Items, ()> {
        let order = state.order.clone();
        let keys = order.clone();
        data_grid_keyed(
            &spec(),
            order.len(),
            300.0,
            0.0,
            move |r, c| Box::new(el::<_, Items, ()>("span", format!("item{}c{c}", order[r]))),
            move |r| GridRowKey::Id(format!("item{}", keys[r])),
            |_: &mut Items, _| {},
            |_| None,
            &GridOptions::default(),
            |_: &mut Items, _: KeyEvent| {},
        )
    }
    let dom: DomHandle = Rc::new(RefCell::new(ScriptedDom::new()));
    let mut runner = GenetAppRunner::<_, _, _, ()>::new(
        dom.clone(),
        keyed as fn(&Items) -> GridView<Items, ()>,
        Items {
            order: (0..8).collect(),
        },
    );
    let root = runner.root();
    let rows_by_text = |d: &ScriptedDom| {
        let mut rows = Vec::new();
        find_all(d, root, "grid-row", &mut rows);
        rows.into_iter()
            .map(|node| {
                let mut text = String::new();
                texts_under(d, node, &mut text);
                (text, node)
            })
            .collect::<std::collections::HashMap<_, _>>()
    };
    let before = rows_by_text(&dom.borrow());
    assert_eq!(before.len(), 8);
    dom.borrow_mut().drain_mutations(&mut Vec::new());
    runner.update(|s| s.order.reverse());
    let mut mutations = Vec::new();
    dom.borrow_mut().drain_mutations(&mut mutations);
    let after = rows_by_text(&dom.borrow());
    for (text, node) in &after {
        assert_eq!(before.get(text), Some(node), "{text} kept its node");
    }
    let written = mutations
        .iter()
        .filter(|m| matches!(m, DomMutation::CharacterDataChanged { .. }))
        .count();
    assert_eq!(written, 0, "no row's text is written again");
}
