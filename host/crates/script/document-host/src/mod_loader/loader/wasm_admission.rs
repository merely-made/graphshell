// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Admission of a Wasm component and its install record (mod projects MP1,
//! D2, D4, D7). No Wasmtime here: bytes, hashes and the record.
//!
//! - **Conformant** pins the component's hash at admission and refuses a
//!   changed component until it is re-admitted. Its grant is the manifest's
//!   declared capabilities the surface allowed.
//! - **DevMode** reads the component live, pins nothing and grants nothing.
//!
//! Hashes are BLAKE3 (the workspace's content hash), written `blake3:<hex>`.
//! There is no signing key field yet (D7).

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::free_fns::component_preamble_ok;
use super::lua_pack::AdmissionLevel;
use super::types::{ModCapability, ModManifest};
use crate::{OriginRefusal, validate_origin};

/// The BLAKE3 hash of `bytes`, as `blake3:<hex>`.
pub fn hash_bytes(bytes: &[u8]) -> String {
    format!("blake3:{}", blake3::hash(bytes).to_hex())
}

/// A capability's name as the manifest spells it.
pub fn capability_name(capability: ModCapability) -> &'static str {
    match capability {
        ModCapability::Network => "network",
        ModCapability::Filesystem => "filesystem",
        ModCapability::Identity => "identity",
        ModCapability::Clipboard => "clipboard",
        ModCapability::Exec => "exec",
    }
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// When something last happened to an install, and what.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordEvent {
    pub at_unix: u64,
    pub note: String,
}

impl RecordEvent {
    pub fn now(note: impl Into<String>) -> Self {
        Self {
            at_unix: now_unix(),
            note: note.into(),
        }
    }
}

/// What the host knows about one admitted Wasm mod (D7: hashes, no key).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallRecord {
    pub id: String,
    pub version: String,
    pub level: AdmissionLevel,
    /// Hash of the admitted `.wasm` component. In dev mode it tracks the last
    /// component seen and nothing is checked against it.
    pub source_hash: String,
    /// Hash of the `.cwasm` the host built from the admitted component.
    pub build_hash: Option<String>,
    pub declared: Vec<String>,
    pub granted: Vec<String>,
    /// `provides` names plus `document-script:<origin>` bindings.
    pub provided: Vec<String>,
    pub last_check: Option<RecordEvent>,
    pub last_build: Option<RecordEvent>,
    pub last_trap: Option<RecordEvent>,
}

impl InstallRecord {
    /// # Errors
    ///
    /// The serializer's message.
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| e.to_string())
    }

    /// # Errors
    ///
    /// The parser's message.
    pub fn from_json(text: &str) -> Result<Self, String> {
        serde_json::from_str(text).map_err(|e| e.to_string())
    }

    /// Check `bytes` against the pinned component hash. Dev mode pins nothing.
    ///
    /// # Errors
    ///
    /// `ComponentChanged` when the hash differs.
    pub fn verify_component(&self, bytes: &[u8]) -> Result<(), WasmRefusal> {
        if self.level == AdmissionLevel::DevMode {
            return Ok(());
        }
        let found = hash_bytes(bytes);
        if found == self.source_hash {
            Ok(())
        } else {
            Err(WasmRefusal::ComponentChanged {
                id: self.id.clone(),
                pinned: self.source_hash.clone(),
                found,
            })
        }
    }

    /// Whether `.cwasm` bytes are the build the host recorded.
    ///
    /// # Errors
    ///
    /// `ForeignCwasm` unless they hash to the recorded build of a conformant record.
    pub fn verify_cwasm(&self, bytes: &[u8]) -> Result<(), WasmRefusal> {
        let found = hash_bytes(bytes);
        match (&self.level, &self.build_hash) {
            (AdmissionLevel::Conformant, Some(recorded)) if *recorded == found => Ok(()),
            _ => Err(WasmRefusal::ForeignCwasm {
                id: self.id.clone(),
                recorded: self.build_hash.clone(),
                found,
            }),
        }
    }

    pub fn note_trap(&mut self, why: impl Into<String>) {
        self.last_trap = Some(RecordEvent::now(why));
    }

    /// Record the outcome of `check` (D1) as the last check.
    pub fn note_check(&mut self, outcome: impl Into<String>) {
        self.last_check = Some(RecordEvent::now(outcome));
    }

    /// Record the outcome of a toolchain build of the component (D1), a success or
    /// the plain reason it did not happen. Leaves `build_hash`, the `.cwasm`, alone.
    pub fn note_component_build(&mut self, outcome: impl Into<String>) {
        self.last_build = Some(RecordEvent::now(outcome));
    }

    pub fn note_build(&mut self, build_hash: String) {
        self.last_build = Some(RecordEvent::now(build_hash.clone()));
        self.build_hash = Some(build_hash);
    }
}

