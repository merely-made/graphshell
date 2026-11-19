// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

use std::collections::BTreeMap;

use super::*;
use crate::catalog::Family;
use crate::projection::{
    ItemSizes, ProjectionCompiler, ProjectionDataset, ProjectionFieldType, ProjectionOccurrence,
    ProjectionValue,
};
use sceno::{FrameKind, Representation, SourceRef};
use scenograph::presentation::{Extent, NodeMark, ROW_KIND};
use scenograph::{
    Appearance, Arrangement, Channel, Encoding, FieldSpec, Interaction, Provenance, Reading,
    RevisionEvidence, SelectionMode, SourceBinding,
};

const ORIGIN: Vec2 = Vec2 { x: 40.0, y: 100.0 };

fn compiler() -> ProjectionCompiler {
    ProjectionCompiler::new(ItemSizes {
        card: Size2::new(120.0, 30.0),
    })
}

/// Three rows; `rank` decides the reading order.
fn dataset(rank: [f64; 3]) -> ProjectionDataset {
    let row = |id: &str, rank: f64, score: f64| {
        ProjectionOccurrence::new(
            id,
            SourceRef::new("fixture", id),
            BTreeMap::from([
                ("id".into(), ProjectionValue::Text(id.into())),
                ("title".into(), ProjectionValue::Text(format!("Title {id}"))),
                ("rank".into(), ProjectionValue::Number(rank)),
                ("score".into(), ProjectionValue::Number(score)),
            ]),
        )
    };
    ProjectionDataset {
        source: SourceBinding {
            authority: "fixture.local".into(),
            domain: "set".into(),
            resource: "set:table".into(),
        },
        revision: "table-r1".into(),
        fields: BTreeMap::from([
            ("id".into(), ProjectionFieldType::Text),
            ("title".into(), ProjectionFieldType::Text),
            ("rank".into(), ProjectionFieldType::Number),
            ("score".into(), ProjectionFieldType::Number),
        ]),
        occurrences: vec![
            row("a", rank[0], 3.0),
            row("b", rank[1], 1.0),
            row("c", rank[2], 2.0),
        ],
    }
}

fn definition(fields: &[(&str, &str)]) -> ProjectionDefinition {
    let data = dataset([0.0, 1.0, 2.0]);
    ProjectionDefinition {
        id: "table".into(),
        label: "Table".into(),
        source: data.source.clone(),
        reading: Reading {
            kind: "nodes".into(),
            key: "id".into(),
            value: None,
        },
        encoding: Encoding {
            x: Channel::Field("rank".into()),
            y: Channel::Field("rank".into()),
            color: None,
            label: Some(Channel::Field("title".into())),
        },
        arrangement: Arrangement {
            kind: "table".into(),
            direction: "coordinates".into(),
            spacing: 16,
            options: BTreeMap::from([
                ("column_width".into(), "100".into()),
                ("row_height".into(), "20".into()),
                ("header".into(), "24".into()),
            ]),
        },
        frame: Some(Frame::Flow {
            origin: ORIGIN,
            width: 500.0,
        }),
        fields: fields
            .iter()
            .map(|(path, name)| FieldSpec::new(*path, *name))
            .collect(),
        interaction: Interaction {
            selection: SelectionMode::Multiple,
            pan: false,
            zoom: false,
        },
        appearance: Appearance {
            realization: "canvas".into(),
            title: "Table".into(),
            theme: "slate".into(),
        },
        provenance: Provenance {
            author: "test".into(),
            source_revision: Some(data.revision.clone()),
            revision_evidence: RevisionEvidence::PublicGeneration,
            note: String::new(),
        },
        ..Default::default()
    }
}

const FIELDS: &[(&str, &str)] = &[("title", "Title"), ("score", "Score"), ("id", "Id")];

fn row_center(compiled: &CompiledProjection, occurrence: &str) -> Vec2 {
    let instance = compiled.instance_by_occurrence[occurrence];
    compiled.scene.items[instance.0 as usize]
        .transform
        .translate
}

