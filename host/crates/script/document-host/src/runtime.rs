// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! P2.4 — the `crate::mod_loader` `WasmModRuntime` bridge.
//!
//! Implements `crate::mod_loader`'s `WasmModRuntime` DI trait **without editing
//! that crate** (it is deliberately runtime-free — the trait exists precisely so
//! Wasmtime stays host-side, §11.1). A host wires
//!
//! ```ignore
//! ModRegistry::new().with_wasm_runtime(Arc::new(DocumentScriptRuntime::new(allowed)))
//! ```
//!
//! and every manifest-described `ModType::Wasm` mod flows through the loader's
//! lifecycle into this host:
//!
//! - `activate` — host-policy capability check (refuse before instantiation), map
//!   the manifest to a [`Grant`], build the component instance under that grant,
//!   run the `activate` export, and retain the live instance keyed by `mod_id`.
//! - `deactivate` — run the `deactivate` export on the retained instance and drop
//!   it (store + instance freed).
//!
//! ## The two impedance mismatches, and how they are bridged
//!
//! - **sync trait vs async host.** The trait is sync `&self`; our `document-core`
//!   exports are invoked via `call_async` (so a future `fetch` can suspend a turn,
//!   §11.7-7). We bridge with a per-call [`pollster::block_on`] — a minimal
//!   executor with no global runtime, safe to call from any non-reentrant thread.
//!   With no async import yet, every call completes in a single poll.
//! - **`Send + Sync` trait vs the per-mod store.** The trait demands `Send + Sync`;
//!   we retain live `Store<ScriptHost>`s behind a `Mutex`. That compiles only
//!   because `ScriptHost` is `Send` (genet's `ScriptedDom` is an id-keyed arena of
//!   `HashMap`/`Vec`, its `QualName`s string-cache atoms — no `Rc`/`RefCell`). If
//!   that ever regresses, the fallback is a dedicated worker thread; for now the
//!   live store rides in the runtime directly.
//!
//! ## Capability policy (the load-bearing check)
//!
//! The `document-core` world exposes `log` + `document`; both are world-intrinsic
//! for a document mod, so [`grant_for`] grants both. The manifest's host
//! `ModCapability`s (Network / Filesystem / Identity / Clipboard / Exec) gate
//! *host* capabilities that map to future imports (Network → `fetch`); none has a
//! live import yet, so they do not change the grant. They are not inert, though:
//! a runtime is constructed with the set of capabilities its **surface** will
//! grant, and `activate` refuses — before any instantiation — a mod requesting a
//! capability outside that set. That is the host-policy enforcement point (§11.4):
//! a surface that will not grant `Network` rejects a `Network` mod up front.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use crate::mod_loader::{
    AdmissionLevel, InstallRecord, ModCapability, ModManifest, WasmModRuntime, WasmModSource,
    WasmRefusal, admit_wasm, hash_bytes,
};
use wasmtime::{Engine, Store, StoreLimits};

/// A deadline no run reaches: the runtime has no watchdog, and its engine is the
/// guarded one only so a host-built `.cwasm` is config-compatible with it.
const NO_DEADLINE: u64 = 1 << 40;

use crate::{
    BuildError, BuiltComponent, CapPermission, CheckReport, ComponentBuilder, DocumentCore, Grant,
    ScriptHost, World, build_instance, seed_dom,
};

/// One activated wasm mod: the live store + bindings, retained so `deactivate` can
/// drive the teardown export before the store (and the instance it owns) is freed.
struct ActiveMod {
    store: Store<ScriptHost>,
    bindings: DocumentCore,
}

/// The document-host's implementation of `crate::mod_loader`'s `WasmModRuntime`.
/// Owns one Wasmtime `Engine` (cheap, `Arc`-backed) and a `mod_id`-keyed table of
/// activated instances. Construct it with the set of host capabilities the surface
/// is willing to grant; `activate` enforces that set before instantiating.
pub struct DocumentScriptRuntime {
    engine: Engine,
    /// Host capability policy: which `ModCapability`s this surface will grant.
    allowed: Vec<ModCapability>,
    /// Live activated instances, keyed by `mod_id`.
    active: Mutex<HashMap<String, ActiveMod>>,
    /// Install records of admitted mods, keyed by `mod_id` (mod projects MP1).
    /// In memory: the registry persists nothing for Wasm mods yet.
    records: Mutex<HashMap<String, InstallRecord>>,
}

impl DocumentScriptRuntime {
    /// A runtime that grants exactly the `allowed` host capabilities. A mod whose
    /// manifest requests anything outside this set is refused at `activate`.
    pub fn new(allowed: Vec<ModCapability>) -> Self {
        Self {
            engine: crate::guarded_engine().expect("the host's engine configuration is valid"),
            allowed,
            active: Mutex::new(HashMap::new()),
            records: Mutex::new(HashMap::new()),
        }
    }

