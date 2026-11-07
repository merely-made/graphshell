// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The `block-guest` world: a pure component that exports `block-adapter`, imports only
//! `log` and `caps`, and returns a view as a `sceno` scene plus a `vector-sheet`.

use wit_parser::{Resolve, WorldItem, WorldKey};

fn resolve() -> Resolve {
    let mut resolve = Resolve::default();
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("wit");
    resolve.push_dir(&dir).expect("world.wit parses");
    resolve
}

fn name(key: &WorldKey) -> String {
    match key {
        WorldKey::Name(n) => n.clone(),
        WorldKey::Interface(_) => "<interface>".into(),
    }
}

#[test]
fn block_guest_exports_block_adapter_and_imports_only_log_and_caps() {
    let resolve = resolve();
    let (_, world) = resolve
        .worlds
        .iter()
        .find(|(_, w)| w.name == "block-guest")
        .expect("block-guest world");
    let imports: Vec<String> = world
        .imports
        .iter()
        .map(|(key, item)| match item {
            WorldItem::Interface { id, .. } => resolve.interfaces[*id].name.clone().unwrap_or(name(key)),
            _ => name(key),
        })
        .collect();
    assert_eq!(imports, ["log", "caps"], "unimported means unreachable");
    let exports: Vec<String> = world
        .exports
        .iter()
        .filter_map(|(_, item)| match item {
            WorldItem::Interface { id, .. } => resolve.interfaces[*id].name.clone(),
            _ => None,
        })
        .collect();
    assert_eq!(exports, ["block-adapter"]);
}

#[test]
fn block_adapter_has_the_six_calls() {
    let resolve = resolve();
    let (_, iface) = resolve
        .interfaces
        .iter()
        .find(|(_, i)| i.name.as_deref() == Some("block-adapter"))
        .expect("block-adapter interface");
    let mut calls: Vec<&str> = iface.functions.keys().map(String::as_str).collect();
    calls.sort_unstable();
    assert_eq!(calls, ["apply", "choices", "describe", "rows", "run", "view"]);
    for record in ["claim", "block", "change", "view-ask", "view-result"] {
        assert!(iface.types.contains_key(record), "record {record}");
    }
}

#[test]
fn a_view_result_payload_is_a_versioned_sheet() {
    use vector_sheet::{Op, Paint, Path, Role, VectorSheet, FillRule, SHEET_VERSION};
    let sheet = VectorSheet {
        width: 10.0,
        height: 10.0,
        ops: vec![Op::Fill {
            path: Path::rect(0.0, 0.0, 5.0, 5.0),
            rule: FillRule::NonZero,
            paint: Paint::new(Role::Ink),
        }],
    };
    let payload = sheet.encode().unwrap();
    assert_eq!(payload[0], SHEET_VERSION);
    assert_eq!(VectorSheet::decode(&payload).unwrap(), sheet);
}
