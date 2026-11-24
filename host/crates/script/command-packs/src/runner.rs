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

    /// Run the pack command `command_id` over `context`.
    ///
    /// The script's source is read at each run. Each invocation is checked
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
        let source = std::fs::read_to_string(entry.root.join(&entry.source)).map_err(|error| {
            Refusal::SourceUnreadable {
                id: id.clone(),
                why: error.to_string(),
            }
        })?;
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

impl LuaPackRuntime for PackRunner {
    fn activate(&self, pack: &AdmittedPack) -> Result<Vec<String>, String> {
        let mut commands = lock(&self.commands);
        let mut entries = lock(&self.entries);
        let mut done: Vec<String> = Vec::new();
        for contribution in &pack.manifest.contributes {
            let Contribution::Command {
                id,
                label,
                effect,
                source,
            } = contribution
            else {
                continue;
            };
            if commands.get(id).is_some() {
                for id in &done {
                    commands.unregister(id);
                    entries.remove(id);
                }
                return Err(format!("command {id} is already registered"));
            }
            let mod_id = &pack.manifest.mod_id;
            commands.register(Command::new(id, label, mod_id, *effect));
            entries.insert(
                id.clone(),
                Entry {
                    pack: mod_id.clone(),
                    root: pack.root.clone(),
                    source: source.clone(),
                    grants: pack.manifest.grants.clone(),
                },
            );
            done.push(id.clone());
        }
        Ok(done)
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