    /// A fully-trusted surface: every host capability granted.
    pub fn permissive() -> Self {
        Self::new(vec![
            ModCapability::Network,
            ModCapability::Filesystem,
            ModCapability::Identity,
            ModCapability::Clipboard,
            ModCapability::Exec,
        ])
    }

    /// The `mod_id`s currently active, in arbitrary order. (Test/inspection seam.)
    pub fn active_ids(&self) -> Vec<String> {
        match self.active.lock() {
            Ok(active) => active.keys().cloned().collect(),
            Err(_) => Vec::new(),
        }
    }

    /// Admit the component at `component` for `manifest` at `level` and keep its
    /// install record (D2). Conformant pins the component's hash and, unless
    /// `readmit`, refuses a component that differs from the one already pinned;
    /// dev mode pins nothing and grants nothing.
    ///
    /// # Errors
    ///
    /// The refusal, when the component is unreadable, not a component, or changed.
    pub fn admit(
        &self,
        manifest: &ModManifest,
        version: &str,
        component: &Path,
        level: AdmissionLevel,
        readmit: bool,
    ) -> Result<InstallRecord, WasmRefusal> {
        let mut records = self
            .records
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let prior = if readmit {
            None
        } else {
            records.get(&manifest.mod_id)
        };
        let record = admit_wasm(manifest, version, component, level, &self.allowed, prior)?;
        records.insert(manifest.mod_id.clone(), record.clone());
        Ok(record)
    }

    /// The install record of `mod_id`, if admitted.
    pub fn record(&self, mod_id: &str) -> Option<InstallRecord> {
        self.records.lock().ok()?.get(mod_id).cloned()
    }

    /// Precompile the admitted component of `mod_id` to a `.cwasm` at `out` and
    /// record its hash (D4). Only a conformant record builds, and only from bytes
    /// that still match its pinned hash.
    ///
    /// # Errors
    ///
    /// No conformant record, a changed component, or a compile or write failure.
    pub fn build_cwasm(
        &self,
        mod_id: &str,
        component: &Path,
        out: &Path,
    ) -> Result<InstallRecord, String> {
        let mut records = self.records.lock().map_err(|_| "records lock poisoned")?;
        let record = records
            .get_mut(mod_id)
            .ok_or_else(|| WasmRefusal::NoRecord { id: mod_id.into() }.words())?;
        if record.level != AdmissionLevel::Conformant {
            return Err(format!("'{mod_id}' is in dev mode; only an admitted component builds"));
        }
        let bytes = std::fs::read(component).map_err(|e| format!("{}: {e}", component.display()))?;
        record.verify_component(&bytes).map_err(|e| e.words())?;
        let cwasm = crate::precompile_bytes(&bytes).map_err(|e| format!("precompile: {e}"))?;
        std::fs::write(out, &cwasm).map_err(|e| format!("{}: {e}", out.display()))?;
        record.note_build(hash_bytes(&cwasm));
        Ok(record.clone())
    }

    /// The surface's grant for `manifest`: what `activate` would link. Dev mode never
    /// grants (D2).
    fn grant_of(&self, manifest: &ModManifest) -> Grant {
        let dev = self
            .record(&manifest.mod_id)
            .is_some_and(|r| r.level == AdmissionLevel::DevMode);
        grant_for(manifest, if dev { &[] } else { &self.allowed })
    }

    /// Check the component at `component` against the grant this surface gives
    /// `manifest` and against `world`, and keep the result as the record's last
    /// check (MP2, D1).
    ///
    /// # Errors
    ///
    /// The refusal, when the file is unreadable or not a component.
    pub fn check(
        &self,
        manifest: &ModManifest,
        component: &Path,
        world: &World,
    ) -> Result<CheckReport, WasmRefusal> {
        let report = crate::check(component, &self.grant_of(manifest), world)?;
        if let Ok(mut records) = self.records.lock() {
            if let Some(r) = records.get_mut(&manifest.mod_id) {
                r.note_check(report.words());
            }
        }
        Ok(report)
    }

    /// Build the project of `mod_id` with `builder` and keep the outcome as the
    /// record's last build. The component is not re-admitted; a conformant mod
    /// still needs `admit` with `readmit` to pin it (D1).
    ///
    /// # Errors
    ///
    /// The builder's reason.
    pub fn build_project(
        &self,
        mod_id: &str,
        builder: &dyn ComponentBuilder,
        project: &Path,
    ) -> Result<BuiltComponent, BuildError> {
        let result = builder.build(project);
        if let Ok(mut records) = self.records.lock() {
            if let Some(r) = records.get_mut(mod_id) {
                r.note_component_build(match &result {
                    Ok(b) => format!("built {}", b.path.display()),
                    Err(e) => e.words(),
                });
            }
        }
        result
    }

