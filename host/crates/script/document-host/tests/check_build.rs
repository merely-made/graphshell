// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Mod projects MP2: the host's `check` and the optional build adapter (D1).

use std::path::{Path, PathBuf};

use document_host::mod_loader::{
    AdmissionLevel, ModCapability, ModManifest, ModType, WasmRefusal,
};
use document_host::runtime::DocumentScriptRuntime;
use document_host::{
    BuildError, CapPermission, CargoBuilder, ComponentBuilder, Grant, World, check,
};

fn guest() -> PathBuf {
    let p = std::env::var("DOC_HOST_GUEST_WASM").unwrap_or_else(|_| {
        "guest/target/wasm32-wasip2/release/document_core_guest.wasm".to_string()
    });
    let path = PathBuf::from(&p);
    assert!(path.exists(), "fixture guest missing at {p}");
    path
}

#[test]
fn check_passes_the_guest_under_its_grants() {
    let report = check(&guest(), &Grant::allow_all(), &World::document_core()).expect("loads");
    assert!(report.ok(), "{}", report.words());
    assert!(report.imports.contains(&"mere:script/net".to_string()));
    assert!(report.exports.contains(&"handle-event".to_string()));
}

#[test]
fn check_names_an_ungranted_import() {
    let grant = Grant {
        net: CapPermission::Deny,
        ..Grant::allow_all()
    };
    let report = check(&guest(), &grant, &World::document_core()).expect("loads");
    assert!(!report.ok());
    assert_eq!(report.ungranted_imports, vec!["mere:script/net".to_string()]);
    assert!(report.missing_exports.is_empty());
    assert!(report.words().contains("import mere:script/net is not granted"));
}

#[test]
fn check_names_a_missing_export() {
    let world = World::document_core().requiring("on-event");
    let report = check(&guest(), &Grant::allow_all(), &world).expect("loads");
    assert!(!report.ok());
    assert_eq!(report.missing_exports, vec!["on-event".to_string()]);
    assert!(report.ungranted_imports.is_empty());
    assert!(report.words().contains("export on-event is missing"));
}

#[test]
fn check_refuses_what_is_not_a_component() {
    let dir = tempfile::tempdir().expect("tempdir");
    let junk = dir.path().join("junk.wasm");
    std::fs::write(&junk, b"not wasm").expect("write");
    let r = check(&junk, &Grant::allow_all(), &World::document_core());
    assert!(matches!(r, Err(WasmRefusal::NotAComponent { .. })), "{r:?}");
    let r = check(&dir.path().join("absent.wasm"), &Grant::allow_all(), &World::document_core());
    assert!(matches!(r, Err(WasmRefusal::Unreadable { .. })), "{r:?}");
}

fn manifest(caps: Vec<ModCapability>) -> ModManifest {
    ModManifest::new("mod:checked", "checked", ModType::Wasm, vec![], vec![], caps)
}

#[test]
fn the_runtime_records_the_check_on_the_install_record() {
    let rt = DocumentScriptRuntime::new(vec![ModCapability::Network]);
    let with_net = manifest(vec![ModCapability::Network]);
    rt.admit(&with_net, "1", &guest(), AdmissionLevel::Conformant, false)
        .expect("admit");
    let ok = rt
        .check(&with_net, &guest(), &World::document_core())
        .expect("check");
    assert!(ok.ok());
    assert_eq!(
        rt.record("mod:checked").and_then(|r| r.last_check).map(|e| e.note),
        Some("check passed".to_string())
    );
    // Declaring no network leaves `net` unlinked, so the guest's import is named.
    let without = manifest(vec![]);
    let bad = rt
        .check(&without, &guest(), &World::document_core())
        .expect("check");
    assert_eq!(bad.ungranted_imports, vec!["mere:script/net".to_string()]);
    let note = rt.record("mod:checked").and_then(|r| r.last_check).expect("recorded");
    assert!(note.note.contains("mere:script/net"), "{}", note.note);
}

/// A crate that builds to a component with no WIT of its own.
fn write_project(dir: &Path) {
    std::fs::create_dir_all(dir.join("src")).expect("src");
    std::fs::write(
        dir.join("Cargo.toml"),
        "[workspace]\n\n[package]\nname = \"tiny-mod\"\nversion = \"0.0.1\"\nedition = \"2021\"\npublish = false\n\n[lib]\ncrate-type = [\"cdylib\"]\n",
    )
    .expect("manifest");
    std::fs::write(dir.join("src/lib.rs"), "#[no_mangle]\npub extern \"C\" fn nothing() {}\n")
        .expect("lib");
}

#[test]
fn the_cargo_adapter_builds_a_fixture_project() {
    let dir = tempfile::tempdir().expect("tempdir");
    let project = dir.path().join("project");
    write_project(&project);
    let builder = CargoBuilder::new().with_target_dir(dir.path().join("out"));
    // No toolchain here is a skip, as build.rs warns for its guests (mod-projects D13).
    let built = match builder.build(&project) {
        Ok(built) => built,
        Err(e @ (BuildError::ToolchainAbsent { .. } | BuildError::TargetMissing { .. })) => {
            eprintln!("skipped: {e}");
            return;
        },
        Err(e) => panic!("the fixture project did not build: {e}"),
    };
    assert!(built.path.exists());
    assert!(built.path.starts_with(dir.path()), "built outside the temp dir");
    // It is a component the host can load.
    let report = check(&built.path, &Grant::allow_all(), &World::document_core()).expect("loads");
    assert_eq!(report.missing_exports.len(), 3, "an empty component exports nothing");

    let rt = DocumentScriptRuntime::permissive();
    let m = manifest(vec![]);
    rt.admit(&m, "1", &guest(), AdmissionLevel::DevMode, false).expect("admit");
    rt.build_project("mod:checked", &builder, &project).expect("build");
    let note = rt.record("mod:checked").and_then(|r| r.last_build).expect("recorded");
    assert!(note.note.starts_with("built "), "{}", note.note);
}

#[test]
fn the_cargo_adapter_reports_a_missing_cargo_plainly() {
    let dir = tempfile::tempdir().expect("tempdir");
    let project = dir.path().join("project");
    write_project(&project);
    let builder = CargoBuilder::new().with_cargo(dir.path().join("no-such-cargo"));
    let err = builder.build(&project).expect_err("no cargo");
    assert!(matches!(err, BuildError::ToolchainAbsent { .. }), "{err:?}");
    assert!(err.words().contains("use a prebuilt component"), "{}", err.words());

    let rt = DocumentScriptRuntime::permissive();
    rt.admit(&manifest(vec![]), "1", &guest(), AdmissionLevel::DevMode, false)
        .expect("admit");
    assert!(rt.build_project("mod:checked", &builder, &project).is_err());
    let note = rt.record("mod:checked").and_then(|r| r.last_build).expect("recorded");
    assert!(note.note.contains("no build toolchain"), "{}", note.note);
}

#[test]
fn the_cargo_adapter_refuses_a_directory_that_is_no_project() {
    let dir = tempfile::tempdir().expect("tempdir");
    let err = CargoBuilder::new().build(dir.path()).expect_err("no manifest");
    assert!(matches!(err, BuildError::BadProject(_)), "{err:?}");
}
