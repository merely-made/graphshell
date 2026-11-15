// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

use super::*;

const BASE: &str = "https://example.test/notes#";

fn sample() -> Vec<FieldSpec> {
    vec![
        FieldSpec {
            description: "What the note is called".into(),
            ..FieldSpec::new("title", "Title")
        },
        FieldSpec {
            group: Some("Status".into()),
            viewer: Some("chip".into()),
            ..FieldSpec::new("state", "State")
        },
        FieldSpec {
            group: Some("Status".into()),
            viewer: Some("bar".into()),
            ..FieldSpec::new("done ratio/2", "Done")
        },
        FieldSpec {
            viewer: Some("thumbnail".into()),
            ..FieldSpec::new("cover", "Cover")
        },
    ]
}

#[test]
fn a_shape_round_trip_keeps_every_term_and_the_order() {
    let fields = sample();
    let shape = to_shape(&fields, BASE);
    assert_eq!(from_shape(&shape, BASE).unwrap(), fields);

    let properties = shape["sh:property"].as_array().unwrap();
    assert_eq!(properties[2]["sh:order"], 2);
    // A key that is not a legal IRI segment is escaped, and comes back whole.
    assert_eq!(
        properties[2]["sh:path"]["@id"],
        format!("{BASE}done%20ratio%2F2")
    );
    assert_eq!(properties[0]["sh:description"], "What the note is called");
    assert_eq!(properties[1]["sh:group"]["rdfs:label"], "Status");
}

#[test]
fn a_viewer_exports_as_dash_where_dash_has_it_and_as_ours_otherwise() {
    let shape = to_shape(&sample(), BASE);
    let viewer = |i: usize| shape["sh:property"][i]["dash:viewer"]["@id"].clone();
    assert_eq!(viewer(1), format!("{VIEWER_NAMESPACE}chip"));
    assert_eq!(viewer(3), "dash:ImageViewer");
    // No viewer named: nothing written, and the default read back.
    assert!(shape["sh:property"][0].get("dash:viewer").is_none());
    assert_eq!(sample()[0].viewer_id(), DEFAULT_VIEWER);
}

#[test]
fn import_sorts_by_order_and_keeps_a_foreign_path_whole() {
    let shape = serde_json::json!({
        "sh:property": [
            { "sh:path": { "@id": "http://purl.org/dc/terms/created" }, "sh:name": "Created", "sh:order": 20 },
            { "sh:path": { "@id": format!("{BASE}title") }, "sh:name": "Title", "sh:order": 10 },
        ]
    });
    let fields = from_shape(&shape, BASE).unwrap();
    assert_eq!(fields[0].path, "title");
    assert_eq!(fields[1].path, "http://purl.org/dc/terms/created");
    assert!(from_shape(&serde_json::json!({}), BASE).is_err());
}

#[test]
fn a_field_without_a_path_or_a_name_and_a_repeated_path_are_reported() {
    let mut issues = Vec::new();
    let fields = [
        FieldSpec::new("title", "Title"),
        FieldSpec::new("title", ""),
        FieldSpec::new(" ", "Blank"),
    ];
    validate_fields(&mut issues, &fields);
    let named: Vec<&str> = issues.iter().map(|issue| issue.field.as_str()).collect();
    assert_eq!(named, ["fields[1].path", "fields[1].name", "fields[2].path"]);
}

#[test]
fn the_catalog_holds_the_first_tables_viewers_and_takes_more() {
    let catalog = ViewerCatalog::built_in();
    for id in [DEFAULT_VIEWER, "chip", "bar", "check"] {
        assert!(catalog.contains(id), "{id}");
    }
    assert!(!catalog.contains("thumbnail"));
    assert!(catalog.with("thumbnail").contains("thumbnail"));
}
