// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

use std::fs;
use std::path::{Path, PathBuf};

use hest::EffectClass;

use super::*;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/command_pack")
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap().flatten() {
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// A scratch copy of the fixture, with `edit` applied to its manifest text.
fn pack_with(edit: impl FnOnce(String) -> String) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    copy_dir(&fixture(), dir.path());
    let path = dir.path().join(LUA_PACK_MANIFEST);
    let text = fs::read_to_string(&path).unwrap();
    fs::write(&path, edit(text)).unwrap();
    dir
}

fn refusal(dir: &tempfile::TempDir) -> PackRefusal {
    lua_pack::admit(dir.path(), AdmissionLevel::Conformant).expect_err("pack must be refused")
}

#[test]
fn the_fixture_pack_admits_conformant() {
    let pack = lua_pack::admit(&fixture(), AdmissionLevel::Conformant).expect("admits");
    assert_eq!(pack.level, AdmissionLevel::Conformant);
    assert!(pack.diagnostics.is_empty());
    assert_eq!(pack.manifest.mod_type, ModType::Lua);
    assert_eq!(pack.manifest.mod_id, "notes-pack");
    assert_eq!(pack.manifest.grants.classes, vec![EffectClass::Curation]);
    assert_eq!(pack.manifest.grants.commands, vec!["identity:webfinger"]);
    assert_eq!(
        pack.manifest.contributes[1],
        Contribution::Command {
            id: "notes-pack:publish".into(),
            label: "Publish the note".into(),
            effect: EffectClass::ExternalEffect,
            source: "publish.lua".into(),
        }
    );
}

#[test]
fn a_missing_manifest_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    assert!(matches!(refusal(&dir), PackRefusal::NoManifest { .. }));
}

#[test]
fn a_manifest_that_is_not_the_schema_is_refused_at_both_levels() {
    let dir = pack_with(|text| text.replace("abi = 1\n", ""));
    assert!(matches!(refusal(&dir), PackRefusal::MalformedSchema { .. }));
    assert!(matches!(
        lua_pack::admit(dir.path(), AdmissionLevel::DevMode),
        Err(PackRefusal::MalformedSchema { .. })
    ));
}

#[test]
fn an_unknown_key_is_refused() {
    let dir = pack_with(|text| text.replace("[grants]\n", "[grants]\nfloor = 1\n"));
    assert_eq!(
        refusal(&dir),
        PackRefusal::UnknownField {
            key: "grants.floor".into()
        }
    );
    let dir = pack_with(|text| text.replace("label =", "tooltip = \"x\"\nlabel ="));
    assert_eq!(
        refusal(&dir),
        PackRefusal::UnknownField {
            key: "contributes[0].tooltip".into()
        }
    );
}

#[test]
fn a_pack_of_another_mod_type_is_refused() {
    let dir = pack_with(|text| text.replace("\"lua\"", "\"wasm\""));
    assert_eq!(
        refusal(&dir),
        PackRefusal::NotLuaPack {
            found: "wasm".into()
        }
    );
}

#[test]
fn an_unknown_abi_is_refused() {
    let dir = pack_with(|text| text.replace("abi = 1", "abi = 2"));
    assert_eq!(
        refusal(&dir),
        PackRefusal::UnknownAbi {
            found: 2,
            supported: LUA_PACK_ABI
        }
    );
}

#[test]
fn a_contribution_that_is_not_a_command_is_refused() {
    let dir =
        pack_with(|text| text + "\n[[contributes]]\nkind = \"pane\"\nid = \"notes-pack:pane\"\n");
    assert_eq!(
        refusal(&dir),
        PackRefusal::ForeignContribution {
            id: "notes-pack:pane".into()
        }
    );
}

