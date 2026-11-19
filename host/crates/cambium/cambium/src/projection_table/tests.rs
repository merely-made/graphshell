// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use genet_scripted_dom::{NodeId, ScriptedDom};
use layout_dom_api::{LayoutDom, LocalName, Namespace};
use sceno::{Frame, Size2, SourceRef, Vec2};
use scenograph::dataset::{
    ProjectionDataset, ProjectionFieldType, ProjectionOccurrence, ProjectionValue,
};
use scenograph::{
    Appearance, Arrangement, Channel, Encoding, FieldSpec, Interaction, ProjectionDraft,
    Provenance, Reading, SelectionMode, SortKey, SourceBinding,
};
use scenomise::projection::{ItemSizes, ProjectionCompiler, TABLE_REALIZATION_ID};

use super::*;
use crate::grid::GridView;
use crate::{DomHandle, GenetAppRunner, Key, KeyEvent, Modifiers, NamedKey, PointerClick};

fn source() -> SourceBinding {
    SourceBinding {
        authority: "fixture.local".into(),
        domain: "books".into(),
        resource: "books:shelf".into(),
    }
}

/// `rows` books: a title, pages, read or not, a date, an author reference
/// and a list of tags; every seventh book has no pages.
pub(super) fn dataset(rows: usize) -> ProjectionDataset {
    let occurrences = (0..rows)
        .map(|i| {
            let id = format!("b{i:06}");
            let mut values = BTreeMap::from([
                ("id".into(), ProjectionValue::Text(id.clone())),
                (
                    "title".into(),
                    ProjectionValue::Text(format!("Title {}", (i * 7919) % rows)),
                ),
                ("read".into(), ProjectionValue::Boolean(i % 3 == 0)),
                (
                    "added".into(),
                    ProjectionValue::Date(format!("2026-{:02}-{:02}", 1 + i % 12, 1 + i % 28)),
                ),
                (
                    "author".into(),
                    ProjectionValue::Reference(SourceRef::new("people", format!("p{}", i % 5))),
                ),
                (
                    "tags".into(),
                    ProjectionValue::List(vec![ProjectionValue::Text(format!("t{}", i % 4))]),
                ),
            ]);
            if i % 7 != 3 {
                values.insert(
                    "pages".into(),
                    ProjectionValue::Number(((i * 37) % 900) as f64),
                );
            }
            ProjectionOccurrence::new(id.clone(), SourceRef::new("books", id), values)
        })
        .collect();
    ProjectionDataset {
        source: source(),
        revision: "shelf-r1".into(),
        fields: BTreeMap::from([
            ("id".into(), ProjectionFieldType::Text),
            ("title".into(), ProjectionFieldType::Text),
            ("pages".into(), ProjectionFieldType::Number),
            ("read".into(), ProjectionFieldType::Boolean),
            ("added".into(), ProjectionFieldType::Date),
            ("author".into(), ProjectionFieldType::Reference),
            (
                "tags".into(),
                ProjectionFieldType::List(scenograph::dataset::ProjectionItemType::Text),
            ),
        ]),
        occurrences,
    }
}

fn field(path: &str, name: &str, viewer: Option<&str>) -> FieldSpec {
    let mut field = FieldSpec::new(path, name);
    field.viewer = viewer.map(str::to_owned);
    field
}

