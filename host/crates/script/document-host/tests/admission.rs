// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Mod projects MP1: Wasm admission at two levels, the install record, and
//! host-built `.cwasm` only (D2, D4, D7).
//!
//! Two fixture components stand in for "the project's component before and
//! after an edit": the standard guest (imports `net`) and the bomb guest
//! (imports nothing). Both are valid components with different bytes.

use std::path::{Path, PathBuf};

use document_host::mod_loader::{
    AdmissionLevel, InstallRecord, ModCapability, ModManifest, ModType, WasmModRuntime,
    WasmModSource, WasmRefusal, admit_wasm, hash_bytes,
};
use document_host::runtime::DocumentScriptRuntime;

fn fixture(var: &str, default: &str) -> PathBuf {
    let p = std::env::var(var).unwrap_or_else(|_| default.to_string());
    let path = PathBuf::from(&p);
    assert!(path.exists(), "fixture guest missing at {p}");
    path
}

fn guest() -> PathBuf {
    fixture(
        "DOC_HOST_GUEST_WASM",
        "guest/target/wasm32-wasip2/release/document_core_guest.wasm",
    )
}

fn bomb() -> PathBuf {
    fixture(
        "DOC_HOST_BOMB_WASM",
        "guest-bomb/target/wasm32-wasip2/release/document_core_guest_bomb.wasm",
    )
}

fn manifest(id: &str) -> ModManifest {
    ModManifest::new(
        id,
        id,
        ModType::Wasm,
        vec!["doc.surface".into()],
        vec![],
        vec![ModCapability::Network],
    )
}

fn source(path: &Path) -> WasmModSource {
    WasmModSource {
        module_path: path.to_path_buf(),
        manifest_path: PathBuf::from("mod.toml"),
    }
}

fn net_runtime() -> DocumentScriptRuntime {
    DocumentScriptRuntime::new(vec![ModCapability::Network])
}

/// A project folder holding `mod.wasm`, initially the standard guest.
fn project() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let wasm = dir.path().join("mod.wasm");
    std::fs::copy(guest(), &wasm).expect("copy guest");
    (dir, wasm)
}

fn edit_project(wasm: &Path) {
    std::fs::copy(bomb(), wasm).expect("swap in the edited component");
}

#[test]
fn conformant_pins_the_hash_and_refuses_a_changed_component_until_readmitted() {
    let (_dir, wasm) = project();
    let rt = net_runtime();
    let m = manifest("proj.pinned");

    let rec = rt
        .admit(&m, "1.0.0", &wasm, AdmissionLevel::Conformant, false)
        .expect("admit");
    assert_eq!(rec.source_hash, hash_bytes(&std::fs::read(&wasm).unwrap()));
    assert_eq!(rec.granted, vec!["network".to_string()]);
    assert_eq!(rec.provided, vec!["doc.surface".to_string()]);

    rt.activate(&m, &source(&wasm)).expect("pinned component activates");
    rt.deactivate("proj.pinned").expect("deactivate");

    edit_project(&wasm);
    let err = rt
        .activate(&m, &source(&wasm))
        .expect_err("a changed component must be refused");
    assert!(err.contains("changed since admission"), "got: {err}");
    assert!(!rt.is_active("proj.pinned"));
    assert!(
        rt.record("proj.pinned").unwrap().last_trap.is_some(),
        "the refusal is noted in the record"
    );

    // Admission against the existing pin refuses too, until re-admitted.
    let err = rt
        .admit(&m, "1.0.1", &wasm, AdmissionLevel::Conformant, false)
        .expect_err("admission against the pin refuses");
    assert!(matches!(err, WasmRefusal::ComponentChanged { .. }), "{err:?}");

    let rec2 = rt
        .admit(&m, "1.0.1", &wasm, AdmissionLevel::Conformant, true)
        .expect("re-admit");
    assert_ne!(rec2.source_hash, rec.source_hash);
    rt.activate(&m, &source(&wasm)).expect("re-admitted component activates");
}

