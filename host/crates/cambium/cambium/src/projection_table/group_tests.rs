// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The table's filter, grouping, heading bands, collapse and several-key
//! header gesture (table projection plan, T5).

use layout_dom_api::LayoutDom;
use sceno::Size2;
use scenograph::SortKey;
use scenograph::filter::{Condition, FilterOp, FilterValue};
use scenograph::table::CollapsedGroups;
use scenomise::projection::{ItemSizes, ProjectionCompiler};

use super::tests::{attr, dataset, draft, find_role, open, runner, text};
use super::*;
use crate::{Key, KeyEvent, Modifiers, NamedKey, PointerClick};

fn by_read() -> Vec<SortKey> {
    vec![SortKey::descending("read")]
}

fn status(table: ProjectionTable) -> String {
    let (dom, runner) = runner(table);
    let d = dom.borrow();
    let mut found = Vec::new();
    find_role(&d, runner.root(), "status", &mut found);
    text(&d, found[0])
}

#[test]
fn a_grouping_draws_a_heading_row_with_its_count_and_counts_it_in_the_grid() {
    let mut table = open(30);
    table.apply_group(by_read(), 0);
    assert_eq!(table.group_count(), 2);
    // Read first (descending: true before false); 10 of 30 are read.
    assert_eq!(
        table.band(0),
        Some(TableBandView::Heading(TableHeadingView {
            group: 0,
            key: r#"[{"field":"read","value":{"kind":"boolean","value":true}}]"#.into(),
            level: 0,
            label: "Read: Yes".into(),
            count: 10,
            collapsed: false,
        }))
    );
    assert_eq!(table.band(1), Some(TableBandView::Row(0)));
    assert!(
        matches!(table.band(11), Some(TableBandView::Heading(h)) if h.label == "Read: No" && h.count == 20)
    );
    assert_eq!(table.band_count(), 32);
    let (dom, runner) = runner(table);
    let d = dom.borrow();
    let mut grid = Vec::new();
    find_role(&d, runner.root(), "grid", &mut grid);
    assert_eq!(
        attr(&d, grid[0], "aria-rowcount"),
        "33",
        "headings are rows"
    );
    let mut headings = Vec::new();
    find_role(&d, runner.root(), "rowheader", &mut headings);
    assert!(!headings.is_empty());
    assert_eq!(attr(&d, headings[0], "aria-colspan"), "6");
    assert_eq!(text(&d, headings[0]), "Read: Yes10 rows");
}

#[test]
fn a_heading_collapses_as_view_state_and_selects_no_row() {
    let (dom, mut runner) = runner(open(30));
    runner.update(|table| table.apply_group(by_read(), 0));
    let heading_row = {
        let d = dom.borrow();
        let mut rows = Vec::new();
        find_role(&d, runner.root(), "row", &mut rows);
        *rows
            .iter()
            .find(|&&row| attr(&d, row, "aria-expanded") == "true")
            .expect("a heading row")
    };
    runner.dispatch_click(heading_row, PointerClick::at((2.0, 2.0)));
    let table = runner.state();
    assert!(matches!(table.band(0), Some(TableBandView::Heading(h)) if h.collapsed));
    assert!(matches!(table.band(1), Some(TableBandView::Heading(h)) if h.label == "Read: No"));
    assert_eq!(table.band_count(), 22);
    assert_eq!(table.band_row(0), None, "a heading is no row");
    // The recipe and its history did not move: one step, the grouping.
    assert_eq!(table.draft().reading.group, by_read());
    let saved = serde_json::to_string(table.collapsed()).unwrap();
    {
        let d = dom.borrow();
        let mut rows = Vec::new();
        find_role(&d, runner.root(), "row", &mut rows);
        assert!(
            rows.iter()
                .any(|&row| attr(&d, row, "aria-expanded") == "false")
        );
    }
    runner.update(|table| assert!(table.undo()));
    assert!(!runner.state().editor().can_undo());
    // A reopened table takes the saved collapse back.
    let mut reopened = open(30);
    reopened.apply_group(by_read(), 0);
    reopened.set_collapsed(serde_json::from_str::<CollapsedGroups>(&saved).unwrap());
    assert_eq!(reopened.band_count(), 22);
}

#[test]
fn the_keyboard_stops_on_a_heading_and_enter_collapses_it() {
    let (_dom, mut runner) = runner(open(30));
    runner.update(|table| table.apply_group(by_read(), 0));
    runner.focus_traverse(true);
    runner.dispatch_key(KeyEvent::new(Key::Named(NamedKey::ArrowDown)));
    assert_eq!(runner.state().focus().row, Some(0));
    runner.dispatch_key(KeyEvent::new(Key::Named(NamedKey::Enter)));
    assert!(matches!(runner.state().band(0), Some(TableBandView::Heading(h)) if h.collapsed));
    runner.dispatch_key(KeyEvent::new(Key::Named(NamedKey::ArrowDown)));
    assert_eq!(runner.state().focus().row, Some(1), "onto the next heading");
    runner.dispatch_key(KeyEvent::new(Key::Named(NamedKey::Space)));
    assert_eq!(runner.state().band_count(), 2, "both collapsed");
}