#[test]
fn one_row_per_occurrence_in_reading_order_in_a_flow_frame() {
    let compiled = compiler()
        .compile(&definition(FIELDS), &dataset([2.0, 0.0, 1.0]))
        .unwrap();
    // Reading order is b, c, a: rows stack under the 24 px header.
    for (rank, occurrence) in ["b", "c", "a"].into_iter().enumerate() {
        assert_eq!(
            row_center(&compiled, occurrence),
            Vec2::new(
                ORIGIN.x + 150.0,
                ORIGIN.y + 24.0 + 10.0 + rank as f32 * 20.0
            ),
            "{occurrence}"
        );
    }
    let row = sceno::Footprint::Rect {
        size: Size2::new(300.0, 20.0),
    };
    assert!(
        compiled
            .scene
            .items
            .iter()
            .all(|item| item.footprint == row)
    );
    assert!(compiled.scene.items.iter().all(|item| item.representation
        == Representation::Open {
            kind: ROW_KIND.into()
        }));
    let bounds = compiled.scene.bounds;
    assert_eq!(bounds.origin.x, ORIGIN.x);
    assert_eq!(bounds.size.w, 500.0, "the page width bounds the scene");
}

#[test]
fn the_arrangement_declares_what_it_assigns_and_accepts() {
    let capability = Family::Table.capability();
    assert_eq!(Family::Table.extent(), Extent::Assigned);
    assert_eq!(capability.frames, [FrameKind::Flow, FrameKind::Unbounded]);
    assert!(capability.placed_forms.is_empty());
    let keys: Vec<_> = capability.options.iter().map(|o| o.key.as_str()).collect();
    assert_eq!(keys, ["column_width", "row_height", "header"]);

    let mut bounded = definition(FIELDS);
    bounded.frame = Some(Frame::Bounded {
        bounds: Rect::new(Vec2::ZERO, Size2::new(400.0, 300.0)),
    });
    let refused = compiler()
        .compile(&bounded, &dataset([0.0, 1.0, 2.0]))
        .unwrap_err();
    assert!(refused.iter().any(|issue| issue.field == "frame"));

    let mut glyph = definition(FIELDS);
    glyph.marks.node = Some(NodeMark::new(Representation::Glyph, [Extent::Measured]));
    let refused = compiler()
        .compile(&glyph, &dataset([0.0, 1.0, 2.0]))
        .unwrap_err();
    assert!(
        refused
            .iter()
            .any(|issue| issue.message.contains("table imposes the assigned extent")),
        "{refused:?}"
    );
}

#[test]
fn columns_follow_the_fields_list_and_default_to_the_label() {
    let compiled = compiler()
        .compile(&definition(FIELDS), &dataset([0.0, 1.0, 2.0]))
        .unwrap();
    let layout = TableLayout::of(&compiled, &definition(FIELDS)).unwrap();
    let columns: Vec<_> = layout
        .columns
        .iter()
        .map(|c| (c.field.as_str(), c.x, c.width))
        .collect();
    assert_eq!(
        columns,
        [
            ("title", 0.0, 100.0),
            ("score", 100.0, 100.0),
            ("id", 200.0, 100.0)
        ]
    );
    assert_eq!(layout.table.origin, ORIGIN);

    let bare = definition(&[]);
    let compiled = compiler()
        .compile(&bare, &dataset([0.0, 1.0, 2.0]))
        .unwrap();
    let layout = TableLayout::of(&compiled, &bare).unwrap();
    assert_eq!(layout.columns.len(), 1);
    assert_eq!(layout.columns[0].field, "title");
    assert_eq!(
        compiled.scene.items[0].footprint.bounds().unwrap().size.w,
        100.0
    );
}

#[test]
fn picking_a_point_returns_the_row_and_the_field_under_it() {
    let definition = definition(FIELDS);
    let compiled = compiler()
        .compile(&definition, &dataset([2.0, 0.0, 1.0]))
        .unwrap();
    let layout = TableLayout::of(&compiled, &definition).unwrap();
    // Second row (c), second column (score).
    let point = Vec2::new(ORIGIN.x + 150.0, ORIGIN.y + 24.0 + 25.0);
    assert_eq!(
        layout.pick(&compiled, point),
        Some(TableHit::Cell(TableTarget::cell("c", "score")))
    );
    assert_eq!(
        layout.pick(&compiled, Vec2::new(ORIGIN.x + 250.0, ORIGIN.y + 5.0)),
        Some(TableHit::Header { field: "id".into() })
    );
    assert_eq!(
        layout.pick(&compiled, Vec2::new(ORIGIN.x + 301.0, ORIGIN.y + 30.0)),
        None
    );
    assert_eq!(
        layout.pick(&compiled, Vec2::new(ORIGIN.x + 10.0, ORIGIN.y + 200.0)),
        None
    );
    assert_eq!(
        layout.target_rect(&compiled, &TableTarget::cell("c", "score")),
        Some(Rect::new(
            Vec2::new(ORIGIN.x + 100.0, ORIGIN.y + 44.0),
            Size2::new(100.0, 20.0)
        ))
    );
}

