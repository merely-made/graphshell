// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The dataset's added field types at compile (table projection plan, T2).

use super::catalog_tests::{dataset, definition, fields, sizes};
use super::*;
use scenograph::FieldSpec;
use scenograph::dataset::ProjectionItemType;

/// The catalog fixture with a date, a date-time, a reference and a list
/// disclosed; `b` leaves them all out and `a` has an empty list.
fn typed() -> ProjectionDataset {
    let mut data = dataset();
    data.fields.extend([
        ("due".into(), ProjectionFieldType::Date),
        ("at".into(), ProjectionFieldType::DateTime),
        ("author".into(), ProjectionFieldType::Reference),
        (
            "tags".into(),
            ProjectionFieldType::List(ProjectionItemType::Text),
        ),
    ]);
    for occurrence in &mut data.occurrences {
        let values = &mut occurrence.values;
        match occurrence.occurrence_id.as_str() {
            "c" => {
                values.insert("due".into(), ProjectionValue::Date("2026-11-19".into()));
                values.insert(
                    "at".into(),
                    ProjectionValue::DateTime("2026-11-19T10:30:00-01:00".into()),
                );
                values.insert(
                    "author".into(),
                    ProjectionValue::Reference(SourceRef::new("fixture", "person-1")),
                );
                values.insert(
                    "tags".into(),
                    ProjectionValue::List(vec![
                        ProjectionValue::Text("z".into()),
                        ProjectionValue::Text("a".into()),
                    ]),
                );
            },
            "a" => {
                values.insert("tags".into(), ProjectionValue::List(Vec::new()));
            },
            _ => {},
        }
    }
    data
}

fn typed_definition() -> ProjectionDefinition {
    let mut definition = definition("grid", "x");
    definition.fields = ["due", "at", "author", "tags"]
        .map(|path| FieldSpec::new(path, path))
        .into();
    definition
}

#[test]
fn a_dataset_with_the_added_types_compiles() {
    let compiler = ProjectionCompiler::new(sizes(40.0, 20.0));
    let data = typed();
    // A saved dataset reads back to the same values, list order kept.
    let read: ProjectionDataset =
        serde_json::from_slice(&serde_json::to_vec(&data).unwrap()).unwrap();
    assert_eq!(read, data);
    compiler.compile(&typed_definition(), &read).unwrap();
}

#[test]
fn a_malformed_value_is_refused_with_its_occurrence_and_field() {
    let compiler = ProjectionCompiler::new(sizes(40.0, 20.0));
    let cases = [
        ("due", ProjectionValue::Date("2026-02-30".into())),
        (
            "at",
            ProjectionValue::DateTime("2026-11-19T10:30:00".into()),
        ),
        ("author", ProjectionValue::Text("Ada".into())),
        (
            "author",
            ProjectionValue::Reference(SourceRef::new("fixture", "")),
        ),
        (
            "tags",
            ProjectionValue::List(vec![ProjectionValue::List(Vec::new())]),
        ),
        (
            "tags",
            ProjectionValue::List(vec![ProjectionValue::Number(1.0)]),
        ),
    ];
    for (field, value) in cases {
        let mut data = typed();
        data.occurrences[1]
            .values
            .insert(field.into(), value.clone());
        let issues = compiler.compile(&typed_definition(), &data).unwrap_err();
        let expected = format!("occurrences.a.values.{field}");
        assert!(
            fields(&issues).contains(&expected.as_str()),
            "{value:?} in {field}: {issues:?}"
        );
    }
}