#[test]
fn dev_mode_loads_the_changed_component_live_and_never_grants() {
    let (_dir, wasm) = project();
    let rt = net_runtime();
    let m = manifest("proj.dev");

    let rec = rt
        .admit(&m, "0.1.0", &wasm, AdmissionLevel::DevMode, false)
        .expect("admit dev");
    assert!(rec.granted.is_empty(), "dev mode never grants");
    // The standard guest imports net; nothing is granted, so it will not link.
    let err = rt
        .activate(&m, &source(&wasm))
        .expect_err("dev mode grants no net");
    assert!(err.contains("mere:script/net"), "got: {err}");

    // Edit the project: the next activation reads the new bytes, no re-admission.
    edit_project(&wasm);
    rt.activate(&m, &source(&wasm))
        .expect("dev mode loads the edited component live");
    assert!(rt.is_active("proj.dev"));
    rt.deactivate("proj.dev").unwrap();

    // Admitting again updates the tracked hash without refusing.
    let rec2 = rt
        .admit(&m, "0.1.0", &wasm, AdmissionLevel::DevMode, false)
        .expect("dev re-admit");
    assert_ne!(rec2.source_hash, rec.source_hash);
    assert!(rec2.granted.is_empty());
}

#[test]
fn foreign_cwasm_is_refused_before_any_deserialize() {
    let (dir, wasm) = project();
    let rt = net_runtime();
    let m = manifest("proj.foreign");
    rt.admit(&m, "1.0.0", &wasm, AdmissionLevel::Conformant, false)
        .expect("admit");

    // Garbage bytes: had the host called `deserialize`, the error would be a
    // deserialization failure. It must be the hash refusal instead.
    let junk = dir.path().join("junk.cwasm");
    std::fs::write(&junk, b"not a precompiled component").unwrap();
    let err = rt.activate(&m, &source(&junk)).expect_err("foreign cwasm");
    assert!(err.contains("was not built by this host"), "got: {err}");
    assert!(!err.to_lowercase().contains("deserial"), "got: {err}");

    // A genuine cwasm the host did not record is just as foreign.
    let genuine = dir.path().join("genuine.cwasm");
    std::fs::write(&genuine, document_host::precompile_to_cwasm(&wasm).unwrap()).unwrap();
    let err = rt.activate(&m, &source(&genuine)).expect_err("unrecorded cwasm");
    assert!(err.contains("was not built by this host"), "got: {err}");

    // No record at all: refused as well.
    let other = manifest("proj.unrecorded");
    let err = rt.activate(&other, &source(&genuine)).expect_err("no record");
    assert!(err.contains("not built by this host") || err.contains("install record"), "got: {err}");
    assert!(rt.active_ids().is_empty());
}

#[test]
fn host_built_cwasm_loads_and_a_tampered_one_does_not() {
    let (dir, wasm) = project();
    let rt = net_runtime();
    let m = manifest("proj.built");
    rt.admit(&m, "1.0.0", &wasm, AdmissionLevel::Conformant, false)
        .expect("admit");

    let cwasm = dir.path().join("mod.cwasm");
    let rec = rt.build_cwasm("proj.built", &wasm, &cwasm).expect("host build");
    assert_eq!(
        rec.build_hash.as_deref(),
        Some(hash_bytes(&std::fs::read(&cwasm).unwrap()).as_str())
    );
    assert!(rec.last_build.is_some());

    rt.activate(&m, &source(&cwasm)).expect("host-built cwasm loads");
    assert!(rt.is_active("proj.built"));
    rt.deactivate("proj.built").unwrap();

    let mut bytes = std::fs::read(&cwasm).unwrap();
    bytes.push(0);
    std::fs::write(&cwasm, bytes).unwrap();
    let err = rt.activate(&m, &source(&cwasm)).expect_err("tampered cwasm");
    assert!(err.contains("was not built by this host"), "got: {err}");

    // Dev mode never builds a cwasm.
    let dev = manifest("proj.devbuild");
    rt.admit(&dev, "0", &wasm, AdmissionLevel::DevMode, false).unwrap();
    assert!(rt.build_cwasm("proj.devbuild", &wasm, &cwasm).is_err());
}

#[test]
fn the_install_record_round_trips() {
    let (_dir, wasm) = project();
    let m = manifest("proj.record");
    let mut rec = admit_wasm(
        &m,
        "2.3.4",
        &wasm,
        AdmissionLevel::Conformant,
        &[ModCapability::Network],
        None,
    )
    .expect("admit");
    rec.note_build(hash_bytes(b"built"));
    rec.note_trap("boom");

    let json = rec.to_json().expect("serialize");
    assert!(json.contains("\"level\": \"conformant\""), "{json}");
    assert!(!json.to_lowercase().contains("signing"), "no key field yet (D7)");
    let back = InstallRecord::from_json(&json).expect("deserialize");
    assert_eq!(back, rec);
    assert_eq!(back.version, "2.3.4");
    assert_eq!(back.declared, vec!["network".to_string()]);
    assert!(back.last_check.is_some() && back.last_build.is_some() && back.last_trap.is_some());
}
