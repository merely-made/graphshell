// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use serde_json::{Value, json};

const MERE: &str = env!("CARGO_BIN_EXE_mere");

fn run(dir: &Path, args: &[&str]) -> Output {
    Command::new(MERE)
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn local_query_preserves_unbound_and_escaped_cells() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("data.ttl"),
        "<urn:book> <urn:title> \"Ada | Grace\\nVol. 1\" .",
    )
    .unwrap();
    fs::write(
        dir.path().join("read.rq"),
        "SELECT ?title ?missing WHERE { <urn:book> <urn:title> ?title }",
    )
    .unwrap();
    let result = run(
        dir.path(),
        &["query", "--graph", "data.ttl", "--query", "read.rq"],
    );
    assert!(result.status.success(), "{:?}", result.stderr);
    assert_eq!(
        serde_json::from_slice::<Value>(&result.stdout).unwrap(),
        json!([["Ada | Grace\nVol. 1", null]])
    );
}

/// D16: a carrier this build lacks is refused by name, before anything binds.
#[cfg(feature = "test-identity")]
#[test]
fn a_build_without_a_carrier_refuses_it_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let path = |name: &str| dir.path().join(name);
    fs::write(path("data.ttl"), "<urn:a> <urn:b> <urn:c> .").unwrap();
    fs::write(path("q.rq"), "SELECT * { ?s ?p ?o }").unwrap();
    fs::write(path("owner.seed"), "01".repeat(32)).unwrap();
    fs::write(path("reader.seed"), "02".repeat(32)).unwrap();
    let contact = run(
        dir.path(),
        &["--identity-seed", "reader.seed", "contact", "--out", "reader.contact"],
    );
    assert!(contact.status.success(), "{}", String::from_utf8_lossy(&contact.stderr));
    let built = [
        ("iroh", cfg!(feature = "iroh")),
        ("i2p", cfg!(feature = "i2p")),
        ("reticulum", cfg!(feature = "reticulum")),
        ("webrtc", cfg!(feature = "webrtc")),
    ];
    for (via, _) in built.iter().filter(|(_, built)| !built) {
        let invite = format!("{via}.invite");
        let result = run(
            dir.path(),
            &[
                "--identity-seed", "owner.seed", "serve", "--graph", "data.ttl",
                "--allow-query", "q.rq", "--contact", "reader.contact", "--invite", &invite,
                "--via", via, "--listen", "127.0.0.1:9",
            ],
        );
        let stderr = String::from_utf8_lossy(&result.stderr);
        assert!(!result.status.success(), "{via} served in a build without it");
        assert!(stderr.contains(&format!("--features {via}")), "{stderr}");
        assert!(!path(&invite).exists(), "no invitation for a carrier that never bound");
    }
}