#[test]
fn a_saved_selection_of_rows_and_cells_survives_a_sort() {
    let definition = definition(FIELDS);
    let before = compiler()
        .compile(&definition, &dataset([0.0, 1.0, 2.0]))
        .unwrap();
    let layout = TableLayout::of(&before, &definition).unwrap();
    let Some(TableHit::Cell(cell)) =
        layout.pick(&before, Vec2::new(ORIGIN.x + 150.0, ORIGIN.y + 24.0 + 5.0))
    else {
        panic!("a cell")
    };
    assert_eq!(cell, TableTarget::cell("a", "score"));
    let mut selection = TableSelection::one(cell.clone());
    selection.toggle(TableTarget::row("c"));
    let saved = serde_json::to_string(&selection).unwrap();

    // The order reverses: a is now last and c first.
    let after = compiler()
        .compile(&definition, &dataset([2.0, 1.0, 0.0]))
        .unwrap();
    let mut reopened: TableSelection = serde_json::from_str(&saved).unwrap();
    assert!(
        TableLayout::of(&after, &definition)
            .unwrap()
            .restore(&after, &mut reopened)
            .is_empty()
    );
    assert_eq!(reopened, selection);
    let layout = TableLayout::of(&after, &definition).unwrap();
    assert_eq!(
        layout.target_rect(&after, &cell).unwrap().origin,
        Vec2::new(ORIGIN.x + 100.0, ORIGIN.y + 24.0 + 40.0),
        "the cell follows its row to the bottom"
    );
    assert_eq!(
        layout
            .target_rect(&after, &TableTarget::row("c"))
            .unwrap()
            .origin,
        Vec2::new(ORIGIN.x, ORIGIN.y + 24.0)
    );

    // Reordering the fields moves the column, not the selection.
    let reordered = super::tests::definition(&[("score", "Score"), ("title", "Title")]);
    let moved = compiler()
        .compile(&reordered, &dataset([2.0, 1.0, 0.0]))
        .unwrap();
    let layout = TableLayout::of(&moved, &reordered).unwrap();
    assert_eq!(
        layout.target_rect(&moved, &cell).unwrap().origin.x,
        ORIGIN.x
    );
    let mut narrowed = reopened.clone();
    let dropped = TableLayout::of(&moved, &definition_without_score())
        .unwrap()
        .restore(&moved, &mut narrowed);
    assert_eq!(dropped, [cell]);
    assert_eq!(narrowed.targets, [TableTarget::row("c")]);
}

fn definition_without_score() -> ProjectionDefinition {
    definition(&[("title", "Title")])
}

#[test]
fn a_table_takes_multiple_selection_and_other_families_still_refuse_it() {
    let compiler = compiler();
    assert!(
        compiler
            .compile(&definition(FIELDS), &dataset([0.0, 1.0, 2.0]))
            .is_ok()
    );
    let mut grid = definition(FIELDS);
    grid.arrangement.kind = "grid".into();
    grid.arrangement.options.clear();
    grid.frame = None;
    let refused = compiler
        .compile(&grid, &dataset([0.0, 1.0, 2.0]))
        .unwrap_err();
    assert!(refused.iter().any(|issue| issue.field == "interaction"));
}

#[test]
fn an_unbounded_frame_or_none_places_the_header_at_its_origin() {
    let mut unframed = definition(FIELDS);
    unframed.frame = None;
    let compiled = compiler()
        .compile(&unframed, &dataset([0.0, 1.0, 2.0]))
        .unwrap();
    assert_eq!(row_center(&compiled, "a"), Vec2::new(150.0, 34.0));
    let mut unbounded = unframed.clone();
    unbounded.frame = Some(Frame::Unbounded { origin: ORIGIN });
    let compiled = compiler()
        .compile(&unbounded, &dataset([0.0, 1.0, 2.0]))
        .unwrap();
    assert_eq!(
        row_center(&compiled, "a"),
        Vec2::new(ORIGIN.x + 150.0, ORIGIN.y + 34.0)
    );
}