#[test]
fn an_unqualified_id_is_refused() {
    let dir = pack_with(|text| text.replace("notes-pack:tidy", "tidy"));
    assert_eq!(
        refusal(&dir),
        PackRefusal::UnqualifiedId {
            id: "tidy".into(),
            expected_prefix: "notes-pack:".into()
        }
    );
    let dir = pack_with(|text| text.replace("notes-pack:tidy", "other-pack:tidy"));
    assert!(matches!(refusal(&dir), PackRefusal::UnqualifiedId { .. }));
    let dir = pack_with(|text| text.replace("notes-pack:tidy", "notes-pack:"));
    assert!(matches!(refusal(&dir), PackRefusal::UnqualifiedId { .. }));
}

#[test]
fn a_duplicate_id_is_refused() {
    let dir = pack_with(|text| text.replace("notes-pack:publish", "notes-pack:tidy"));
    assert_eq!(
        refusal(&dir),
        PackRefusal::DuplicateId {
            id: "notes-pack:tidy".into()
        }
    );
}

#[test]
fn a_path_leaving_the_pack_is_refused() {
    let mut escapes = vec![
        "../outside.lua",
        "lib/../../outside.lua",
        "/etc/outside.lua",
    ];
    if cfg!(windows) {
        escapes.extend(["C:/outside.lua", "..\\outside.lua"]);
    }
    for escape in escapes {
        let dir = pack_with(|text| text.replace("tidy.lua", escape));
        let expected = PackRefusal::PathEscape {
            declared: escape.into(),
        };
        // A backslash in TOML needs escaping; compare the parsed value.
        let dir = if escape.contains('\\') {
            pack_with(|text| text.replace("tidy.lua", &escape.replace('\\', "\\\\")))
        } else {
            dir
        };
        assert_eq!(refusal(&dir), expected, "{escape}");
    }
}

#[test]
fn a_declared_file_that_is_missing_is_refused() {
    let dir = pack_with(|text| text.replace("tidy.lua", "gone.lua"));
    assert!(matches!(refusal(&dir), PackRefusal::Unreadable { path, .. } if path == "gone.lua"));
}

#[test]
fn a_source_that_is_not_lua_is_refused() {
    let dir = pack_with(|text| text.replace("tidy.lua", "notes.txt"));
    fs::write(dir.path().join("notes.txt"), "x").unwrap();
    assert_eq!(
        refusal(&dir),
        PackRefusal::NotLuaSource {
            path: "notes.txt".into()
        }
    );
}

#[test]
fn an_undeclared_file_is_refused_and_named_with_forward_slashes() {
    let dir = pack_with(|text| text);
    fs::create_dir_all(dir.path().join("deep")).unwrap();
    fs::write(dir.path().join("deep").join("stray.lua"), "return {}").unwrap();
    fs::write(dir.path().join("second.toml"), "").unwrap();
    assert_eq!(
        refusal(&dir),
        PackRefusal::UndeclaredFile {
            path: "deep/stray.lua".into()
        }
    );
}

#[test]
fn a_declared_path_spelled_another_way_is_the_same_file() {
    let dir = pack_with(|text| text.replace("\"lib/util.lua\"", "\"./lib//util.lua\""));
    assert!(lua_pack::admit(dir.path(), AdmissionLevel::Conformant).is_ok());
    if cfg!(windows) {
        let dir = pack_with(|text| text.replace("\"lib/util.lua\"", "\"lib\\\\util.lua\""));
        assert!(lua_pack::admit(dir.path(), AdmissionLevel::Conformant).is_ok());
    }
}

#[test]
fn a_pack_with_no_commands_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join(LUA_PACK_MANIFEST),
        "abi = 1\nmod_id = \"empty\"\nmod_type = \"lua\"\n",
    )
    .unwrap();
    assert!(matches!(refusal(&dir), PackRefusal::EmptyPack { .. }));
}

