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
    AdmissionLevel, LuaPackRuntime, ModExtensionRecord, ModRegistry, ModStatus, PackLifecycleError,
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
    write_pack_as(root, "p", grants, commands);
}

fn write_pack_as(root: &Path, pack: &str, grants: &str, commands: &[(&str, &str, &str)]) {
    let mut manifest =
        format!("abi = 1\nmod_id = \"{pack}\"\nmod_type = \"lua\"\n[grants]\n{grants}\n");
    for (name, effect, lua) in commands {
        manifest.push_str(&format!(
            "[[contributes]]\nkind = \"command\"\nid = \"{pack}:{name}\"\nlabel = \"{name}\"\n\
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

// Lifecycle (command packs CP3): install, unload and reload in a running registry.

const NOTE: &str = "command(\"host:note\", \"\")";

struct Live {
    commands: Arc<Mutex<CommandSet>>,
    runner: Arc<PackRunner>,
    registry: ModRegistry,
}

fn live() -> Live {
    let commands = host();
    let runner = Arc::new(PackRunner::new(commands.clone(), SCHEMA));
    let registry = ModRegistry::new().with_lua_runtime(runner.clone());
    Live {
        commands,
        runner,
        registry,
    }
}

fn registered(live: &Live, id: &str) -> bool {
    live.commands.lock().unwrap().get(id).is_some()
}

fn argument(live: &Live, id: &str) -> Vec<u8> {
    live.runner.run(id, &ctx()).unwrap().remove(0).arguments
}

fn say(text: &str) -> String {
    format!("command(\"host:note\", \"{text}\")")
}

#[test]
fn install_registers_a_pack_in_a_running_registry() {
    let dir = tempfile::tempdir().unwrap();
    write_pack(dir.path(), CURATION, &[("go", "Curation", NOTE)]);
    let mut l = live();
    let id = l
        .registry
        .install_lua_pack(dir.path(), AdmissionLevel::Conformant)
        .unwrap();
    assert_eq!(l.registry.get_status(&id), Some(ModStatus::Active));
    assert!(registered(&l, "p:go"));
    assert_eq!(
        l.registry.extension_records_for(&id).unwrap(),
        &[ModExtensionRecord::Action {
            action_id: "p:go".into()
        }]
    );
    // A second install of the same id is refused and changes nothing.
    assert!(
        l.registry
            .install_lua_pack(dir.path(), AdmissionLevel::Conformant)
            .is_err()
    );
    assert_eq!(l.runner.run("p:go", &ctx()).unwrap().len(), 1);
}

#[test]
fn an_install_the_host_cannot_register_leaves_nothing() {
    let dir = tempfile::tempdir().unwrap();
    write_pack(dir.path(), CURATION, &[("go", "Curation", NOTE)]);
    let mut l = live();
    let held = Command::new("p:go", "Go", "host", EffectClass::Curation);
    l.commands.lock().unwrap().register(held);
    let error = l
        .registry
        .install_lua_pack(dir.path(), AdmissionLevel::Conformant)
        .unwrap_err();
    assert!(
        matches!(error, PackLifecycleError::Activation { .. }),
        "{error}"
    );
    assert!(l.registry.get_status("p").is_none());
    assert!(l.registry.lua_pack("p").is_none());
}

#[test]
fn a_reload_changes_a_commands_behaviour_without_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    write_pack(dir.path(), CURATION, &[("go", "Curation", NOTE)]);
    let mut l = live();
    let id = l
        .registry
        .install_lua_pack(dir.path(), AdmissionLevel::Conformant)
        .unwrap();
    assert!(!registered(&l, "p:more"));
    // The edit adds a command, widens the grants and changes what go asks for.
    let grants = "classes = [\"Curation\", \"DomainTruth\"]";
    let save = "command(\"host:save\", \"\")";
    write_pack(
        dir.path(),
        grants,
        &[("go", "Curation", save), ("more", "Curation", NOTE)],
    );
    // Nothing watches the disk: the old pack still runs as it did.
    assert_eq!(l.runner.run("p:go", &ctx()).unwrap()[0].id, "host:note");
    assert!(!registered(&l, "p:more"));
    l.registry.reload_lua_pack(&id).unwrap();
    assert_eq!(l.runner.run("p:go", &ctx()).unwrap()[0].id, "host:save");
    assert!(registered(&l, "p:more"));
    assert_eq!(l.registry.get_status(&id), Some(ModStatus::Active));
    assert_eq!(l.registry.extension_records_for(&id).unwrap().len(), 2);
    // A command the new version dropped leaves the host.
    fs::remove_file(dir.path().join("more.lua")).unwrap();
    write_pack(dir.path(), grants, &[("go", "Curation", save)]);
    l.registry.reload_lua_pack(&id).unwrap();
    assert!(!registered(&l, "p:more"));
    assert!(matches!(
        l.runner.run("p:more", &ctx()),
        Err(Refusal::UnknownCommand { .. })
    ));
}

#[test]
fn a_conformant_pack_keeps_its_admitted_sources_until_reloaded() {
    let dir = tempfile::tempdir().unwrap();
    write_pack(dir.path(), CURATION, &[("go", "Curation", &say("one"))]);
    let mut l = live();
    let id = l
        .registry
        .install_lua_pack(dir.path(), AdmissionLevel::Conformant)
        .unwrap();
    assert_eq!(argument(&l, "p:go"), b"one".to_vec());
    fs::write(dir.path().join("go.lua"), say("two")).unwrap();
    assert_eq!(argument(&l, "p:go"), b"one".to_vec());
    l.registry.reload_lua_pack(&id).unwrap();
    assert_eq!(argument(&l, "p:go"), b"two".to_vec());
}

#[test]
fn a_dev_mode_pack_reads_its_file_at_each_run() {
    let dir = tempfile::tempdir().unwrap();
    write_pack(dir.path(), CURATION, &[("go", "Curation", &say("one"))]);
    let mut l = live();
    l.registry
        .install_lua_pack(dir.path(), AdmissionLevel::DevMode)
        .unwrap();
    assert!(l.registry.lua_pack("p").unwrap().sources.is_empty());
    assert_eq!(argument(&l, "p:go"), b"one".to_vec());
    fs::write(dir.path().join("go.lua"), say("two")).unwrap();
    assert_eq!(argument(&l, "p:go"), b"two".to_vec());
}

#[test]
fn an_unload_removes_the_packs_commands_and_entries() {
    let dir = tempfile::tempdir().unwrap();
    write_pack(
        dir.path(),
        CURATION,
        &[("a", "Curation", NOTE), ("b", "Curation", NOTE)],
    );
    let mut l = live();
    let id = l
        .registry
        .install_lua_pack(dir.path(), AdmissionLevel::Conformant)
        .unwrap();
    l.registry.unload_lua_pack(&id).unwrap();
    for name in ["p:a", "p:b"] {
        assert!(!registered(&l, name));
        assert!(matches!(
            l.runner.run(name, &ctx()),
            Err(Refusal::UnknownCommand { .. })
        ));
    }
    assert!(registered(&l, "host:note"));
    assert!(l.registry.get_status(&id).is_none());
    assert!(matches!(
        l.registry.unload_lua_pack(&id),
        Err(PackLifecycleError::UnknownPack(_))
    ));
    // The same pack installs again.
    l.registry
        .install_lua_pack(dir.path(), AdmissionLevel::Conformant)
        .unwrap();
    assert!(registered(&l, "p:a"));
}

#[test]
fn a_reload_that_fails_admission_keeps_the_last_good_version() {
    let dir = tempfile::tempdir().unwrap();
    write_pack(dir.path(), CURATION, &[("go", "Curation", &say("one"))]);
    let mut l = live();
    let id = l
        .registry
        .install_lua_pack(dir.path(), AdmissionLevel::Conformant)
        .unwrap();
    let before = l.commands.lock().unwrap().clone();
    // The edit leaves a stray file the manifest does not declare.
    write_pack(dir.path(), CURATION, &[("go", "Curation", &say("two"))]);
    fs::write(dir.path().join("stray.lua"), "").unwrap();
    let error = l.registry.reload_lua_pack(&id).unwrap_err();
    assert!(matches!(error, PackLifecycleError::Refused(_)), "{error}");
    assert!(error.to_string().contains("stray.lua"), "{error}");
    assert_eq!(*l.commands.lock().unwrap(), before);
    assert_eq!(l.registry.get_status(&id), Some(ModStatus::Active));
    assert_eq!(argument(&l, "p:go"), b"one".to_vec());
}

#[test]
fn a_reload_that_fails_activation_keeps_the_last_good_version() {
    let dir = tempfile::tempdir().unwrap();
    write_pack(dir.path(), CURATION, &[("go", "Curation", &say("one"))]);
    let mut l = live();
    let id = l
        .registry
        .install_lua_pack(dir.path(), AdmissionLevel::Conformant)
        .unwrap();
    // The host holds an id the new version wants.
    let held = Command::new("p:new", "New", "host", EffectClass::Curation);
    l.commands.lock().unwrap().register(held);
    let before = l.commands.lock().unwrap().clone();
    write_pack(
        dir.path(),
        CURATION,
        &[("go", "Curation", &say("two")), ("new", "Curation", NOTE)],
    );
    let error = l.registry.reload_lua_pack(&id).unwrap_err();
    assert!(
        matches!(&error, PackLifecycleError::Activation { reason, .. } if reason.contains("p:new"))
    );
    assert_eq!(*l.commands.lock().unwrap(), before);
    assert_eq!(argument(&l, "p:go"), b"one".to_vec());
    assert_eq!(l.registry.get_manifest(&id).unwrap().contributes.len(), 1);
}

#[test]
fn reload_all_reloads_each_pack_and_reports_each_failure() {
    let (da, db) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    write_pack_as(da.path(), "a", CURATION, &[("go", "Curation", &say("a1"))]);
    write_pack_as(db.path(), "b", CURATION, &[("go", "Curation", &say("b1"))]);
    let mut l = live();
    for dir in [&da, &db] {
        l.registry
            .install_lua_pack(dir.path(), AdmissionLevel::Conformant)
            .unwrap();
    }
    write_pack_as(da.path(), "a", CURATION, &[("go", "Curation", &say("a2"))]);
    fs::write(db.path().join("mod.toml"), "not toml [").unwrap();
    let results = l.registry.reload_all_lua_packs();
    let ids: Vec<&str> = results.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(ids, ["a", "b"]);
    assert!(results[0].1.is_ok());
    assert!(matches!(results[1].1, Err(PackLifecycleError::Refused(_))));
    assert_eq!(argument(&l, "a:go"), b"a2".to_vec());
    assert_eq!(argument(&l, "b:go"), b"b1".to_vec());
}

#[test]
fn commands_for_a_target_are_those_naming_it_and_those_naming_none() {
    let dir = tempfile::tempdir().unwrap();
    let manifest = "abi = 1\nmod_id = \"p\"\nmod_type = \"lua\"\n[grants]\nclasses = [\"Curation\"]\n\
        [[contributes]]\nkind = \"command\"\nid = \"p:any\"\nlabel = \"any\"\neffect = \"Curation\"\nsource = \"a.lua\"\n\
        [[contributes]]\nkind = \"command\"\nid = \"p:app\"\nlabel = \"app\"\neffect = \"Curation\"\nsource = \"b.lua\"\ntargets = [\"app\"]\n\
        [[contributes]]\nkind = \"command\"\nid = \"p:both\"\nlabel = \"both\"\neffect = \"Curation\"\nsource = \"c.lua\"\ntargets = [\"app\", \"viewer\"]\n";
    for f in ["a.lua", "b.lua", "c.lua"] {
        fs::write(dir.path().join(f), "").unwrap();
    }
    fs::write(dir.path().join("mod.toml"), manifest).unwrap();
    let runner = PackRunner::new(host(), SCHEMA);
    let pack = lua_pack::admit(dir.path(), AdmissionLevel::Conformant).expect("admits");
    runner.activate(&pack).unwrap();
    let ids = |t: &str| -> Vec<String> {
        runner.commands_for(t).into_iter().map(|c| c.id).collect()
    };
    assert_eq!(ids("app"), ["p:any", "p:app", "p:both"]);
    assert_eq!(ids("viewer"), ["p:any", "p:both"]);
    assert_eq!(ids("other"), ["p:any"]);
}
