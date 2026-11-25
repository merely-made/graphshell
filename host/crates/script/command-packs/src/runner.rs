// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Registration and run of pack commands (command packs CP2).

use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use document_host::mod_loader::{AdmittedPack, Contribution, Grants, LuaPackRuntime};
use hest::menu::{Command, CommandSet};
use hest::{EffectClass, Invocation};
use script_didge::{Budget, CommandLimits, DEFAULT_MAX_OPS, collect_invocations_with_context};

use crate::CommandContext;

/// What bounds one run: the script's fuel and the batch's retention limits.
/// The heap ceiling is didgeridoo's own (16 MiB).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PackLimits {
    pub budget: Budget,
    pub commands: CommandLimits,
}

impl Default for PackLimits {
    fn default() -> Self {
        Self {
            budget: Budget::Steps(DEFAULT_MAX_OPS),
            commands: CommandLimits::default(),
        }
    }
}

/// Why a run produced no invocations. A refusal voids the whole batch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// No registered pack command has this id.
    UnknownCommand { id: String },
    /// The command's source file could not be read.
    SourceUnreadable { id: String, why: String },
    /// The script failed, ran out of fuel or heap, or had a request refused
    /// by the host's offered set.
    Script { id: String, why: String },
    /// An invocation neither named by id nor covered by its effect class in
    /// the pack's grants. `class` is the host's declaration.
    NotGranted {
        pack: String,
        id: String,
        class: EffectClass,
    },
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownCommand { id } => write!(f, "no pack command {id}"),
            Self::SourceUnreadable { id, why } => write!(f, "{id}: source unreadable: {why}"),
            Self::Script { id, why } => write!(f, "{id}: {why}"),
            Self::NotGranted { pack, id, class } => write!(
                f,
                "pack {pack} is not granted {id}: neither the command nor its class {class:?}"
            ),
        }
    }
}

impl std::error::Error for Refusal {}

/// A registered command: where its source lives and what its pack may do.
#[derive(Clone)]
struct Entry {
    pack: String,
    root: PathBuf,
    source: String,
    /// Palettes the command appears on; empty is every target.
    targets: Vec<String>,
    /// The text admitted with a conformant pack (D10); `None` reads live.
    pinned: Option<String>,
    grants: Grants,
}