#[test]
fn dev_mode_loads_the_broken_pack_and_lists_what_conformant_refuses() {
    let dir = pack_with(|text| {
        text.replace("abi = 1", "abi = 9")
            .replace("[grants]\n", "[grants]\nfloor = 1\n")
            .replace("notes-pack:publish", "publish")
    });
    fs::write(dir.path().join("stray.lua"), "return {}").unwrap();
    let pack = lua_pack::admit(dir.path(), AdmissionLevel::DevMode).expect("dev mode loads");
    assert_eq!(pack.level, AdmissionLevel::DevMode);
    assert_eq!(pack.manifest.contributes.len(), 2);
    assert_eq!(
        pack.diagnostics,
        vec![
            PackRefusal::UnknownField {
                key: "grants.floor".into()
            },
            PackRefusal::UnknownAbi {
                found: 9,
                supported: LUA_PACK_ABI
            },
            PackRefusal::UnqualifiedId {
                id: "publish".into(),
                expected_prefix: "notes-pack:".into()
            },
            PackRefusal::UndeclaredFile {
                path: "stray.lua".into()
            },
        ]
    );
}

#[test]
fn dev_mode_skips_duplicates_and_escapes_with_a_diagnostic() {
    let dir = pack_with(|text| {
        text.replace("notes-pack:publish", "notes-pack:tidy")
            .replace("publish.lua", "../outside.lua")
    });
    let pack = lua_pack::admit(dir.path(), AdmissionLevel::DevMode).expect("dev mode loads");
    let ids: Vec<_> = pack
        .manifest
        .contributes
        .iter()
        .map(Contribution::id)
        .collect();
    assert_eq!(ids, ["notes-pack:tidy"]);
    assert!(pack.diagnostics.contains(&PackRefusal::DuplicateId {
        id: "notes-pack:tidy".into()
    }));
    assert!(pack.diagnostics.contains(&PackRefusal::PathEscape {
        declared: "../outside.lua".into()
    }));
}

#[test]
fn dev_mode_never_changes_grants() {
    let dir = pack_with(|text| text.replace("abi = 1", "abi = 9"));
    let loose = lua_pack::admit(dir.path(), AdmissionLevel::DevMode).unwrap();
    let strict = lua_pack::admit(&fixture(), AdmissionLevel::Conformant).unwrap();
    assert_eq!(loose.manifest.grants, strict.manifest.grants);
}

#[test]
fn the_registry_records_the_level_each_pack_was_admitted_at() {
    let mut registry = ModRegistry::from_manifests_for_tests(Vec::new());
    let id = registry
        .admit_lua_pack(fixture(), AdmissionLevel::Conformant)
        .expect("admits");
    assert_eq!(
        registry.admission_level(&id),
        Some(AdmissionLevel::Conformant)
    );
    assert_eq!(registry.get_status(&id), Some(ModStatus::Discovered));
    assert_eq!(registry.get_manifest(&id).unwrap().mod_type, ModType::Lua);
    assert_eq!(registry.lua_pack(&id).unwrap().root, fixture());

    let dev = pack_with(|text| {
        text.replace("notes-pack", "dev-pack")
            .replace("abi = 1", "abi = 9")
    });
    let dev_id = registry
        .admit_lua_pack(dev.path(), AdmissionLevel::DevMode)
        .expect("admits in dev mode");
    assert_eq!(
        registry.admission_level(&dev_id),
        Some(AdmissionLevel::DevMode)
    );
    assert!(!registry.lua_pack(&dev_id).unwrap().diagnostics.is_empty());
    assert_eq!(registry.admission_level("mod:core-viewer"), None);
}

#[test]
fn a_refused_pack_is_not_registered_and_a_known_mod_id_is_not_taken_twice() {
    let mut registry = ModRegistry::from_manifests_for_tests(Vec::new());
    let broken = pack_with(|text| text.replace("abi = 1", "abi = 2"));
    assert!(
        registry
            .admit_lua_pack(broken.path(), AdmissionLevel::Conformant)
            .is_err()
    );
    assert!(registry.get_manifest("notes-pack").is_none());
    registry
        .admit_lua_pack(fixture(), AdmissionLevel::Conformant)
        .unwrap();
    assert_eq!(
        registry.admit_lua_pack(fixture(), AdmissionLevel::DevMode),
        Err(PackRefusal::DuplicateModId {
            mod_id: "notes-pack".into()
        })
    );
    assert_eq!(
        registry.admission_level("notes-pack"),
        Some(AdmissionLevel::Conformant)
    );
}
