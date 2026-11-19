// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

use super::*;
use crate::dataset::ProjectionItemType;

fn fields() -> BTreeMap<String, ProjectionFieldType> {
    BTreeMap::from([
        ("title".into(), ProjectionFieldType::Text),
        ("pages".into(), ProjectionFieldType::Number),
        ("read".into(), ProjectionFieldType::Boolean),
        ("added".into(), ProjectionFieldType::Date),
        ("author".into(), ProjectionFieldType::Reference),
        (
            "tags".into(),
            ProjectionFieldType::List(ProjectionItemType::Text),
        ),
    ])
}

fn book(
    title: &str,
    pages: Option<f64>,
    added: &str,
    tags: &[&str],
) -> BTreeMap<String, ProjectionValue> {
    let mut values = BTreeMap::from([
        ("title".into(), ProjectionValue::Text(title.into())),
        ("read".into(), ProjectionValue::Boolean(pages.is_some())),
        ("added".into(), ProjectionValue::Date(added.into())),
        (
            "author".into(),
            ProjectionValue::Reference(SourceRef::new("people", title)),
        ),
        (
            "tags".into(),
            ProjectionValue::List(
                tags.iter()
                    .map(|t| ProjectionValue::Text((*t).into()))
                    .collect(),
            ),
        ),
    ]);
    if let Some(pages) = pages {
        values.insert("pages".into(), ProjectionValue::Number(pages));
    }
    values
}

fn kept(conditions: &[Condition]) -> Vec<String> {
    let rows = [
        book("Harbour Lights", Some(320.0), "2021-03-04", &["novel"]),
        book("harbour", None, "2019-12-31", &[]),
        book("Ember", Some(80.0), "2024-01-01", &["poems", "essay"]),
    ];
    let matcher = Matcher::new(conditions, &fields()).expect("valid filter");
    rows.iter()
        .filter(|values| matcher.matches(values))
        .map(|values| values["title"].text().unwrap().to_owned())
        .collect()
}

fn text(t: &str) -> Option<FilterValue> {
    Some(FilterValue::Text(t.into()))
}

#[test]
fn contains_ignores_case_and_is_compares_exact_bytes() {
    let c = |op, value| Condition::new("title", op, value);
    assert_eq!(
        kept(&[c(FilterOp::Contains, text("HARBOUR"))]),
        ["Harbour Lights", "harbour"]
    );
    assert_eq!(kept(&[c(FilterOp::StartsWith, text("emb"))]), ["Ember"]);
    assert_eq!(kept(&[c(FilterOp::Is, text("harbour"))]), ["harbour"]);
    assert_eq!(
        kept(&[c(FilterOp::Is, text("Harbour"))]),
        Vec::<String>::new()
    );
}

#[test]
fn every_condition_holds_and_a_missing_value_passes_only_is_not_and_is_empty() {
    let pages = |op, value: f64| Condition::new("pages", op, Some(FilterValue::Number(value)));
    assert_eq!(
        kept(&[pages(FilterOp::Ge, 80.0)]),
        ["Harbour Lights", "Ember"]
    );
    assert_eq!(kept(&[pages(FilterOp::Gt, 80.0)]), ["Harbour Lights"]);
    assert_eq!(
        kept(&[pages(FilterOp::IsNot, 80.0)]),
        ["Harbour Lights", "harbour"]
    );
    assert_eq!(
        kept(&[Condition::new("pages", FilterOp::IsEmpty, None)]),
        ["harbour"]
    );
    assert_eq!(
        kept(&[
            pages(FilterOp::Lt, 400.0),
            Condition::new("title", FilterOp::Contains, text("o")),
        ]),
        ["Harbour Lights"]
    );
}

#[test]
fn dates_order_by_the_calendar_and_lists_test_their_items() {
    let between = Condition::new(
        "added",
        FilterOp::Between,
        Some(FilterValue::Many(vec![
            FilterValue::Text("2020-01-01".into()),
            FilterValue::Text("2024-01-01".into()),
        ])),
    );
    assert_eq!(kept(&[between]), ["Harbour Lights", "Ember"]);
    let tagged = Condition::new("tags", FilterOp::ContainsItem, text("essay"));
    assert_eq!(kept(&[tagged]), ["Ember"]);
    // An empty list is empty; a whole list can be asked for exactly.
    assert_eq!(
        kept(&[Condition::new("tags", FilterOp::IsEmpty, None)]),
        ["harbour"]
    );
    let whole = Condition::new(
        "tags",
        FilterOp::Is,
        Some(FilterValue::Many(vec![
            FilterValue::Text("poems".into()),
            FilterValue::Text("essay".into()),
        ])),
    );
    assert_eq!(kept(&[whole]), ["Ember"]);
    let by = Condition::new(
        "author",
        FilterOp::Is,
        Some(FilterValue::Reference(SourceRef::new("people", "Ember"))),
    );
    assert_eq!(kept(&[by]), ["Ember"]);
}

#[test]
fn refusals_name_the_field() {
    let refused = |condition: Condition| {
        let refusals = Matcher::new(&[condition], &fields()).expect_err("refused");
        assert_eq!(refusals.len(), 1);
        refusals[0].1.clone()
    };
    let why = refused(Condition::new("shelf", FilterOp::IsEmpty, None));
    assert!(
        why.contains("\"shelf\"") && why.contains("does not disclose"),
        "{why}"
    );
    let why = refused(Condition::new(
        "read",
        FilterOp::Gt,
        Some(FilterValue::Boolean(true)),
    ));
    assert!(
        why.contains("\"read\"") && why.contains("cannot answer"),
        "{why}"
    );
    let why = refused(Condition::new("pages", FilterOp::Is, text("many")));
    assert!(
        why.contains("\"pages\"") && why.contains("not a number"),
        "{why}"
    );
    let why = refused(Condition::new("added", FilterOp::Is, text("yesterday")));
    assert!(why.contains("\"added\""), "{why}");
    let why = refused(Condition::new(
        "pages",
        FilterOp::Between,
        Some(FilterValue::Number(1.0)),
    ));
    assert!(why.contains("two values"), "{why}");
    let why = refused(Condition::new("title", FilterOp::IsEmpty, text("x")));
    assert!(why.contains("takes no value"), "{why}");
}

#[test]
fn a_condition_reads_and_writes_as_field_op_value() {
    let condition = Condition::new("pages", FilterOp::Ge, Some(FilterValue::Number(450.0)));
    let json = serde_json::to_value(&condition).unwrap();
    assert_eq!(
        json,
        serde_json::json!({ "field": "pages", "op": "ge", "value": 450.0 })
    );
    let back: Condition = serde_json::from_value(json).unwrap();
    assert_eq!(back, condition);
    let empty: Condition =
        serde_json::from_value(serde_json::json!({ "field": "pages", "op": "is_empty" })).unwrap();
    assert_eq!(empty.value, None);
    assert_eq!(condition.words("Pages"), "Pages is at least 450");
    assert_eq!(empty.words("Pages"), "Pages is empty");
}