    /// Whether `mod_id` is currently active.
    pub fn is_active(&self, mod_id: &str) -> bool {
        self.active
            .lock()
            .map(|a| a.contains_key(mod_id))
            .unwrap_or(false)
    }
}

/// Map a mod to the `document-core` import grant (mod projects MP0, D2). `log` and
/// `document` are intrinsic to a document mod. Every other interface is linked only
/// when the manifest declares its capability AND the surface granted it
/// (`allowed`): `Network` -> `mere:script/net`. The other `ModCapability`s gate no
/// import yet. An unlinked import fails instantiation, naming it.
fn grant_for(manifest: &ModManifest, allowed: &[ModCapability]) -> Grant {
    let granted =
        |c: ModCapability| manifest.capabilities.contains(&c) && allowed.contains(&c);
    Grant {
        log: CapPermission::Allow,
        document: CapPermission::Allow,
        net: if granted(ModCapability::Network) {
            CapPermission::Allow
        } else {
            CapPermission::Deny
        },
    }
}

impl WasmModRuntime for DocumentScriptRuntime {
    fn activate(&self, manifest: &ModManifest, source: &WasmModSource) -> Result<(), String> {
        // 1. Host-policy capability check — before any instantiation.
        if let Some(denied) = manifest
            .capabilities
            .iter()
            .find(|c| !self.allowed.contains(c))
        {
            let import = match denied {
                ModCapability::Network => " (import mere:script/net stays unlinked)",
                _ => "",
            };
            return Err(format!(
                "mod '{}' requests capability {:?}{import} not granted by this surface",
                manifest.mod_id, denied
            ));
        }

        // 2. No double-activation (the loader should not, but be defensive).
        {
            let active = self
                .active
                .lock()
                .map_err(|_| "runtime lock poisoned".to_string())?;
            if active.contains_key(&manifest.mod_id) {
                return Err(format!("mod '{}' already active", manifest.mod_id));
            }
        }

        // 3. Build the instance under the grant + run the `activate` lifecycle
        //    export. Instantiation itself fails if the component requires an import
        //    the grant omitted (the runtime-enforced capability boundary).
        // Extension mods get a fresh seeded DOM (they are not page-attached) and are
        // unguarded for P2.4 (the page-script guards live in `DocumentScript`).
        //    An admitted mod loads against its install record (conformant: the
        //    pinned hash; dev mode: live, granting nothing); a `.cwasm` needs one.
        let record = self.record(&manifest.mod_id);
        let granted_by_surface: &[ModCapability] = match &record {
            Some(r) if r.level == AdmissionLevel::DevMode => &[],
            _ => &self.allowed,
        };
        let grant = grant_for(manifest, granted_by_surface);
        let note_trap = |why: &str| {
            if let Ok(mut records) = self.records.lock() {
                if let Some(r) = records.get_mut(&manifest.mod_id) {
                    r.note_trap(why);
                }
            }
        };
        let (mut store, bindings) = pollster::block_on(build_instance(
            &self.engine,
            &source.module_path,
            seed_dom(),
            &grant,
            StoreLimits::default(),
            Some(NO_DEADLINE),
            record.as_ref(),
        ))
        .map_err(|e| {
            note_trap(&e.to_string());
            format!("instantiate '{}': {e}", manifest.mod_id)
        })?;
        pollster::block_on(bindings.call_activate(&mut store)).map_err(|e| {
            note_trap(&e.to_string());
            format!("activate '{}': {e}", manifest.mod_id)
        })?;

        // 4. Retain the live instance for `deactivate`.
        self.active
            .lock()
            .map_err(|_| "runtime lock poisoned".to_string())?
            .insert(manifest.mod_id.clone(), ActiveMod { store, bindings });
        Ok(())
    }

    fn deactivate(&self, mod_id: &str) -> Result<(), String> {
        // Remove under the lock, then run the export with the lock released so a
        // teardown call never blocks other activations.
        let entry = {
            let mut active = self
                .active
                .lock()
                .map_err(|_| "runtime lock poisoned".to_string())?;
            active.remove(mod_id)
        };
        let ActiveMod {
            mut store,
            bindings,
        } = entry.ok_or_else(|| format!("mod '{mod_id}' not active"))?;
        pollster::block_on(bindings.call_deactivate(&mut store))
            .map_err(|e| format!("deactivate '{mod_id}': {e}"))?;
        // `store` (and the instance it owns) dropped here.
        Ok(())
    }
}