pub(super) fn draft() -> ProjectionDraft {
    let mut draft = ProjectionDraft::new();
    draft.id = "shelf".into();
    draft.label = "Shelf".into();
    draft.source = source();
    draft.reading = Reading {
        kind: "nodes".into(),
        key: "id".into(),
        value: None,
        ..Default::default()
    };
    draft.encoding = Encoding {
        x: Channel::Field("id".into()),
        y: Channel::Field("id".into()),
        color: None,
        label: Some(Channel::Field("title".into())),
    };
    draft.arrangement = Arrangement {
        kind: "table".into(),
        direction: "coordinates".into(),
        spacing: 16,
        options: BTreeMap::from([
            ("column_width".into(), "120".into()),
            ("row_height".into(), "24".into()),
            ("header".into(), "28".into()),
        ]),
    };
    draft.frame = Some(Frame::Flow {
        origin: Vec2::ZERO,
        width: 900.0,
    });
    draft.fields = vec![
        field("title", "Title", None),
        field("pages", "Pages", Some("bar")),
        field("read", "Read", Some("check")),
        field("added", "Added", None),
        field("author", "Author", Some("chip")),
        field("tags", "Tags", None),
    ];
    draft.interaction = Interaction {
        selection: SelectionMode::Multiple,
        pan: false,
        zoom: false,
    };
    draft.appearance = Appearance {
        realization: TABLE_REALIZATION_ID.into(),
        title: "Reading list".into(),
        theme: "slate".into(),
    };
    draft.provenance = Provenance {
        author: "test".into(),
        source_revision: Some("shelf-r1".into()),
        ..Default::default()
    };
    draft
}

pub(super) fn open(rows: usize) -> ProjectionTable {
    let compiler = ProjectionCompiler::new(ItemSizes {
        card: Size2::new(120.0, 24.0),
    });
    let mut table =
        ProjectionTable::open(draft(), dataset(rows), compiler, TableViewers::built_in())
            .unwrap_or_else(|issues| panic!("{issues:?}"));
    table.viewport_height = 240.0;
    table
}

fn view(table: &ProjectionTable) -> GridView<ProjectionTable, ()> {
    projection_table(table)
}

pub(super) fn attr(dom: &ScriptedDom, node: NodeId, name: &str) -> String {
    dom.attribute(node, &Namespace::from(""), &LocalName::from(name))
        .unwrap_or_default()
        .to_string()
}

pub(super) fn find_role(dom: &ScriptedDom, node: NodeId, role: &str, out: &mut Vec<NodeId>) {
    if attr(dom, node, "role") == role {
        out.push(node);
    }
    for child in dom.dom_children(node) {
        find_role(dom, child, role, out);
    }
}

pub(super) fn text(dom: &ScriptedDom, node: NodeId) -> String {
    let mut out = dom.text(node).map(str::to_owned).unwrap_or_default();
    for child in dom.dom_children(node) {
        out.push_str(&text(dom, child));
    }
    out
}

pub(super) fn runner(
    table: ProjectionTable,
) -> (
    DomHandle,
    GenetAppRunner<
        ProjectionTable,
        fn(&ProjectionTable) -> GridView<ProjectionTable, ()>,
        GridView<ProjectionTable, ()>,
        (),
    >,
) {
    let dom: DomHandle = Rc::new(RefCell::new(ScriptedDom::new()));
    let runner = GenetAppRunner::new(dom.clone(), view as fn(&_) -> _, table);
    (dom, runner)
}

#[test]
fn a_table_recipe_opens_with_headers_from_field_names_and_only_visible_rows() {
    let (dom, runner) = runner(open(10_000));
    let d = dom.borrow();
    let mut headers = Vec::new();
    find_role(&d, runner.root(), "columnheader", &mut headers);
    let names: Vec<_> = headers.iter().map(|h| text(&d, *h)).collect();
    assert_eq!(names, ["Title", "Pages", "Read", "Added", "Author", "Tags"]);
    let mut rows = Vec::new();
    find_role(&d, runner.root(), "row", &mut rows);
    // A 240 px body of 24 px rows is ten rows, plus overscan and the header.
    assert!(
        rows.len() <= 14,
        "only the window is drawn, got {}",
        rows.len()
    );
    let mut grids = Vec::new();
    find_role(&d, runner.root(), "grid", &mut grids);
    assert_eq!(attr(&d, grids[0], "aria-rowcount"), "10001");
    assert_eq!(attr(&d, grids[0], "aria-label"), "Reading list");
}

