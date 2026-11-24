// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Install, unload and reload of Lua command packs in a running registry
//! (command packs CP3, D9, D10). Nothing here watches the disk: a reload runs
//! when the host or the user asks.

use std::path::Path;
use std::sync::Arc;

use super::ModRegistry;
use crate::mod_loader::loader::contributes::verify_records;
use crate::mod_loader::loader::free_fns::LuaPackRuntime;
use crate::mod_loader::loader::lua_pack::{self, AdmissionLevel, AdmittedPack, PackLifecycleError};
use crate::mod_loader::loader::types::{ModExtensionRecord, ModStatus};

fn action_records(ids: Vec<String>) -> Vec<ModExtensionRecord> {
    ids.into_iter()
        .map(|action_id| ModExtensionRecord::Action { action_id })
        .collect()
}

impl ModRegistry {
    fn lua_runtime_or_err(&self) -> Result<Arc<dyn LuaPackRuntime>, PackLifecycleError> {
        self.lua_runtime
            .clone()
            .ok_or(PackLifecycleError::NoRuntime)
    }

    /// Admit the pack at `root` at `level` and activate it now, registering
    /// its commands in the running host. A disabled mod id is admitted
    /// `Unloaded` and not activated.
    ///
    /// # Errors
    ///
    /// Admission or activation failed; nothing is left registered.
    pub fn install_lua_pack(
        &mut self,
        root: impl AsRef<Path>,
        level: AdmissionLevel,
    ) -> Result<String, PackLifecycleError> {
        let runtime = self.lua_runtime_or_err()?;
        let mod_id = self
            .admit_lua_pack(root, level)
            .map_err(PackLifecycleError::Refused)?;
        if self.status.get(&mod_id) == Some(&ModStatus::Unloaded) {
            return Ok(mod_id);
        }
        let pack = self.lua_packs[&mod_id].clone();
        match Self::activate_pack(runtime.as_ref(), &pack) {
            Ok(records) => {
                self.status.insert(mod_id.clone(), ModStatus::Active);
                self.extension_records.insert(mod_id.clone(), records);
                Ok(mod_id)
            },
            Err(error) => {
                self.forget_lua_pack(&mod_id);
                Err(error)
            },
        }
    }

    /// Deactivate the pack `mod_id`, removing its commands from the host,
    /// and forget it; it can be installed again.
    ///
    /// # Errors
    ///
    /// The pack is unknown, or a command could not be removed (the pack is
    /// then quarantined and stays known).
    pub fn unload_lua_pack(&mut self, mod_id: &str) -> Result<(), PackLifecycleError> {
        if !self.lua_packs.contains_key(mod_id) {
            return Err(PackLifecycleError::UnknownPack(mod_id.to_string()));
        }
        let runtime = self.lua_runtime_or_err()?;
        if self.status.get(mod_id) == Some(&ModStatus::Active) {
            self.unload_mod_with(mod_id, |record| match record {
                ModExtensionRecord::Action { action_id } => runtime.unregister(&action_id),
                _ => Ok(()),
            })
            .map_err(PackLifecycleError::Unload)?;
        }
        self.forget_lua_pack(mod_id);
        Ok(())
    }

    /// Re-admit the pack `mod_id` from its root at its own level and swap it
    /// in (D9). On any failure the previous version stays active and
    /// registered, unchanged. A pack that is not active is re-admitted but
    /// not activated.
    ///
    /// # Errors
    ///
    /// Why admission or activation failed.
    pub fn reload_lua_pack(&mut self, mod_id: &str) -> Result<(), PackLifecycleError> {
        let Some(old) = self.lua_packs.get(mod_id).cloned() else {
            return Err(PackLifecycleError::UnknownPack(mod_id.to_string()));
        };
        let new = lua_pack::admit(&old.root, old.level).map_err(PackLifecycleError::Refused)?;
        if new.manifest.mod_id != old.manifest.mod_id {
            return Err(PackLifecycleError::IdChanged {
                was: old.manifest.mod_id,
                now: new.manifest.mod_id,
            });
        }
        let mut records = None;
        if self.status.get(mod_id) == Some(&ModStatus::Active) {
            let runtime = self.lua_runtime_or_err()?;
            let activation = |reason: String| PackLifecycleError::Activation {
                mod_id: mod_id.to_string(),
                reason,
            };
            let ids = runtime.replace(&old, &new).map_err(activation)?;
            let verified = verify_records(mod_id, &new.manifest.contributes, action_records(ids));
            match verified {
                Ok(done) => records = Some(done),
                Err(error) => {
                    // Put the old version back before reporting.
                    let _ = runtime.replace(&new, &old);
                    let (reason, _) = error.into_parts();
                    return Err(activation(reason));
                },
            }
        }
        self.manifests
            .insert(mod_id.to_string(), new.manifest.clone());
        self.lua_packs.insert(mod_id.to_string(), new);
        if let Some(done) = records {
            self.extension_records.insert(mod_id.to_string(), done);
        }
        Ok(())
    }

    /// Reload every Lua pack, in id order. Each pack succeeds or keeps its
    /// previous version on its own.
    pub fn reload_all_lua_packs(&mut self) -> Vec<(String, Result<(), PackLifecycleError>)> {
        let mut ids: Vec<String> = self.lua_packs.keys().cloned().collect();
        ids.sort();
        ids.into_iter()
            .map(|id| {
                let result = self.reload_lua_pack(&id);
                (id, result)
            })
            .collect()
    }

    fn activate_pack(
        runtime: &dyn LuaPackRuntime,
        pack: &AdmittedPack,
    ) -> Result<Vec<ModExtensionRecord>, PackLifecycleError> {
        let mod_id = &pack.manifest.mod_id;
        let fail = |reason: String| PackLifecycleError::Activation {
            mod_id: mod_id.clone(),
            reason,
        };
        let ids = runtime.activate(pack).map_err(fail)?;
        verify_records(mod_id, &pack.manifest.contributes, action_records(ids)).map_err(|error| {
            let (reason, applied) = error.into_parts();
            for record in applied {
                if let ModExtensionRecord::Action { action_id } = record {
                    let _ = runtime.unregister(&action_id);
                }
            }
            fail(reason)
        })
    }

    fn forget_lua_pack(&mut self, mod_id: &str) {
        self.manifests.remove(mod_id);
        self.status.remove(mod_id);
        self.lua_packs.remove(mod_id);
        self.extension_records.remove(mod_id);
        self.load_order.retain(|id| id != mod_id);
    }
}