#[test]
fn shift_adds_a_sort_key_a_plain_click_replaces_and_headers_show_priority() {
    let (dom, mut runner) = runner(open(20));
    let headers = {
        let d = dom.borrow();
        let mut headers = Vec::new();
        find_role(&d, runner.root(), "columnheader", &mut headers);
        headers
    };
    let shift = Modifiers {
        shift: true,
        ..Modifiers::default()
    };
    runner.dispatch_click(headers[2], PointerClick::at((1.0, 1.0)));
    runner.dispatch_click(
        headers[1],
        PointerClick::at((1.0, 1.0)).with_modifiers(shift),
    );
    runner.dispatch_click(
        headers[3],
        PointerClick::at((1.0, 1.0)).with_modifiers(shift),
    );
    runner.dispatch_click(
        headers[1],
        PointerClick::at((1.0, 1.0)).with_modifiers(shift),
    );
    assert_eq!(
        runner.state().draft().reading.sort,
        [
            SortKey::ascending("read"),
            SortKey::descending("pages"),
            SortKey::ascending("added"),
        ]
    );
    {
        let d = dom.borrow();
        let priorities: Vec<String> = headers.iter().map(|&h| text(&d, h)).collect();
        assert_eq!(priorities[..4], ["Title", "Pages2", "Read1", "Added3"]);
        assert_eq!(attr(&d, headers[2], "aria-sort"), "ascending");
        assert_eq!(
            attr(&d, headers[1], "aria-sort"),
            "none",
            "ARIA marks the lead key"
        );
    }
    // Shift+Enter on a focused header adds too.
    runner.focus_traverse(true);
    runner.dispatch_key(KeyEvent::with_mods(Key::Named(NamedKey::Enter), shift));
    assert_eq!(runner.state().draft().reading.sort.len(), 4);
    runner.dispatch_click(headers[0], PointerClick::at((1.0, 1.0)));
    assert_eq!(
        runner.state().draft().reading.sort,
        [SortKey::ascending("title")]
    );
}

#[test]
fn a_filter_states_its_conditions_and_the_frozen_form_leaves_out_what_it_drops() {
    let mut table = open(30);
    table.apply_filter(
        vec![
            Condition::new("pages", FilterOp::Ge, Some(FilterValue::Number(300.0))),
            Condition::new(
                "title",
                FilterOp::Contains,
                Some(FilterValue::Text("1".into())),
            ),
        ],
        0,
    );
    let kept = table.row_count();
    assert!(kept > 0 && kept < 30, "{kept}");
    table.apply_group(by_read(), 0);
    table.apply_sort(vec![SortKey::ascending("pages")], 0);
    let frozen = table.frozen();
    assert_eq!(frozen.rows.len(), kept);
    assert!(frozen.groups.iter().all(|g| g.level == 0));
    let html = frozen.to_html("shelf");
    assert_eq!(
        html.matches("scope=\"rowgroup\"").count(),
        table.group_count()
    );
    assert_eq!(
        status(table),
        format!(
            "{kept} of 30 rows, where Pages is at least 300 and Title contains \u{201c}1\u{201d}, grouped by Read, descending, sorted by Pages, ascending"
        )
    );
}

#[test]
fn a_filter_or_grouping_change_reads_rows_as_a_fresh_compile_does() {
    let filters = [
        Vec::new(),
        vec![Condition::new(
            "tags",
            FilterOp::ContainsItem,
            Some(FilterValue::Text("t2".into())),
        )],
        vec![Condition::new("pages", FilterOp::IsEmpty, None)],
    ];
    let groups = [
        Vec::new(),
        vec![SortKey::ascending("author")],
        vec![SortKey::descending("read"), SortKey::ascending("tags")],
    ];
    for filter in &filters {
        for group in &groups {
            let mut changed = open(200);
            changed.apply_sort(vec![SortKey::ascending("pages")], 0);
            changed.apply_filter(filter.clone(), 0);
            changed.apply_group(group.clone(), 0);
            let mut fresh_draft = draft();
            fresh_draft.reading.sort = vec![SortKey::ascending("pages")];
            fresh_draft.reading.filter = filter.clone();
            fresh_draft.reading.group = group.clone();
            let compiler = ProjectionCompiler::new(ItemSizes {
                card: Size2::new(120.0, 24.0),
            });
            let fresh = ProjectionTable::open(
                fresh_draft,
                dataset(200),
                compiler,
                TableViewers::built_in(),
            )
            .unwrap();
            let bands = |table: &ProjectionTable| {
                (0..table.band_count())
                    .map(|band| match table.band(band).unwrap() {
                        TableBandView::Row(rank) => table.row_id(rank).unwrap().to_owned(),
                        TableBandView::Heading(h) => format!("{} ({})", h.label, h.count),
                    })
                    .collect::<Vec<_>>()
            };
            assert_eq!(bands(&changed), bands(&fresh), "{filter:?} {group:?}");
            assert_eq!(changed.frozen(), fresh.frozen());
        }
    }
}

