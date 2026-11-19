// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

use serde_json::json;

use super::*;

fn reference(adapter: &str, id: &str) -> ProjectionValue {
    ProjectionValue::Reference(SourceRef::new(adapter, id))
}

fn sorted(mut values: Vec<Option<ProjectionValue>>) -> Vec<Option<ProjectionValue>> {
    values.sort_by(|a, b| compare_values(a.as_ref(), b.as_ref()));
    values
}

#[test]
fn the_first_three_types_keep_their_saved_spelling() {
    // The saved forms from before the type additions, byte for byte.
    let fields = r#"{"a":"text","b":"number","c":"boolean"}"#;
    let read: BTreeMap<String, ProjectionFieldType> = serde_json::from_str(fields).unwrap();
    assert_eq!(serde_json::to_string(&read).unwrap(), fields);
    let values = r#"[{"kind":"text","value":"x"},{"kind":"number","value":1.5},{"kind":"boolean","value":true}]"#;
    let read: Vec<ProjectionValue> = serde_json::from_str(values).unwrap();
    assert_eq!(serde_json::to_string(&read).unwrap(), values);
}

#[test]
fn the_new_types_have_a_disclosed_form() {
    let fields = BTreeMap::from([
        ("due".to_owned(), ProjectionFieldType::Date),
        ("at".to_owned(), ProjectionFieldType::DateTime),
        ("author".to_owned(), ProjectionFieldType::Reference),
        (
            "tags".to_owned(),
            ProjectionFieldType::List(ProjectionItemType::Text),
        ),
    ]);
    assert_eq!(
        serde_json::to_value(&fields).unwrap(),
        json!({"due": "date", "at": "date_time", "author": "reference", "tags": {"list": "text"}})
    );
    let values = vec![
        ProjectionValue::Date("2026-11-19".into()),
        ProjectionValue::DateTime("2026-11-19T12:00:00Z".into()),
        reference("mere.graph", "node-7"),
        ProjectionValue::List(vec![ProjectionValue::Text("a".into())]),
        ProjectionValue::List(vec![]),
    ];
    let written = serde_json::to_value(&values).unwrap();
    assert_eq!(
        written,
        json!([
            {"kind": "date", "value": "2026-11-19"},
            {"kind": "date_time", "value": "2026-11-19T12:00:00Z"},
            {"kind": "reference", "value": {"adapter": "mere.graph", "id": "node-7"}},
            {"kind": "list", "value": [{"kind": "text", "value": "a"}]},
            {"kind": "list", "value": []},
        ])
    );
    let read: Vec<ProjectionValue> = serde_json::from_value(written).unwrap();
    assert_eq!(read, values);
}

#[test]
fn a_list_of_lists_cannot_be_disclosed() {
    assert!(
        serde_json::from_value::<ProjectionFieldType>(json!({"list": {"list": "text"}})).is_err()
    );
    let nested = ProjectionValue::List(vec![ProjectionValue::List(vec![])]);
    let refusal = nested
        .refusal(ProjectionFieldType::List(ProjectionItemType::Text))
        .unwrap();
    assert!(refusal.contains("list of lists"), "{refusal}");
    assert_eq!(nested.field_type(), None);
}

#[test]
fn a_value_is_checked_against_its_disclosed_type() {
    use ProjectionFieldType as T;
    assert_eq!(
        ProjectionValue::Date("2026-11-19".into()).refusal(T::Date),
        None
    );
    assert!(
        ProjectionValue::Date("2026-11-31".into())
            .refusal(T::Date)
            .is_some()
    );
    assert!(
        ProjectionValue::Date("2026-11-19".into())
            .refusal(T::DateTime)
            .is_some()
    );
    assert!(
        ProjectionValue::DateTime("2026-11-19T12:00:00".into())
            .refusal(T::DateTime)
            .is_some()
    );
    assert_eq!(reference("mere.graph", "n1").refusal(T::Reference), None);
    assert!(reference("mere.graph", " ").refusal(T::Reference).is_some());
    // A label is text, not a reference.
    assert_eq!(
        ProjectionValue::Text("Ada".into())
            .refusal(T::Reference)
            .as_deref(),
        Some("value does not match the disclosed field type")
    );
    let dates = T::List(ProjectionItemType::Date);
    assert_eq!(ProjectionValue::List(vec![]).refusal(dates), None);
    let mixed = ProjectionValue::List(vec![
        ProjectionValue::Date("2026-01-01".into()),
        ProjectionValue::Text("soon".into()),
    ]);
    assert!(mixed.refusal(dates).unwrap().starts_with("item 1"));
    assert_eq!(mixed.field_type(), None);
    assert!(
        ProjectionValue::Text("a".into())
            .refusal(T::List(ProjectionItemType::Text))
            .is_some()
    );
}