/// Why a Wasm component, or a `.cwasm`, was not admitted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WasmRefusal {
    Unreadable {
        path: String,
        why: String,
    },
    NotAComponent {
        path: String,
    },
    /// The component differs from the one pinned; re-admit it to accept.
    ComponentChanged {
        id: String,
        pinned: String,
        found: String,
    },
    /// A `.cwasm` the host did not build (or a dev-mode record).
    ForeignCwasm {
        id: String,
        recorded: Option<String>,
        found: String,
    },
    /// A `.cwasm` for a mod with no install record.
    NoRecord {
        id: String,
    },
    /// A `document_script_origins` entry broader than one registrable domain (D8).
    BadOrigin {
        id: String,
        refusal: OriginRefusal,
    },
}

impl WasmRefusal {
    pub fn words(&self) -> String {
        match self {
            Self::Unreadable { path, why } => format!("{path} could not be read: {why}"),
            Self::NotAComponent { path } => format!("{path} is not a Wasm component"),
            Self::ComponentChanged { id, pinned, found } => format!(
                "component of '{id}' changed since admission (pinned {pinned}, found {found}); re-admit it"
            ),
            Self::ForeignCwasm {
                id,
                recorded,
                found,
            } => format!(
                "cwasm for '{id}' was not built by this host (hash {found}, recorded {})",
                recorded.as_deref().unwrap_or("none")
            ),
            Self::NoRecord { id } => format!("no install record for '{id}'; a cwasm needs one"),
            Self::BadOrigin { id, refusal } => format!("'{id}': {}", refusal.words()),
        }
    }
}

impl std::fmt::Display for WasmRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.words())
    }
}

impl std::error::Error for WasmRefusal {}

/// Admit the component at `component` for `manifest` at `level`.
///
/// `allowed` is what the surface will grant. `prior` is the mod's current
/// record: a conformant component that differs from the one it pins is refused
/// (`ComponentChanged`) until the caller re-admits with `prior: None`.
///
/// # Errors
///
/// `Err` when the file cannot be read, is not a component, or changed.
pub fn admit_wasm(
    manifest: &ModManifest,
    version: &str,
    component: &Path,
    level: AdmissionLevel,
    allowed: &[ModCapability],
    prior: Option<&InstallRecord>,
) -> Result<InstallRecord, WasmRefusal> {
    for origin in &manifest.document_script_origins {
        validate_origin(origin).map_err(|refusal| WasmRefusal::BadOrigin {
            id: manifest.mod_id.clone(),
            refusal,
        })?;
    }
    let bytes = std::fs::read(component).map_err(|e| WasmRefusal::Unreadable {
        path: component.display().to_string(),
        why: e.to_string(),
    })?;
    if !component_preamble_ok(&bytes) {
        return Err(WasmRefusal::NotAComponent {
            path: component.display().to_string(),
        });
    }
    let hash = hash_bytes(&bytes);
    let mut kept_build = None;
    if let Some(p) = prior {
        if level == AdmissionLevel::Conformant && p.level == AdmissionLevel::Conformant {
            p.verify_component(&bytes)?;
            kept_build = p.build_hash.clone();
        }
    }
    let names = |caps: Vec<ModCapability>| -> Vec<String> {
        caps.into_iter()
            .map(|c| capability_name(c).to_string())
            .collect()
    };
    let declared = names(manifest.capabilities.clone());
    // Dev mode never grants (D2).
    let granted = match level {
        AdmissionLevel::Conformant => names(
            manifest
                .capabilities
                .iter()
                .copied()
                .filter(|c| allowed.contains(c))
                .collect(),
        ),
        AdmissionLevel::DevMode => Vec::new(),
    };
    let mut provided = manifest.provides.clone();
    provided.extend(
        manifest
            .document_script_origins
            .iter()
            .map(|o| format!("document-script:{o}")),
    );
    Ok(InstallRecord {
        id: manifest.mod_id.clone(),
        version: version.to_string(),
        level,
        source_hash: hash,
        build_hash: kept_build,
        declared,
        granted,
        provided,
        last_check: Some(RecordEvent::now("admitted")),
        last_build: None,
        last_trap: None,
    })
}
