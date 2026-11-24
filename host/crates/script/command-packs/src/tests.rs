// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Headless tests: packs are written to a scratch directory, admitted,
//! registered and run, with no app around them.

use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};

use document_host::mod_loader::loader::lua_pack;
use document_host::mod_loader::{
    AdmissionLevel, LuaPackRuntime, ModExtensionRecord, ModRegistry, ModStatus,
};
use hest::EffectClass;
use hest::menu::{Command, CommandSet};
use script_didge::{Budget, CommandLimits};
use script_engine_api::ScriptValue;

use super::*;

const SCHEMA: &str = "test.commands/v1";
const CURATION: &str = "classes = [\"Curation\"]";

fn host() -> Arc<Mutex<CommandSet>> {
    let mut set = CommandSet::new();
    for (id, effect) in [
        ("host:note", EffectClass::Curation),
        ("host:save", EffectClass::DomainTruth),
        ("host:send", EffectClass::ExternalEffect),
    ] {
        set.register(Command::new(id, id, "host", effect));
    }
    Arc::new(Mutex::new(set))
}

/// Write pack `p`: `grants` is the grants table body, each command is
/// (name, effect, lua source).
fn write_pack(root: &Path, grants: &str, commands: &[(&str, &str, &str)]) {
    let mut manifest = format!("abi = 1\nmod_id = \"p\"\nmod_type = \"lua\"\n[grants]\n{grants}\n");
    for (name, effect, lua) in commands {
        manifest.push_str(&format!(
            "[[contributes]]\nkind = \"command\"\nid = \"p:{name}\"\nlabel = \"{name}\"\n\
             effect = \"{effect}\"\nsource = \"{name}.lua\"\n"
        ));
        fs::write(root.join(format!("{name}.lua")), lua).unwrap();
    }
    fs::write(root.join("mod.toml"), manifest).unwrap();
}

struct Fixture {
    commands: Arc<Mutex<CommandSet>>,
    runner: PackRunner,
    _dir: tempfile::TempDir,
}

fn fixture_with(limits: PackLimits, grants: &str, commands: &[(&str, &str, &str)]) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    write_pack(dir.path(), grants, commands);
    let host = host();
    let runner = PackRunner::new(host.clone(), SCHEMA).with_limits(limits);
    let pack = lua_pack::admit(dir.path(), AdmissionLevel::Conformant).expect("pack admits");
    runner.activate(&pack).expect("registers");
    Fixture {
        commands: host,
        runner,
        _dir: dir,
    }
}

fn fixture(grants: &str, commands: &[(&str, &str, &str)]) -> Fixture {
    fixture_with(PackLimits::default(), grants, commands)
}

fn ctx() -> CommandContext {
    CommandContext::new()
        .with_selection(["n1", "n2"])
        .with_focus("graph", Some("g1".into()))
}

#[test]
fn a_granted_run_returns_invocations_built_from_the_context() {
    let lua = "command(\"host:note\", context.selection[2] .. \"@\" .. context.focus.target)";
    let f = fixture(CURATION, &[("tidy", "Curation", lua)]);
    let out = f.runner.run("p:tidy", &ctx()).unwrap();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].id, "host:note");
    assert_eq!(out[0].effect, EffectClass::Curation);
    assert_eq!(out[0].arguments, b"n2@g1".to_vec());
    assert_eq!(out[0].schema_id, SCHEMA);
}

#[test]
fn the_context_carries_its_version_arguments_and_host_extensions() {
    let lua =
        "command(\"host:note\", context.version .. context.extensions.mode .. context.arguments.n)";
    let f = fixture(CURATION, &[("tidy", "Curation", lua)]);
    let c = ctx()
        .with_extension("mode", ScriptValue::Str("x".into()))
        .with_argument("n", ScriptValue::Int(3));
    let out = f.runner.run("p:tidy", &c).unwrap();
    assert_eq!(out[0].arguments, b"1x3".to_vec());
}

#[test]
fn an_ungranted_id_is_refused_by_name_and_voids_the_batch() {
    let lua = "command(\"host:note\", \"\"); command(\"host:save\", \"\")";
    let f = fixture("commands = [\"host:note\"]", &[("go", "Curation", lua)]);
    let refusal = f.runner.run("p:go", &ctx()).unwrap_err();
    assert!(matches!(&refusal, Refusal::NotGranted { id, .. } if id == "host:save"));
    assert!(refusal.to_string().contains("host:save"));
}

#[test]
fn an_ungranted_class_is_refused_and_names_it() {
    let f = fixture(
        CURATION,
        &[("go", "Curation", "command(\"host:send\", \"\")")],
    );
    let refusal = f.runner.run("p:go", &ctx()).unwrap_err();
    assert!(matches!(
        refusal,
        Refusal::NotGranted {
            class: EffectClass::ExternalEffect,
            ..
        }
    ));
    assert!(refusal.to_string().contains("ExternalEffect"));
}

#[test]
fn an_id_grant_admits_one_command_outside_the_granted_classes() {
    let grants = "classes = [\"Curation\"]\ncommands = [\"host:save\"]";
    let f = fixture(
        grants,
        &[("go", "Curation", "command(\"host:save\", \"\")")],
    );
    assert_eq!(f.runner.run("p:go", &ctx()).unwrap().len(), 1);
}

