// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Admission of a Lua command pack (command packs CP1, D1 to D3).
//!
//! A pack is a directory holding `mod.toml` and the `.lua` files it names.
//! Admission is a level (graph-behaviors D3 to D5):
//!
//! - **Conformant** (the default) is Mesocosm's door: all or nothing, an ABI
//!   number, no unknown keys, every file declared, no path leaving the pack,
//!   ids qualified by the pack, and one named refusal per failure, checked in
//!   a fixed order so two machines refuse a pack with the same words.
//! - **DevMode** loads what parses, tolerates undeclared files, skips
//!   duplicates and unusable commands, and lists in `diagnostics` what
//!   conformant would have refused. It never changes `grants`.
//!
//! Paths are compared as `Path` components and canonical forms, never as
//! strings with a separator, so `a\b.lua` and `a/b.lua` are one file.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use serde::Deserialize;

use super::contributes::Contribution;
use super::types::{Grants, ModManifest, ModType};

/// The manifest's file name at the pack root.
pub const LUA_PACK_MANIFEST: &str = "mod.toml";

/// The pack format this build reads.
pub const LUA_PACK_ABI: u32 = 1;

const TOP_KEYS: &[&str] = &[
    "abi",
    "mod_id",
    "display_name",
    "mod_type",
    "files",
    "grants",
    "contributes",
];
const GRANT_KEYS: &[&str] = &["classes", "commands"];
const COMMAND_KEYS: &[&str] = &["kind", "id", "label", "effect", "source"];

/// How strictly a pack is admitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionLevel {
    Conformant,
    DevMode,
}

/// Why a pack, or one of its commands, would not be admitted conformant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackRefusal {
    NoManifest {
        root: String,
    },
    Unreadable {
        path: String,
        why: String,
    },
    /// Not TOML, or not the shape the schema declares. Refused at both levels.
    MalformedSchema {
        path: String,
        why: String,
    },
    /// A key the schema does not name, as a dotted path (`grants.extra`).
    UnknownField {
        key: String,
    },
    /// `mod_type` is not `"lua"`.
    NotLuaPack {
        found: String,
    },
    UnknownAbi {
        found: u32,
        supported: u32,
    },
    /// A `contributes` entry that is not a `command`.
    ForeignContribution {
        id: String,
    },
    /// A command id that is not `<mod_id>:<name>`.
    UnqualifiedId {
        id: String,
        expected_prefix: String,
    },
    DuplicateId {
        id: String,
    },
    /// A declared path leaves the pack root. Refused before it is opened.
    PathEscape {
        declared: String,
    },
    /// A command source that is not a `.lua` file.
    NotLuaSource {
        path: String,
    },
    /// A `.lua` or `.toml` file in the pack that the manifest never declared.
    UndeclaredFile {
        path: String,
    },
    /// The pack declares no commands.
    EmptyPack {
        root: String,
    },
    /// Registry level: another mod already holds this id.
    DuplicateModId {
        mod_id: String,
    },
}

impl PackRefusal {
    /// The refusal as the plain sentence a diagnostic prints.
    pub fn words(&self) -> String {
        match self {
            Self::NoManifest { root } => format!("no {LUA_PACK_MANIFEST} in {root}"),
            Self::Unreadable { path, why } => format!("{path} could not be read: {why}"),
            Self::MalformedSchema { path, why } => format!("{path} is malformed: {why}"),
            Self::UnknownField { key } => format!("the manifest has an unknown key: {key}"),
            Self::NotLuaPack { found } => {
                format!("mod_type is \"{found}\", and a command pack is \"lua\"")
            },
            Self::UnknownAbi { found, supported } => {
                format!("pack format {found}, and this build reads {supported}")
            },
            Self::ForeignContribution { id } => {
                format!("{id} is not a command, and a pack contributes commands only")
            },
            Self::UnqualifiedId {
                id,
                expected_prefix,
            } => format!("{id} is not qualified by its pack: expected {expected_prefix}<name>"),
            Self::DuplicateId { id } => format!("two commands claim {id}"),
            Self::PathEscape { declared } => format!("{declared} leaves the pack root"),
            Self::NotLuaSource { path } => format!("{path} is not a .lua file"),
            Self::UndeclaredFile { path } => {
                format!("{path} is in the pack and the manifest does not declare it")
            },
            Self::EmptyPack { root } => format!("{root} declares no commands"),
            Self::DuplicateModId { mod_id } => format!("a mod with id {mod_id} is already known"),
        }
    }
}