#[test]
fn columns_follow_the_fields_order() {
    let mut reordered = draft();
    reordered.fields.reverse();
    let compiler = ProjectionCompiler::new(ItemSizes {
        card: Size2::new(120.0, 24.0),
    });
    let table =
        ProjectionTable::open(reordered, dataset(5), compiler, TableViewers::built_in()).unwrap();
    let names: Vec<_> = table.columns().iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["Tags", "Author", "Added", "Read", "Pages", "Title"]);
}

#[test]
fn clicking_a_header_sorts_saves_on_the_recipe_and_is_one_undo_step() {
    let (dom, mut runner) = runner(open(200));
    let pages = {
        let d = dom.borrow();
        let mut headers = Vec::new();
        find_role(&d, runner.root(), "columnheader", &mut headers);
        headers[1]
    };
    runner.dispatch_click(pages, PointerClick::at((1.0, 1.0)));
    let table = runner.state();
    assert_eq!(table.draft().reading.sort, [SortKey::ascending("pages")]);
    let pages_of = |table: &ProjectionTable, rank: usize| table.cell(rank, 1);
    assert_eq!(
        pages_of(table, 0),
        CellContent::Bar {
            fraction: 0.0,
            text: "0".into()
        }
    );
    // Missing pages sort last.
    assert_eq!(pages_of(table, table.row_count() - 1), CellContent::Empty);
    {
        let d = dom.borrow();
        assert_eq!(attr(&d, pages, "aria-sort"), "ascending");
    }
    runner.dispatch_click(pages, PointerClick::at((1.0, 1.0)));
    assert_eq!(
        runner.state().draft().reading.sort,
        [SortKey::descending("pages")]
    );
    runner.update(|table| assert!(table.undo()));
    assert_eq!(
        runner.state().draft().reading.sort,
        [SortKey::ascending("pages")]
    );
    runner.update(|table| assert!(table.undo()));
    assert!(runner.state().draft().reading.sort.is_empty());
    assert!(!runner.state().editor().can_undo());
}

#[test]
fn the_sort_indicator_is_a_drawn_box_and_no_glyph() {
    let (dom, mut runner) = runner(open(200));
    let pages = {
        let d = dom.borrow();
        let mut headers = Vec::new();
        find_role(&d, runner.root(), "columnheader", &mut headers);
        headers[1]
    };
    let marks = |runner: &GenetAppRunner<_, _, _, ()>| {
        let d = dom.borrow();
        let _ = runner;
        let mut out = Vec::new();
        let mut stack = vec![pages];
        while let Some(node) = stack.pop() {
            if attr(&d, node, "class").starts_with("grid-sort") {
                out.push(attr(&d, node, "class"));
            }
            stack.extend(d.dom_children(node));
        }
        (out, text(&d, pages))
    };
    assert_eq!(
        marks(&runner),
        (vec!["grid-sort".to_owned()], "Pages".to_owned())
    );
    runner.dispatch_click(pages, PointerClick::at((1.0, 1.0)));
    assert_eq!(
        marks(&runner),
        (
            vec!["grid-sort grid-sort-asc".to_owned()],
            "Pages".to_owned()
        )
    );
    runner.dispatch_click(pages, PointerClick::at((1.0, 1.0)));
    assert_eq!(
        marks(&runner),
        (
            vec!["grid-sort grid-sort-desc".to_owned()],
            "Pages".to_owned()
        )
    );
}