#[test]
fn a_command_the_host_adds_later_to_a_granted_class_is_accepted() {
    let f = fixture(
        CURATION,
        &[("go", "Curation", "command(\"host:later\", \"\")")],
    );
    // Not offered yet: the script's request is refused by the host's set.
    assert!(matches!(
        f.runner.run("p:go", &ctx()),
        Err(Refusal::Script { .. })
    ));
    let later = Command::new("host:later", "Later", "host", EffectClass::Curation);
    f.commands.lock().unwrap().register(later);
    let out = f.runner.run("p:go", &ctx()).unwrap();
    assert_eq!(out[0].id, "host:later");
}

#[test]
fn the_class_is_the_hosts_declaration_not_the_packs() {
    // The pack files its own command under Curation and DomainTruth is
    // granted; the host declares host:send ExternalEffect.
    let grants = "classes = [\"Curation\", \"DomainTruth\"]";
    let f = fixture(
        grants,
        &[("go", "Curation", "command(\"host:send\", \"\")")],
    );
    assert!(matches!(
        f.runner.run("p:go", &ctx()),
        Err(Refusal::NotGranted {
            class: EffectClass::ExternalEffect,
            ..
        })
    ));
    // The host refiles the command under a granted class: now accepted.
    let refiled = Command::new("host:send", "Send", "host", EffectClass::Curation);
    f.commands.lock().unwrap().register(refiled);
    assert!(f.runner.run("p:go", &ctx()).is_ok());
}

#[test]
fn running_out_of_fuel_is_refused() {
    let limits = PackLimits {
        budget: Budget::Steps(1_000),
        commands: CommandLimits::default(),
    };
    let f = fixture_with(
        limits,
        CURATION,
        &[("spin", "Curation", "while true do end")],
    );
    let refusal = f.runner.run("p:spin", &ctx()).unwrap_err();
    assert!(
        matches!(&refusal, Refusal::Script { why, .. } if why.contains("operation") || why.contains("budget") || why.contains("fuel")),
        "{refusal}"
    );
}

#[test]
fn passing_the_heap_ceiling_is_refused() {
    let limits = PackLimits {
        budget: Budget::Steps(u64::MAX / 2),
        commands: CommandLimits::default(),
    };
    let lua = "local t = {} for i = 1, 3000000 do t[i] = {i, i} end command(\"host:note\", \"\")";
    let f = fixture_with(limits, CURATION, &[("hog", "Curation", lua)]);
    let refusal = f.runner.run("p:hog", &ctx()).unwrap_err();
    assert!(matches!(&refusal, Refusal::Script { why, .. } if why.contains("memory")));
}

#[test]
fn an_unknown_command_is_refused() {
    let f = fixture("classes = []", &[("go", "Curation", "")]);
    assert!(matches!(
        f.runner.run("p:nope", &ctx()),
        Err(Refusal::UnknownCommand { .. })
    ));
}

#[test]
fn a_failed_activation_registers_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let both = [("a", "Curation", ""), ("b", "Curation", "")];
    write_pack(dir.path(), CURATION, &both);
    let host = host();
    // The host already holds the pack's second command id.
    let held = Command::new("p:b", "B", "host", EffectClass::Curation);
    host.lock().unwrap().register(held);
    let before = host.lock().unwrap().clone();
    let runner = PackRunner::new(host.clone(), SCHEMA);
    let pack = lua_pack::admit(dir.path(), AdmissionLevel::Conformant).unwrap();
    let error = runner.activate(&pack).unwrap_err();
    assert!(error.contains("p:b"), "{error}");
    assert_eq!(*host.lock().unwrap(), before);
    assert!(matches!(
        runner.run("p:a", &ctx()),
        Err(Refusal::UnknownCommand { .. })
    ));
}

#[test]
fn load_all_registers_a_pack_and_a_record_rolls_back() {
    let dir = tempfile::tempdir().unwrap();
    write_pack(
        dir.path(),
        CURATION,
        &[("go", "Curation", "command(\"host:note\", \"\")")],
    );
    let host = host();
    let runner = Arc::new(PackRunner::new(host.clone(), SCHEMA));
    let mut registry = ModRegistry::new().with_lua_runtime(runner.clone());
    let level = AdmissionLevel::Conformant;
    let id = registry.admit_lua_pack(dir.path(), level).unwrap();
    registry.resolve_dependencies().unwrap();
    registry.load_all();
    assert_eq!(registry.get_status(&id), Some(ModStatus::Active));
    assert_eq!(
        registry.extension_records_for(&id).unwrap(),
        &[ModExtensionRecord::Action {
            action_id: "p:go".into()
        }]
    );
    assert!(host.lock().unwrap().get("p:go").is_some());
    assert_eq!(runner.run("p:go", &ctx()).unwrap().len(), 1);
    // Rolling the record back removes the command and its entry.
    runner.unregister("p:go").unwrap();
    assert!(host.lock().unwrap().get("p:go").is_none());
    assert!(matches!(
        runner.run("p:go", &ctx()),
        Err(Refusal::UnknownCommand { .. })
    ));
    // It never removes a host command.
    runner.unregister("host:note").unwrap();
    assert!(host.lock().unwrap().get("host:note").is_some());
}

#[test]
fn load_all_without_a_runtime_fails_a_pack() {
    let dir = tempfile::tempdir().unwrap();
    write_pack(dir.path(), "classes = []", &[("go", "Curation", "")]);
    let mut registry = ModRegistry::new();
    let level = AdmissionLevel::Conformant;
    let id = registry.admit_lua_pack(dir.path(), level).unwrap();
    registry.resolve_dependencies().unwrap();
    registry.load_all();
    assert_eq!(registry.get_status(&id), Some(ModStatus::Failed));
}
