// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The `authority-guest` world: a pure component that exports `domain-authority` and
//! imports only `log`, so the gate to a domain can touch no key, vault, clock or store.

use wit_parser::{Resolve, WorldItem};

fn resolve() -> Resolve {
    let mut resolve = Resolve::default();
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("wit");
    resolve.push_dir(&dir).expect("world.wit parses");
    resolve
}

#[test]
fn authority_guest_exports_domain_authority_and_imports_only_log() {
    let resolve = resolve();
    let (_, world) = resolve
        .worlds
        .iter()
        .find(|(_, w)| w.name == "authority-guest")
        .expect("authority-guest world");
    let named = |item: &WorldItem| match item {
        WorldItem::Interface { id, .. } => resolve.interfaces[*id].name.clone(),
        _ => None,
    };
    let imports: Vec<String> = world.imports.values().filter_map(named).collect();
    let exports: Vec<String> = world.exports.values().filter_map(named).collect();
    assert_eq!(imports, ["log"], "unimported means unreachable");
    assert_eq!(exports, ["domain-authority"]);
    assert_eq!(world.imports.len(), 1);
}

#[test]
fn domain_authority_has_describe_validate_and_admit() {
    let resolve = resolve();
    let (_, iface) = resolve
        .interfaces
        .iter()
        .find(|(_, i)| i.name.as_deref() == Some("domain-authority"))
        .expect("domain-authority interface");
    let mut calls: Vec<&str> = iface.functions.keys().map(String::as_str).collect();
    calls.sort_unstable();
    assert_eq!(calls, ["admit", "describe", "validate"]);
    assert!(iface.types.contains_key("claim"), "claim record");
}