#[test]
fn the_keyboard_reaches_every_header_and_cell() {
    let (_dom, mut runner) = runner(open(30));
    let key = |runner: &mut GenetAppRunner<_, _, _, ()>, named: NamedKey, ctrl: bool| {
        runner.dispatch_key(KeyEvent::with_mods(
            Key::Named(named),
            Modifiers {
                ctrl,
                ..Modifiers::default()
            },
        ));
    };
    // Tab enters at the first header: the grid's one tab stop.
    runner.focus_traverse(true);
    let entered = runner.focus().expect("tab lands in the grid");
    let columns = runner.state().columns().len();
    let rows = runner.state().row_count();
    let mut seen = std::collections::BTreeSet::new();
    let _ = entered;
    for row in 0..=rows {
        for col in 0..columns {
            let focus = runner.state().focus();
            seen.insert((focus.row, focus.col));
            // The focused node is the cell the state names.
            let node = runner.focus().expect("a focused node");
            let d = runner.dom();
            let d = d.borrow();
            let role = attr(&d, node, "role");
            let expected = if focus.row.is_none() {
                "columnheader"
            } else {
                "gridcell"
            };
            assert_eq!(
                role,
                expected,
                "at {:?}: class {:?} text {:?}",
                (focus.row, focus.col),
                attr(&d, node, "class"),
                text(&d, node)
            );
            drop(d);
            if col + 1 < columns {
                key(&mut runner, NamedKey::ArrowRight, false);
            }
        }
        key(&mut runner, NamedKey::Home, false);
        if row < rows {
            key(&mut runner, NamedKey::ArrowDown, false);
        }
    }
    assert_eq!(seen.len(), (rows + 1) * columns, "every header and cell");
    key(&mut runner, NamedKey::Home, true);
    assert_eq!(runner.state().focus().row, None);
    key(&mut runner, NamedKey::End, true);
    let focus = runner.state().focus();
    assert_eq!((focus.row, focus.col), (Some(rows - 1), columns - 1));
    // The last row is scrolled into the body.
    let table = runner.state();
    assert!(table.scroll + table.viewport_height >= rows as f32 * table.row_height());
}

#[test]
fn enter_on_a_focused_header_sorts() {
    let (_dom, mut runner) = runner(open(20));
    runner.focus_traverse(true);
    runner.dispatch_key(KeyEvent::new(Key::Named(NamedKey::ArrowRight)));
    runner.dispatch_key(KeyEvent::new(Key::Named(NamedKey::Enter)));
    assert_eq!(
        runner.state().draft().reading.sort,
        [SortKey::ascending("pages")]
    );
}

#[test]
fn cells_show_each_type_plainly_and_the_three_viewers() {
    let table = open(10);
    // Row order follows x, the id: b000000 first.
    assert_eq!(table.row_id(0), Some("b000000"));
    assert_eq!(table.cell(0, 0), CellContent::Text("Title 0".into()));
    assert!(matches!(table.cell(1, 1), CellContent::Bar { .. }));
    assert_eq!(table.cell(0, 2), CellContent::Check(true));
    assert_eq!(table.cell(1, 2), CellContent::Check(false));
    assert_eq!(table.cell(0, 3), CellContent::Text("2026-01-01".into()));
    assert_eq!(table.cell(0, 4), CellContent::Chip("p0".into()));
    assert_eq!(table.cell(0, 5), CellContent::Text("t0".into()));
    assert_eq!(table.cell(3, 1), CellContent::Empty, "b000003 has no pages");
}

#[test]
fn a_viewer_the_table_cannot_draw_is_refused_with_the_field_named() {
    let mut odd = draft();
    odd.fields[0].viewer = Some("sparkline".into());
    let compiler = || {
        ProjectionCompiler::new(ItemSizes {
            card: Size2::new(120.0, 24.0),
        })
        .with_viewers(scenograph::ViewerCatalog::built_in().with("sparkline"))
    };
    let Err(issues) = ProjectionTable::open(odd, dataset(3), compiler(), TableViewers::built_in())
    else {
        panic!("an undrawable viewer opened");
    };
    assert!(
        issues
            .iter()
            .any(|i| i.field == "fields[0].viewer" && i.message.contains("\"Title\"")),
        "{issues:?}"
    );
    let mut ok = draft();
    ok.fields[0].viewer = Some("sparkline".into());
    let viewers = TableViewers::built_in().with("sparkline", |_, _| CellContent::Text("~".into()));
    let table = ProjectionTable::open(ok, dataset(3), compiler(), viewers).unwrap();
    assert_eq!(table.cell(0, 0), CellContent::Text("~".into()));
}

