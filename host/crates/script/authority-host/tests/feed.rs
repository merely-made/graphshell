// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The graph feed is the declared object: a snapshot and the facet store.

#[test]
fn the_graph_feed_parses_as_a_snapshot_and_facets() {
    let graph = kernel::graph::Graph::new();
    let bytes = authority_host::encode_graph_feed(&graph).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let keys: Vec<&str> = value.as_object().unwrap().keys().map(String::as_str).collect();
    assert_eq!(keys, ["facets", "snapshot"]);
    assert!(value["snapshot"].is_object());
}