#[test]
fn a_refused_filter_is_stepped_back_and_names_the_field() {
    let mut table = open(10);
    table.apply_filter(
        vec![Condition::new(
            "read",
            FilterOp::Gt,
            Some(FilterValue::Boolean(true)),
        )],
        0,
    );
    assert!(table.draft().reading.filter.is_empty());
    assert!(
        table
            .notice
            .as_deref()
            .is_some_and(|n| n.contains("\"read\"")),
        "{:?}",
        table.notice
    );
    table.apply_group(vec![SortKey::ascending("shelf")], 0);
    assert!(table.draft().reading.group.is_empty());
    assert!(
        table
            .notice
            .as_deref()
            .is_some_and(|n| n.contains("\"shelf\""))
    );
}

#[test]
fn a_grouped_field_s_header_shows_the_group_s_direction_and_no_rank() {
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
        table.apply_group(by_read(), 0);
    });
    let d = dom.borrow();
    let mut headers = Vec::new();
    find_role(&d, runner.root(), "columnheader", &mut headers);
    let classes_under = |node| {
        let mut out = Vec::new();
        let mut stack = vec![node];
        while let Some(node) = stack.pop() {
            out.push(attr(&d, node, "class"));
            stack.extend(d.dom_children(node));
        }
        out
    };
    // Read: the grouping's direction, a group mark, no rank (table D35).
    assert_eq!(attr(&d, headers[2], "aria-sort"), "descending");
    assert_eq!(text(&d, headers[2]), "Read");
    let read = classes_under(headers[2]);
    assert!(
        read.iter().any(|c| c == "grid-sort grid-sort-desc"),
        "{read:?}"
    );
    assert!(
        read.iter().any(|c| c == "grid-group grid-group-on"),
        "{read:?}"
    );
    // The sort within the groups ranks its other keys from one.
    assert_eq!(text(&d, headers[1]), "Pages1");
    assert_eq!(text(&d, headers[3]), "Added2");
    assert_eq!(attr(&d, headers[1], "aria-sort"), "none");
    assert!(
        !classes_under(headers[1])
            .iter()
            .any(|c| c == "grid-group grid-group-on")
    );
    let html = runner.state().frozen().to_html("shelf");
    assert!(html.contains("aria-sort=\"descending\">Read</th>"));
    assert_eq!(html.matches("aria-sort=").count(), 1);
}

#[test]
fn a_collapse_moves_headings_and_rows_by_their_keys_without_rebuilding_them() {
    use layout_dom_api::{DomMutation, LayoutDomMut as _};
    let mut table = open(30);
    table.viewport_height = 720.0;
    let (dom, mut runner) = runner(table);
    runner.update(|table| table.apply_group(by_read(), 0));
    // Headings by label and rows by their title cell, with their nodes.
    let nodes = |runner: &crate::GenetAppRunner<_, _, _, ()>| {
        let d = dom.borrow();
        let mut rows = Vec::new();
        find_role(&d, runner.root(), "row", &mut rows);
        rows.into_iter()
            .filter(|&row| attr(&d, row, "aria-rowindex") != "1")
            .map(|row| {
                let words = text(&d, row);
                let name = words.split("rows").next().unwrap_or_default().to_owned();
                (name, row)
            })
            .collect::<std::collections::HashMap<_, _>>()
    };
    let before = nodes(&runner);
    let no = before
        .keys()
        .find(|name| name.starts_with("Read: No"))
        .expect("the second heading is drawn")
        .clone();
    dom.borrow_mut().drain_mutations(&mut Vec::new());
    runner.update(|table| table.toggle_group(0));
    let mut mutations = Vec::new();
    dom.borrow_mut().drain_mutations(&mut mutations);
    let after = nodes(&runner);
    // The second heading moved up from band 11 to band 1 and kept its node.
    assert_eq!(before[&no], after[&no], "the heading is kept by its group");
    // Every row still drawn after the collapse is the node it was before.
    let kept: Vec<_> = after
        .keys()
        .filter(|name| before.contains_key(*name))
        .collect();
    assert_eq!(
        kept.len(),
        after.len(),
        "nothing drawn after was built anew"
    );
    for name in kept {
        assert_eq!(before[name], after[name], "{name} kept its node");
    }
    // Their text is not written again: only the collapsed heading's words
    // and marks change, and moved bands change their place.
    let rewritten = mutations
        .iter()
        .filter(|m| matches!(m, DomMutation::CharacterDataChanged { .. }))
        .count();
    assert_eq!(rewritten, 0, "{mutations:?}");
}