#[test]
fn the_frozen_form_is_a_captioned_table_with_one_header_per_field() {
    let mut table = open(12);
    table.click_header(1, 0);
    table.click_header(1, 0);
    let frozen = table.frozen();
    assert_eq!(frozen.check(), Ok(()));
    let html = frozen.to_html("shelf");
    assert!(html.contains("<caption id=\"shelf-caption\">Reading list</caption>"));
    assert_eq!(html.matches("scope=\"col\"").count(), table.columns().len());
    assert!(html.contains("aria-sort=\"descending\">Pages</th>"));
    assert_eq!(frozen.rows.len(), 12);
    // Same order as the grid: descending pages, missing last.
    assert_eq!(frozen.rows[0].source.id, table.row_id(0).unwrap());
    assert_eq!(frozen.rows[11].cells[1], "no value");
}

#[test]
fn a_resort_places_rows_exactly_as_a_fresh_compile_does() {
    for keys in [
        vec![SortKey::ascending("pages")],
        vec![SortKey::descending("added"), SortKey::ascending("title")],
        vec![SortKey::descending("tags")],
        vec![SortKey::ascending("author")],
        Vec::new(),
    ] {
        let mut resorted = open(300);
        resorted.apply_sort(vec![SortKey::descending("read")], 0);
        resorted.apply_sort(keys.clone(), 0);
        let mut fresh_draft = draft();
        fresh_draft.reading.sort = keys.clone();
        let compiler = ProjectionCompiler::new(ItemSizes {
            card: Size2::new(120.0, 24.0),
        });
        let fresh = ProjectionTable::open(
            fresh_draft,
            dataset(300),
            compiler,
            TableViewers::built_in(),
        )
        .unwrap();
        let ids = |table: &ProjectionTable| {
            (0..table.row_count())
                .map(|rank| table.row_id(rank).unwrap().to_owned())
                .collect::<Vec<_>>()
        };
        assert_eq!(ids(&resorted), ids(&fresh), "{keys:?}");
    }
}

#[test]
fn a_several_key_sort_is_one_step_names_every_key_and_drives_the_frozen_form() {
    let keys = vec![SortKey::descending("read"), SortKey::ascending("pages")];
    let (dom, mut runner) = runner(open(40));
    runner.update(|table| table.apply_sort(keys.clone(), 0));
    {
        let d = dom.borrow();
        let mut status = Vec::new();
        find_role(&d, runner.root(), "status", &mut status);
        assert_eq!(
            text(&d, status[0]),
            "40 rows, sorted by Read, descending, then Pages, ascending"
        );
    }
    let table = runner.state();
    assert_eq!(table.sorted(), Some((2, true)), "the lead key");
    let frozen = table.frozen();
    let ids: Vec<_> = (0..table.row_count())
        .map(|rank| table.row_id(rank).unwrap().to_owned())
        .collect();
    let frozen_ids: Vec<_> = frozen
        .rows
        .iter()
        .map(|row| row.source.id.clone())
        .collect();
    assert_eq!(frozen_ids, ids);
    // ARIA puts aria-sort on one header at a time: the lead key's.
    let html = frozen.to_html("shelf");
    assert_eq!(html.matches("aria-sort=").count(), 1);
    assert!(html.contains("aria-sort=\"descending\">Read</th>"));
    runner.update(|table| assert!(table.undo()));
    assert!(runner.state().draft().reading.sort.is_empty());
    assert!(!runner.state().editor().can_undo());
}

#[test]
fn a_refused_sort_is_stepped_back_and_noticed() {
    let mut table = open(10);
    table.apply_sort(vec![SortKey::ascending("colour")], 0);
    assert!(table.draft().reading.sort.is_empty());
    assert!(
        table
            .notice
            .as_deref()
            .is_some_and(|n| n.contains("\"colour\"")),
        "{:?}",
        table.notice
    );
}
