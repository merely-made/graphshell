// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The host's `check` of a component (mod projects MP2, D1): load it, compare its
//! imports with the grant and its exports with the world, and report each mismatch
//! by name. Nothing is instantiated and nothing runs.

use std::path::Path;

use wasmtime::Engine;
use wasmtime::component::Component;

use crate::Grant;
use crate::mod_loader::WasmRefusal;

/// What a component must export to be of a world.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct World {
    pub name: String,
    /// Export names the world requires, without version suffixes.
    pub exports: Vec<String>,
}

impl World {
    /// The `document-core` world: `activate`, `handle-event`, `deactivate`.
    pub fn document_core() -> Self {
        Self {
            name: "document-core".into(),
            exports: ["activate", "handle-event", "deactivate"]
                .map(String::from)
                .into(),
        }
    }

    /// This world plus one more required export.
    #[must_use]
    pub fn requiring(mut self, export: impl Into<String>) -> Self {
        self.exports.push(export.into());
        self
    }
}

/// What `check` found, every mismatch named.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CheckReport {
    /// Every import the component declares, versions stripped.
    pub imports: Vec<String>,
    /// Every export the component declares, versions stripped.
    pub exports: Vec<String>,
    /// Application imports the grant does not link.
    pub ungranted_imports: Vec<String>,
    /// Imports the host offers nowhere.
    pub unknown_imports: Vec<String>,
    /// Exports the world requires and the component lacks.
    pub missing_exports: Vec<String>,
}

impl CheckReport {
    /// Whether the component matches its grants and its world.
    pub fn ok(&self) -> bool {
        self.ungranted_imports.is_empty()
            && self.unknown_imports.is_empty()
            && self.missing_exports.is_empty()
    }

    /// The report in plain words, one clause per mismatch.
    pub fn words(&self) -> String {
        if self.ok() {
            return "check passed".into();
        }
        let mut parts = Vec::new();
        parts.extend(
            self.ungranted_imports
                .iter()
                .map(|i| format!("import {i} is not granted")),
        );
        parts.extend(
            self.unknown_imports
                .iter()
                .map(|i| format!("import {i} is not offered by the host")),
        );
        parts.extend(
            self.missing_exports
                .iter()
                .map(|e| format!("export {e} is missing")),
        );
        format!("check failed: {}", parts.join("; "))
    }
}

/// `mere:script/net@0.1.0` -> `mere:script/net`.
fn unversioned(name: &str) -> &str {
    name.split_once('@').map_or(name, |(base, _)| base)
}

/// The `mere:script` interfaces that carry types only: nothing to grant.
const TYPE_INTERFACES: [&str; 1] = ["mere:script/document"];

/// `caps` reports the grant; it is not a capability (§11.4), so it is always linked.
const ALWAYS_LINKED: [&str; 1] = ["mere:script/caps"];

/// Compare the component at `path` with `grant` and `world`.
///
/// The WASI floor (`wasi:*`) and the always-linked `caps` need no grant; every other
/// `mere:script` capability import must be in the grant's names.
///
/// # Errors
///
/// `Unreadable` when the file cannot be read, `NotAComponent` when it does not load.
pub fn check(path: &Path, grant: &Grant, world: &World) -> Result<CheckReport, WasmRefusal> {
    let bytes = std::fs::read(path).map_err(|e| WasmRefusal::Unreadable {
        path: path.display().to_string(),
        why: e.to_string(),
    })?;
    let engine = Engine::new(&crate::engine_config()).map_err(|e| WasmRefusal::Unreadable {
        path: path.display().to_string(),
        why: e.to_string(),
    })?;
    let component = Component::new(&engine, &bytes).map_err(|_| WasmRefusal::NotAComponent {
        path: path.display().to_string(),
    })?;
    let ty = component.component_type();
    let granted = grant.granted_names();
    let mut report = CheckReport::default();
    for (name, _) in ty.imports(&engine) {
        let name = unversioned(name).to_string();
        let offered = name.starts_with("wasi:")
            || ALWAYS_LINKED.contains(&name.as_str())
            || TYPE_INTERFACES.contains(&name.as_str());
        if !(offered || granted.contains(&name)) {
            if name.starts_with("mere:script/") {
                report.ungranted_imports.push(name.clone());
            } else {
                report.unknown_imports.push(name.clone());
            }
        }
        report.imports.push(name);
    }
    report.exports = ty
        .exports(&engine)
        .map(|(name, _)| unversioned(name).to_string())
        .collect();
    report.missing_exports = world
        .exports
        .iter()
        .filter(|e| !report.exports.contains(e))
        .cloned()
        .collect();
    Ok(report)
}