#[test]
fn a_list_keeps_its_order() {
    let items: Vec<_> = ["c", "a", "b"]
        .map(|t| ProjectionValue::Text(t.into()))
        .into();
    let list = ProjectionValue::List(items.clone());
    let read: ProjectionValue =
        serde_json::from_str(&serde_json::to_string(&list).unwrap()).unwrap();
    assert_eq!(read.list(), Some(items.as_slice()));
    assert_eq!(
        read.field_type(),
        Some(ProjectionFieldType::List(ProjectionItemType::Text))
    );
}

#[test]
fn a_missing_value_sorts_last() {
    for present in [
        ProjectionValue::Text(String::new()),
        ProjectionValue::Number(f64::MAX),
        ProjectionValue::Boolean(true),
        reference("z", "z"),
        ProjectionValue::Date("9999-12-31".into()),
        ProjectionValue::DateTime("9999-12-31T23:59:59Z".into()),
        ProjectionValue::List(vec![]),
    ] {
        assert_eq!(
            sorted(vec![None, Some(present.clone())]),
            vec![Some(present), None]
        );
    }
}

#[test]
fn references_sort_by_source_identity() {
    let order = sorted(vec![
        Some(reference("b", "1")),
        Some(reference("a", "2")),
        Some(reference("a", "10")),
    ]);
    assert_eq!(
        order,
        vec![
            Some(reference("a", "10")),
            Some(reference("a", "2")),
            Some(reference("b", "1")),
        ]
    );
}

#[test]
fn dates_sort_by_day_and_malformed_ones_last() {
    let date = |t: &str| Some(ProjectionValue::Date(t.into()));
    assert_eq!(
        sorted(vec![
            date("not a date"),
            date("2026-11-19"),
            None,
            date("0999-01-01")
        ]),
        vec![
            date("0999-01-01"),
            date("2026-11-19"),
            date("not a date"),
            None
        ]
    );
}

#[test]
fn date_times_sort_as_instants_not_strings() {
    let at = |t: &str| Some(ProjectionValue::DateTime(t.into()));
    // As strings the first sorts first; as instants it is an hour later.
    let later = at("2026-11-19T10:30:00-01:00");
    let earlier = at("2026-11-19T11:00:00Z");
    assert_eq!(
        sorted(vec![later.clone(), earlier.clone()]),
        vec![earlier, later]
    );
    assert_eq!(
        compare_values(
            at("2026-11-19T12:00:00Z").as_ref(),
            at("2026-11-19T13:00:00+01:00").as_ref()
        ),
        Ordering::Equal
    );
}

#[test]
fn lists_sort_item_by_item_and_shorter_first() {
    let list = |items: &[f64]| {
        Some(ProjectionValue::List(
            items.iter().map(|n| ProjectionValue::Number(*n)).collect(),
        ))
    };
    assert_eq!(
        sorted(vec![
            list(&[2.0]),
            list(&[1.0, 5.0]),
            list(&[]),
            list(&[1.0])
        ]),
        vec![list(&[]), list(&[1.0]), list(&[1.0, 5.0]), list(&[2.0])]
    );
}

#[test]
fn the_existing_types_sort_as_before() {
    let text = |t: &str| Some(ProjectionValue::Text(t.into()));
    assert_eq!(
        sorted(vec![text("b"), text("B"), text("a")]),
        vec![text("B"), text("a"), text("b")]
    );
    let number = |n: f64| Some(ProjectionValue::Number(n));
    assert_eq!(
        sorted(vec![number(2.0), number(-1.0)]),
        vec![number(-1.0), number(2.0)]
    );
    let truth = |b: bool| Some(ProjectionValue::Boolean(b));
    assert_eq!(
        sorted(vec![truth(true), truth(false)]),
        vec![truth(false), truth(true)]
    );
}

#[test]
fn every_type_is_listed_once() {
    let all: std::collections::HashSet<_> = ProjectionFieldType::ALL.into_iter().collect();
    assert_eq!(all.len(), ProjectionFieldType::ALL.len());
}