/// A pack that passed admission at `level`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedPack {
    pub manifest: ModManifest,
    pub root: PathBuf,
    pub level: AdmissionLevel,
    /// What conformant admission would have refused. Always empty when
    /// `level` is `Conformant`.
    pub diagnostics: Vec<PackRefusal>,
}

#[derive(Debug, Deserialize)]
struct RawPack {
    abi: u32,
    mod_id: String,
    display_name: Option<String>,
    mod_type: String,
    #[serde(default)]
    files: Vec<String>,
    #[serde(default)]
    grants: Grants,
    #[serde(default)]
    contributes: Vec<Contribution>,
}

/// Admit the pack at `root`.
///
/// # Errors
///
/// `Err` is the first refusal in check order, and
/// at `DevMode` only a pack that cannot be read at all (no manifest, not
/// TOML, wrong types).
pub fn admit(root: &Path, level: AdmissionLevel) -> Result<AdmittedPack, PackRefusal> {
    let manifest_path = root.join(LUA_PACK_MANIFEST);
    if !manifest_path.is_file() {
        return Err(PackRefusal::NoManifest {
            root: root.display().to_string(),
        });
    }
    let text =
        std::fs::read_to_string(&manifest_path).map_err(|error| PackRefusal::Unreadable {
            path: LUA_PACK_MANIFEST.to_string(),
            why: error.to_string(),
        })?;
    let value: toml::Value =
        toml::from_str(&text).map_err(|error| PackRefusal::MalformedSchema {
            path: LUA_PACK_MANIFEST.to_string(),
            why: error.to_string(),
        })?;
    let mut found: Vec<PackRefusal> = unknown_keys(&value)
        .into_iter()
        .map(|key| PackRefusal::UnknownField { key })
        .collect();
    let raw: RawPack =
        value
            .try_into()
            .map_err(|error: toml::de::Error| PackRefusal::MalformedSchema {
                path: LUA_PACK_MANIFEST.to_string(),
                why: error.to_string(),
            })?;

    if raw.mod_type != "lua" {
        found.push(PackRefusal::NotLuaPack {
            found: raw.mod_type.clone(),
        });
    }
    if raw.abi != LUA_PACK_ABI {
        found.push(PackRefusal::UnknownAbi {
            found: raw.abi,
            supported: LUA_PACK_ABI,
        });
    }

    let mut declared: BTreeSet<PathBuf> = BTreeSet::new();
    let loaded = check_commands(root, &raw, &mut declared, &mut found);
    for relative in &raw.files {
        match declared_file(root, relative) {
            Ok(path) => {
                declared.insert(canonical(&path));
            },
            Err(refusal) => found.push(refusal),
        }
    }
    found.extend(undeclared(root, &declared));
    if raw.contributes.is_empty() {
        found.push(PackRefusal::EmptyPack {
            root: root.display().to_string(),
        });
    }

    let diagnostics = match level {
        AdmissionLevel::Conformant => {
            if let Some(first) = found.into_iter().next() {
                return Err(first);
            }
            Vec::new()
        },
        AdmissionLevel::DevMode => found,
    };
    let display_name = raw.display_name.unwrap_or_else(|| raw.mod_id.clone());
    let mut manifest = ModManifest::new(
        raw.mod_id,
        display_name,
        ModType::Lua,
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )
    .with_contributes(loaded);
    manifest.grants = raw.grants;
    Ok(AdmittedPack {
        manifest,
        root: root.to_path_buf(),
        level,
        diagnostics,
    })
}

/// Check each contribution, record what conformant would refuse in `found`
/// and the files it declares in `declared`, and return the commands that can
/// load (dev mode keeps these; a duplicate or unusable one is dropped).
fn check_commands(
    root: &Path,
    raw: &RawPack,
    declared: &mut BTreeSet<PathBuf>,
    found: &mut Vec<PackRefusal>,
) -> Vec<Contribution> {
    let prefix = format!("{}:", raw.mod_id);
    let mut loaded: Vec<Contribution> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for contribution in &raw.contributes {
        let Contribution::Command { id, source, .. } = contribution else {
            found.push(PackRefusal::ForeignContribution {
                id: contribution.id().to_string(),
            });
            continue;
        };
        let mut usable = true;
        if id.strip_prefix(&prefix).is_none_or(str::is_empty) {
            found.push(PackRefusal::UnqualifiedId {
                id: id.clone(),
                expected_prefix: prefix.clone(),
            });
        }
        if !seen.insert(id.clone()) {
            found.push(PackRefusal::DuplicateId { id: id.clone() });
            usable = false;
        }
        match declared_file(root, source) {
            Ok(path) => {
                declared.insert(canonical(&path));
                if !has_extension(&path, "lua") {
                    found.push(PackRefusal::NotLuaSource {
                        path: source.clone(),
                    });
                }
            },
            Err(refusal) => {
                found.push(refusal);
                usable = false;
            },
        }
        if usable {
            loaded.push(contribution.clone());
        }
    }
    loaded
}