/// Registers packs into a host's `CommandSet` and runs their commands.
pub struct PackRunner {
    commands: Arc<Mutex<CommandSet>>,
    schema: String,
    limits: PackLimits,
    entries: Mutex<BTreeMap<String, Entry>>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl PackRunner {
    /// A runner over the host's `commands`; `schema` names the argument schema
    /// of the invocations it collects.
    pub fn new(commands: Arc<Mutex<CommandSet>>, schema: impl Into<String>) -> Self {
        Self {
            commands,
            schema: schema.into(),
            limits: PackLimits::default(),
            entries: Mutex::new(BTreeMap::new()),
        }
    }

    pub fn with_limits(mut self, limits: PackLimits) -> Self {
        self.limits = limits;
        self
    }

    /// Register `pack`'s commands after removing `old_ids`, under one lock
    /// pair. A conflict leaves everything as it was.
    fn replace_locked(
        &self,
        old_ids: &[String],
        pack: &AdmittedPack,
    ) -> Result<Vec<String>, String> {
        let mut commands = lock(&self.commands);
        let mut entries = lock(&self.entries);
        let fresh = pack_commands(pack);
        for (command, _) in &fresh {
            if commands.get(&command.id).is_some() && !old_ids.contains(&command.id) {
                return Err(format!("command {} is already registered", command.id));
            }
        }
        for id in old_ids {
            commands.unregister(id);
            entries.remove(id);
        }
        let mut done = Vec::new();
        for (command, entry) in fresh {
            done.push(command.id.clone());
            entries.insert(command.id.clone(), entry);
            commands.register(command);
        }
        Ok(done)
    }

    /// The pack commands listed on `target` (D13, D14): those naming it in
    /// `targets`, and those naming none.
    pub fn commands_for(&self, target: &str) -> Vec<Command> {
        let commands = lock(&self.commands);
        let entries = lock(&self.entries);
        entries
            .iter()
            .filter(|(_, e)| e.targets.is_empty() || e.targets.iter().any(|t| t == target))
            .filter_map(|(id, _)| commands.get(id).cloned())
            .collect()
    }

    /// Run the pack command `command_id` over `context`.
    ///
    /// A conformant pack runs the sources it was admitted with until it is
    /// reloaded; a dev-mode pack's source is read at each run (D10). Each invocation is checked
    /// against the pack's grants by its id, or by the effect class the host
    /// declared for it, never one the script names.
    ///
    /// # Errors
    ///
    /// A [`Refusal`] voids the batch; none of its invocations are returned.
    pub fn run(
        &self,
        command_id: &str,
        context: &CommandContext,
    ) -> Result<Vec<Invocation>, Refusal> {
        let entry = lock(&self.entries).get(command_id).cloned();
        let entry = entry.ok_or_else(|| Refusal::UnknownCommand {
            id: command_id.to_string(),
        })?;
        let id = command_id.to_string();
        let source = match entry.pinned.clone() {
            Some(text) => text,
            None => std::fs::read_to_string(entry.root.join(&entry.source)).map_err(|error| {
                Refusal::SourceUnreadable {
                    id: id.clone(),
                    why: error.to_string(),
                }
            })?,
        };
        let offered = lock(&self.commands).clone();
        let invocations = collect_invocations_with_context(
            &source,
            &offered,
            &self.schema,
            self.limits.budget,
            self.limits.commands,
            &context.to_script_value(),
        )
        .map_err(|why| Refusal::Script { id, why })?;
        for invocation in &invocations {
            // The class is the host's declaration; an id the host no longer
            // offers is refused rather than trusted at its script's word.
            let declared = offered.get(&invocation.id);
            let class = declared.map_or(invocation.effect, |declared| declared.effect);
            let granted = declared.is_some()
                && (entry.grants.commands.contains(&invocation.id)
                    || entry.grants.classes.contains(&class));
            if !granted {
                return Err(Refusal::NotGranted {
                    pack: entry.pack,
                    id: invocation.id.clone(),
                    class,
                });
            }
        }
        Ok(invocations)
    }
}

/// The commands `pack` contributes, each with its run entry.
fn pack_commands(pack: &AdmittedPack) -> Vec<(Command, Entry)> {
    let mod_id = &pack.manifest.mod_id;
    let mut out = Vec::new();
    for contribution in &pack.manifest.contributes {
        let Contribution::Command {
            id,
            label,
            effect,
            source,
            targets,
        } = contribution
        else {
            continue;
        };
        let entry = Entry {
            pack: mod_id.clone(),
            root: pack.root.clone(),
            source: source.clone(),
            targets: targets.clone(),
            pinned: pack.sources.get(source).cloned(),
            grants: pack.manifest.grants.clone(),
        };
        out.push((Command::new(id, label, mod_id, *effect), entry));
    }
    out
}

impl LuaPackRuntime for PackRunner {
    fn activate(&self, pack: &AdmittedPack) -> Result<Vec<String>, String> {
        self.replace_locked(&[], pack)
    }

    fn replace(&self, old: &AdmittedPack, new: &AdmittedPack) -> Result<Vec<String>, String> {
        let old_ids: Vec<String> = pack_commands(old)
            .into_iter()
            .map(|(command, _)| command.id.clone())
            .collect();
        self.replace_locked(&old_ids, new)
    }

    fn unregister(&self, command_id: &str) -> Result<(), String> {
        let mut commands = lock(&self.commands);
        let mut entries = lock(&self.entries);
        // Only ids this runner registered, never a host command.
        if entries.remove(command_id).is_some() {
            commands.unregister(command_id);
        }
        Ok(())
    }
}
