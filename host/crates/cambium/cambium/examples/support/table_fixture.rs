// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The table fixture both table hosts open (table projection plan, T4): a
//! reading list of `rows` books with one field of each value type, shown
//! plainly and through the chip, bar and check viewers.
//!
//! Included by `#[path]` from the desktop and browser table examples, so
//! the two draw the same recipe over the same data.

use std::collections::BTreeMap;

use cambium::projection_table::{PROJECTION_TABLE_CSS, ProjectionTable, TableViewers};
use sceno::{Frame, Size2, SourceRef, Vec2};
use scenograph::dataset::{
    ProjectionDataset, ProjectionFieldType, ProjectionItemType, ProjectionOccurrence,
    ProjectionValue,
};
use scenograph::{
    Appearance, Arrangement, Channel, Encoding, FieldSpec, Interaction, ProjectionDraft,
    Provenance, SelectionMode, SourceBinding,
};
use scenomise::projection::{ItemSizes, ProjectionCompiler, TABLE_REALIZATION_ID};

const COLUMN_WIDTH: f32 = 150.0;
const ROW_HEIGHT: f32 = 26.0;
const HEADER_HEIGHT: f32 = 30.0;

const WORDS: [&str; 12] = [
    "Harbour",
    "Lantern",
    "Orchard",
    "Meridian",
    "Quarry",
    "Tidewater",
    "Ember",
    "Juniper",
    "Basalt",
    "Kestrel",
    "Saltmarsh",
    "Vellum",
];
const TAGS: [&str; 5] = ["essay", "novel", "atlas", "letters", "poems"];

fn source() -> SourceBinding {
    SourceBinding {
        authority: "fixture.local".into(),
        domain: "books".into(),
        resource: "books:reading-list".into(),
    }
}

/// `rows` books. Every eleventh has no page count, so a missing value shows.
pub fn dataset(rows: usize) -> ProjectionDataset {
    let occurrences = (0..rows)
        .map(|i| {
            let id = format!("book-{i:06}");
            // A fixed scramble, so ids and titles do not sort alike.
            let k = (i.wrapping_mul(2_654_435_761)) % 1_000_003;
            let title = format!(
                "{} {} {}",
                WORDS[k % 12],
                WORDS[(k / 12) % 12].to_lowercase(),
                k % 997
            );
            let mut values = BTreeMap::from([
                ("id".into(), ProjectionValue::Text(id.clone())),
                ("title".into(), ProjectionValue::Text(title)),
                ("read".into(), ProjectionValue::Boolean(k % 3 == 0)),
                (
                    "added".into(),
                    ProjectionValue::Date(format!(
                        "20{:02}-{:02}-{:02}",
                        10 + k % 16,
                        1 + k % 12,
                        1 + k % 28
                    )),
                ),
                (
                    "author".into(),
                    ProjectionValue::Reference(SourceRef::new(
                        "people",
                        format!("author-{}", k % 40),
                    )),
                ),
                (
                    "tags".into(),
                    ProjectionValue::List(
                        (0..1 + k % 2)
                            .map(|t| ProjectionValue::Text(TAGS[(k + t) % 5].into()))
                            .collect(),
                    ),
                ),
            ]);
            if i % 11 != 5 {
                values.insert(
                    "pages".into(),
                    ProjectionValue::Number((40 + k % 960) as f64),
                );
            }
            ProjectionOccurrence::new(id.clone(), SourceRef::new("books", id), values)
        })
        .collect();
    ProjectionDataset {
        source: source(),
        revision: format!("reading-list-{rows}").into(),
        fields: BTreeMap::from([
            ("id".into(), ProjectionFieldType::Text),
            ("title".into(), ProjectionFieldType::Text),
            ("pages".into(), ProjectionFieldType::Number),
            ("read".into(), ProjectionFieldType::Boolean),
            ("added".into(), ProjectionFieldType::Date),
            ("author".into(), ProjectionFieldType::Reference),
            (
                "tags".into(),
                ProjectionFieldType::List(ProjectionItemType::Text),
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

/// The table recipe: six fields, realized on the data grid.
pub fn draft(rows: usize) -> ProjectionDraft {
    let mut draft = ProjectionDraft::new();
    draft.id = "reading-list".into();
    draft.label = "Reading list".into();
    draft.source = source();
    draft.reading.key = "id".into();
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
            ("column_width".into(), COLUMN_WIDTH.to_string()),
            ("row_height".into(), ROW_HEIGHT.to_string()),
            ("header".into(), HEADER_HEIGHT.to_string()),
        ]),
    };
    draft.frame = Some(Frame::Flow {
        origin: Vec2::ZERO,
        width: COLUMN_WIDTH * 6.0,
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
        title: format!("Reading list, {rows} books"),
        theme: "slate".into(),
    };
    draft.provenance = Provenance {
        author: "table fixture".into(),
        source_revision: Some(format!("reading-list-{rows}").into()),
        ..Default::default()
    };
    draft
}

/// Open the fixture with a body `viewport_height` px tall.
pub fn open(rows: usize, viewport_height: f32) -> ProjectionTable {
    let compiler = ProjectionCompiler::new(ItemSizes {
        card: Size2::new(COLUMN_WIDTH, ROW_HEIGHT),
    });
    let mut table = ProjectionTable::open(
        draft(rows),
        dataset(rows),
        compiler,
        TableViewers::built_in(),
    )
    .unwrap_or_else(|issues| panic!("the table fixture did not open: {issues:?}"));
    table.viewport_height = viewport_height;
    table
}

/// The host's colours over the table's structural sheet.
pub fn sheet() -> String {
    format!(
        "{PROJECTION_TABLE_CSS} \
:root {{ font-family: Roboto, 'Segoe UI', sans-serif; font-size: 14px; color: #1f2329; background: #f6f5f1; }} \
.app {{ padding: 16px; }} \
.table-caption {{ font-size: 17px; }} \
.grid {{ background: #ffffff; border: 1px solid #c9c6bd; }} \
.grid-header {{ background: #e7e4dc; }} \
.grid-header-cell {{ font-weight: bold; }} \
.grid-header-cell:focus {{ background: #c6d8ee; }} \
.grid-row-odd {{ background: #f3f2ee; }} \
.grid-row-focus {{ background: #dde8f5; }} \
.grid-cell:focus {{ background: #c6d8ee; }} \
.cell-chip {{ background: #d9e6d2; color: #22401a; }} \
.cell-bar-track {{ background: #e2e0da; }} \
.cell-bar-fill {{ background: #4f7cac; }} \
.cell-check {{ border: 2px solid #6b6f76; }} \
.cell-check-on {{ background: #3d7a46; border-color: #3d7a46; }} \
.table-status {{ color: #4c5058; }}"
    )
}
