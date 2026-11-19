// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The reading's filter and grouping and the table's heading bands (table
//! projection plan, T5).

use std::collections::BTreeMap;

use super::tests::{compiler, definition};
use super::{TableHit, TableLayout};
#[cfg(feature = "authoring")]
use crate::authoring::{EditorAction, ProjectionEditor};
use crate::projection::{
    CompiledProjection, ProjectionDataset, ProjectionFieldType, ProjectionOccurrence,
    ProjectionValue,
};
use sceno::{InstanceId, SourceRef, TableHeading, Vec2};
use scenograph::SortKey;
use scenograph::filter::{Condition, FilterOp, FilterValue};

const FIELDS: &[(&str, &str)] = &[("title", "Title"), ("shelf", "Shelf"), ("score", "Score")];

/// Six books: shelf is art, sea or missing; score orders them.
fn dataset() -> ProjectionDataset {
    let row = |id: &str, shelf: Option<&str>, score: f64| {
        let mut values = BTreeMap::from([
            ("id".into(), ProjectionValue::Text(id.into())),
            ("title".into(), ProjectionValue::Text(format!("Title {id}"))),
            ("rank".into(), ProjectionValue::Number(score)),
            ("score".into(), ProjectionValue::Number(score)),
        ]);
        if let Some(shelf) = shelf {
            values.insert("shelf".into(), ProjectionValue::Text(shelf.into()));
        }
        ProjectionOccurrence::new(id, SourceRef::new("fixture", id), values)
    };
    let mut data = super::tests::dataset([0.0, 1.0, 2.0]);
    data.fields
        .insert("shelf".into(), ProjectionFieldType::Text);
    data.occurrences = vec![
        row("a", Some("sea"), 5.0),
        row("b", Some("art"), 1.0),
        row("c", None, 2.0),
        row("d", Some("art"), 4.0),
        row("e", Some("sea"), 3.0),
        row("f", Some("art"), 0.5),
    ];
    data
}

fn order(compiled: &CompiledProjection) -> Vec<String> {
    (0..compiled.occurrence_by_instance.len())
        .map(|index| compiled.occurrence_by_instance[&InstanceId(index as u32)].clone())
        .collect()
}

fn headings(compiled: &CompiledProjection) -> Vec<TableHeading> {
    match &compiled.score.arrangement {
        sceno::Arrangement::Table(table) => table.headings.clone(),
        other => panic!("not a table: {other:?}"),
    }
}

fn grouped() -> scenograph::ProjectionDefinition {
    let mut grouped = definition(FIELDS);
    grouped.reading.group = vec![SortKey::ascending("shelf")];
    grouped.reading.sort = vec![SortKey::descending("score")];
    grouped
}

#[test]
fn a_filter_keeps_the_rows_that_meet_every_condition() {
    let mut filtered = definition(FIELDS);
    filtered.reading.filter = vec![
        Condition::new("score", FilterOp::Ge, Some(FilterValue::Number(1.0))),
        Condition::new(
            "shelf",
            FilterOp::IsNot,
            Some(FilterValue::Text("sea".into())),
        ),
    ];
    let compiled = compiler().compile(&filtered, &dataset()).unwrap();
    // By rank (= score) ascending; c has no shelf, so it is not sea.
    assert_eq!(order(&compiled), ["b", "c", "d"]);
    assert!(!compiled.instance_by_occurrence.contains_key("f"));
}

#[test]
fn groups_follow_the_field_with_no_value_last_and_rows_keep_the_sort() {
    let compiled = compiler().compile(&grouped(), &dataset()).unwrap();
    assert_eq!(order(&compiled), ["d", "b", "f", "a", "e", "c"]);
    let heading = |position, label: &str, count| TableHeading {
        position,
        level: 0,
        label: label.into(),
        count,
    };
    assert_eq!(
        headings(&compiled),
        [
            heading(0, "Shelf: art", 3),
            heading(3, "Shelf: sea", 2),
            heading(5, "Shelf: No value", 1),
        ]
    );
    // Rows move down past the headings: d below one, a below two.
    let y = |id: &str| {
        compiled.scene.items[compiled.instance_by_occurrence[id].0 as usize]
            .transform
            .translate
            .y
    };
    assert_eq!(y("b") - y("d"), 20.0);
    assert_eq!(y("a") - y("f"), 40.0);
    // Descending groups reverse the field's order; no value stays last.
    let mut down = grouped();
    down.reading.group = vec![SortKey::descending("shelf")];
    let compiled = compiler().compile(&down, &dataset()).unwrap();
    assert_eq!(order(&compiled), ["a", "e", "d", "b", "f", "c"]);
}

#[test]
fn two_levels_head_each_inner_group_inside_its_outer_one() {
    let mut two = grouped();
    two.reading.group = vec![SortKey::ascending("shelf"), SortKey::ascending("score")];
    two.reading.filter = vec![Condition::new(
        "shelf",
        FilterOp::Is,
        Some(FilterValue::Text("art".into())),
    )];
    let compiled = compiler().compile(&two, &dataset()).unwrap();
    let found: Vec<(usize, usize, String, usize)> = headings(&compiled)
        .into_iter()
        .map(|h| (h.position, h.level, h.label, h.count))
        .collect();
    assert_eq!(
        found,
        [
            (0, 0, "Shelf: art".into(), 3),
            (0, 1, "Score: 0.5".into(), 1),
            (1, 1, "Score: 1".into(), 1),
            (2, 1, "Score: 4".into(), 1),
        ]
    );
}

