// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The reading's sort and the table realization (table projection plan, T4).

use super::tests::{FIELDS, compiler, dataset, definition};
#[cfg(feature = "authoring")]
use crate::authoring::{EditorAction, ProjectionEditor};
use crate::projection::{CompiledProjection, TABLE_REALIZATION_ID};
use sceno::InstanceId;
use scenograph::SortKey;

fn order(compiled: &CompiledProjection) -> Vec<String> {
    (0..compiled.occurrence_by_instance.len())
        .map(|index| compiled.occurrence_by_instance[&InstanceId(index as u32)].clone())
        .collect()
}

#[test]
fn a_sort_orders_rows_over_x_and_an_empty_sort_keeps_x() {
    // x (rank) reads a, b, c; score reads b (1), c (2), a (3).
    let data = dataset([0.0, 1.0, 2.0]);
    let plain = compiler().compile(&definition(FIELDS), &data).unwrap();
    assert_eq!(order(&plain), ["a", "b", "c"]);

    let mut sorted = definition(FIELDS);
    sorted.reading.sort = vec![SortKey::ascending("score")];
    assert_eq!(
        order(&compiler().compile(&sorted, &data).unwrap()),
        ["b", "c", "a"]
    );
    sorted.reading.sort = vec![SortKey::descending("score")];
    assert_eq!(
        order(&compiler().compile(&sorted, &data).unwrap()),
        ["a", "c", "b"]
    );
}

#[test]
fn a_sort_on_an_undisclosed_or_repeated_field_is_refused_by_name() {
    let data = dataset([0.0, 1.0, 2.0]);
    let mut bad = definition(FIELDS);
    bad.reading.sort = vec![
        SortKey::ascending("colour"),
        SortKey::ascending("score"),
        SortKey::descending("score"),
    ];
    let issues = compiler().compile(&bad, &data).unwrap_err();
    assert!(
        issues
            .iter()
            .any(|i| i.field == "reading.sort[0].field" && i.message.contains("\"colour\"")),
        "{issues:?}"
    );
    assert!(
        issues
            .iter()
            .any(|i| i.field == "reading.sort[2].field" && i.message.contains("twice")),
        "{issues:?}"
    );
}

#[test]
fn a_sort_is_refused_where_the_arrangement_does_not_place_by_order() {
    let data = dataset([0.0, 1.0, 2.0]);
    let mut scatter = definition(FIELDS);
    scatter.arrangement.kind = "geographic".into();
    scatter.arrangement.options.clear();
    scatter.frame = None;
    scatter.reading.sort = vec![SortKey::ascending("score")];
    let issues = compiler().compile(&scatter, &data).unwrap_err();
    assert!(
        issues.iter().any(|i| i.field == "reading.sort"),
        "{issues:?}"
    );
}

#[test]
fn the_table_realization_is_accepted_for_a_table_only() {
    let data = dataset([0.0, 1.0, 2.0]);
    let mut grid = definition(FIELDS);
    grid.appearance.realization = TABLE_REALIZATION_ID.into();
    let compiled = compiler().compile(&grid, &data).unwrap();
    assert!(super::TableLayout::of(&compiled, &grid).is_some());

    let mut stack = grid.clone();
    stack.arrangement.kind = "stack".into();
    stack.arrangement.options.clear();
    let issues = compiler().compile(&stack, &data).unwrap_err();
    assert!(
        issues
            .iter()
            .any(|i| i.field == "appearance.realization" && i.message.contains("only the table")),
        "{issues:?}"
    );

    let mut unknown = definition(FIELDS);
    unknown.appearance.realization = "spreadsheet".into();
    let issues = compiler().compile(&unknown, &data).unwrap_err();
    assert!(issues.iter().any(|i| i.field == "appearance.realization"));
}

#[cfg(feature = "authoring")]
fn draft() -> scenograph::ProjectionDraft {
    let definition = definition(FIELDS);
    scenograph::ProjectionDraft {
        version: definition.version,
        id: definition.id,
        label: definition.label,
        source: definition.source,
        reading: definition.reading,
        encoding: definition.encoding,
        arrangement: definition.arrangement,
        frame: definition.frame,
        marks: definition.marks,
        fields: definition.fields,
        dynamics: definition.dynamics,
        interaction: definition.interaction,
        appearance: definition.appearance,
        provenance: definition.provenance,
    }
}

#[cfg(feature = "authoring")]
#[test]
fn each_sort_is_one_undo_step_even_inside_the_coalescing_window() {
    let mut editor = ProjectionEditor::new(draft());
    editor.reduce(EditorAction::SetSort(vec![SortKey::ascending("score")]), 0);
    editor.reduce(
        EditorAction::SetSort(vec![SortKey::descending("score")]),
        10,
    );
    assert_eq!(editor.draft().reading.sort, [SortKey::descending("score")]);
    assert!(editor.undo());
    assert_eq!(editor.draft().reading.sort, [SortKey::ascending("score")]);
    assert!(editor.undo());
    assert!(editor.draft().reading.sort.is_empty());
    assert!(!editor.can_undo());
    // The saved definition carries the sort.
    assert!(editor.redo());
    let saved = editor.draft().to_definition().unwrap();
    let json = String::from_utf8(saved.to_json_bytes().unwrap()).unwrap();
    assert!(json.contains(r#""sort":[{"field":"score"}]"#), "{json}");
}