/// Keys the schema does not name, as dotted paths, in a fixed order.
fn unknown_keys(value: &toml::Value) -> Vec<String> {
    let mut unknown = Vec::new();
    let Some(top) = value.as_table() else {
        return unknown;
    };
    let strays = |table: &toml::map::Map<String, toml::Value>, known: &[&str], at: &str| {
        let mut keys: Vec<String> = table
            .keys()
            .filter(|key| !known.contains(&key.as_str()))
            .map(|key| {
                if at.is_empty() {
                    key.clone()
                } else {
                    format!("{at}.{key}")
                }
            })
            .collect();
        keys.sort();
        keys
    };
    unknown.extend(strays(top, TOP_KEYS, ""));
    if let Some(grants) = top.get("grants").and_then(toml::Value::as_table) {
        unknown.extend(strays(grants, GRANT_KEYS, "grants"));
    }
    let entries = top.get("contributes").and_then(toml::Value::as_array);
    for (index, entry) in entries.into_iter().flatten().enumerate() {
        let Some(table) = entry.as_table() else {
            continue;
        };
        if table.get("kind").and_then(toml::Value::as_str) == Some("command") {
            unknown.extend(strays(
                table,
                COMMAND_KEYS,
                &format!("contributes[{index}]"),
            ));
        }
    }
    unknown
}

/// A declared path, resolved inside `root`, that exists as a file.
fn declared_file(root: &Path, declared: &str) -> Result<PathBuf, PackRefusal> {
    let path = inside(root, declared)?;
    match std::fs::metadata(&path) {
        Ok(meta) if meta.is_file() => Ok(path),
        Ok(_) => Err(PackRefusal::Unreadable {
            path: declared.to_string(),
            why: "not a file".to_string(),
        }),
        Err(error) => Err(PackRefusal::Unreadable {
            path: declared.to_string(),
            why: error.to_string(),
        }),
    }
}

/// Resolve `declared` under `root`, refusing a path that leaves it. The
/// component walk refuses `..`, a root and a drive prefix without touching
/// the disk; the canonical comparison then catches a link out.
fn inside(root: &Path, declared: &str) -> Result<PathBuf, PackRefusal> {
    let escape = || PackRefusal::PathEscape {
        declared: declared.to_string(),
    };
    let relative = Path::new(declared);
    let mut normal = false;
    for component in relative.components() {
        match component {
            Component::Normal(_) => normal = true,
            Component::CurDir => {},
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(escape());
            },
        }
    }
    if !normal {
        return Err(escape());
    }
    let joined = root.join(relative);
    match (joined.canonicalize(), root.canonicalize()) {
        (Ok(resolved), Ok(base)) if !resolved.starts_with(&base) => Err(escape()),
        _ => Ok(joined),
    }
}

fn canonical(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn has_extension(path: &Path, wanted: &str) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case(wanted))
}

/// `.lua` and `.toml` files under `root`, other than the manifest, that the
/// manifest did not declare, in sorted order.
fn undeclared(root: &Path, declared: &BTreeSet<PathBuf>) -> Vec<PackRefusal> {
    let mut files = Vec::new();
    walk(root, &mut files);
    let manifest = canonical(&root.join(LUA_PACK_MANIFEST));
    let mut stray: Vec<String> = files
        .into_iter()
        .filter(|path| {
            let resolved = canonical(path);
            resolved != manifest && !declared.contains(&resolved)
        })
        .map(|path| portable(path.strip_prefix(root).unwrap_or(&path)))
        .collect();
    stray.sort();
    stray
        .into_iter()
        .map(|path| PackRefusal::UndeclaredFile { path })
        .collect()
}

fn walk(dir: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, found);
        } else if has_extension(&path, "lua") || has_extension(&path, "toml") {
            found.push(path);
        }
    }
}

/// A relative path with `/` between its components on every platform.
fn portable(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}