#[test]
fn the_view_s_reading_agrees_with_a_fresh_compile() {
    let mut two = grouped();
    two.reading.group.push(SortKey::ascending("title"));
    two.reading.filter = vec![Condition::new(
        "title",
        FilterOp::Contains,
        Some(FilterValue::Text("TITLE".into())),
    )];
    for definition in [grouped(), two] {
        let compiled = compiler().compile(&definition, &dataset()).unwrap();
        let reading = compiler().table_reading(&definition, &dataset()).unwrap();
        let data = dataset();
        let ids: Vec<String> = reading
            .rows
            .iter()
            .map(|&index| data.occurrences[index].occurrence_id.clone())
            .collect();
        assert_eq!(ids, order(&compiled));
        let starts: Vec<(usize, usize, usize)> = reading
            .groups
            .iter()
            .map(|band| (band.start, band.level, band.count))
            .collect();
        let placed: Vec<(usize, usize, usize)> = headings(&compiled)
            .iter()
            .map(|h| (h.position, h.level, h.count))
            .collect();
        assert_eq!(starts, placed);
    }
}

#[test]
fn a_bad_filter_or_grouping_is_refused_naming_the_field() {
    let data = dataset();
    let refused = |edit: &dyn Fn(&mut scenograph::ProjectionDefinition)| {
        let mut bad = definition(FIELDS);
        edit(&mut bad);
        compiler().compile(&bad, &data).unwrap_err()
    };
    let has = |issues: &[crate::projection::CompileIssue], slot: &str, word: &str| {
        assert!(
            issues
                .iter()
                .any(|i| i.field == slot && i.message.contains(word)),
            "{slot} {word}: {issues:?}"
        );
    };
    let issues = refused(&|d| {
        d.reading.filter = vec![
            Condition::new("colour", FilterOp::IsEmpty, None),
            Condition::new(
                "score",
                FilterOp::Contains,
                Some(FilterValue::Text("1".into())),
            ),
            Condition::new(
                "score",
                FilterOp::Is,
                Some(FilterValue::Text("high".into())),
            ),
        ];
    });
    has(&issues, "reading.filter[0]", "\"colour\"");
    has(&issues, "reading.filter[1]", "\"score\"");
    has(&issues, "reading.filter[2]", "\"score\"");
    let issues = refused(&|d| {
        d.reading.group = vec![
            SortKey::ascending("colour"),
            SortKey::ascending("shelf"),
            SortKey::descending("shelf"),
        ];
    });
    has(&issues, "reading.group[0].field", "\"colour\"");
    has(&issues, "reading.group[2].field", "twice");
    let issues = refused(&|d| {
        d.arrangement.kind = "geographic".into();
        d.arrangement.options.clear();
        d.frame = None;
        d.reading.filter = vec![Condition::new("shelf", FilterOp::IsEmpty, None)];
        d.reading.group = vec![SortKey::ascending("shelf")];
    });
    has(&issues, "reading.filter", "place by order");
    has(&issues, "reading.group", "place by order");
}

#[test]
fn a_heading_is_picked_as_a_heading_and_is_no_row() {
    let compiled = compiler().compile(&grouped(), &dataset()).unwrap();
    let definition = grouped();
    let layout = TableLayout::of(&compiled, &definition).unwrap();
    let rect = layout.heading_rect(1).unwrap();
    let inside = Vec2::new(rect.origin.x + 5.0, rect.origin.y + rect.size.h / 2.0);
    assert_eq!(layout.pick(&compiled, inside), Some(TableHit::Heading(1)));
    // The row just below it is the group's first row.
    let below = Vec2::new(inside.x, inside.y + rect.size.h);
    assert!(matches!(
        layout.pick(&compiled, below),
        Some(TableHit::Cell(target)) if target.row == "a"
    ));
}

#[cfg(feature = "authoring")]
#[test]
fn a_filter_and_a_grouping_are_one_undo_step_each_and_reopen_from_the_save() {
    let definition = definition(FIELDS);
    let draft: scenograph::ProjectionDraft =
        serde_json::from_slice(&definition.to_json_bytes().unwrap()).unwrap();
    let mut editor = ProjectionEditor::new(draft);
    let keep = vec![Condition::new("shelf", FilterOp::IsNotEmpty, None)];
    editor.reduce(EditorAction::SetFilter(keep.clone()), 0);
    editor.reduce(EditorAction::SetGroup(vec![SortKey::ascending("shelf")]), 5);
    editor.reduce(
        EditorAction::SetGroup(vec![SortKey::descending("shelf")]),
        10,
    );
    let saved = editor
        .draft()
        .to_definition()
        .unwrap()
        .to_json_bytes()
        .unwrap();
    let reopened: scenograph::ProjectionDefinition = serde_json::from_slice(&saved).unwrap();
    assert_eq!(reopened.reading.filter, keep);
    assert_eq!(reopened.reading.group, [SortKey::descending("shelf")]);
    assert!(editor.undo());
    assert_eq!(editor.draft().reading.group, [SortKey::ascending("shelf")]);
    assert!(editor.undo());
    assert!(editor.draft().reading.group.is_empty());
    assert!(editor.undo());
    assert!(editor.draft().reading.filter.is_empty());
    assert!(!editor.can_undo());
}
