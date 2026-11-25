// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The optional build adapter (mod projects MP2, D1): the host never builds; an
//! external toolchain does, when one is present. Without it a project uses
//! prebuilt components.

use std::path::{Path, PathBuf};
use std::process::Command;

/// A component a builder produced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuiltComponent {
    pub path: PathBuf,
    /// The toolchain's own output, for the run log.
    pub log: String,
}

/// Why a build produced no component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BuildError {
    /// The toolchain is not there to run.
    ToolchainAbsent { tool: String, why: String },
    /// The toolchain runs but cannot build for the component target.
    TargetMissing { target: String },
    /// The project has no usable manifest.
    BadProject(String),
    /// The build ran and failed.
    Failed { status: String, stderr: String },
    /// The build succeeded but left no component where one was expected.
    NoArtifact { expected: String },
}

impl BuildError {
    pub fn words(&self) -> String {
        match self {
            Self::ToolchainAbsent { tool, why } => {
                format!("no build toolchain: {tool} could not be run ({why}); use a prebuilt component")
            },
            Self::TargetMissing { target } => {
                format!("the {target} target is not installed; add it or use a prebuilt component")
            },
            Self::BadProject(why) => format!("not a buildable project: {why}"),
            Self::Failed { status, stderr } => format!("build failed ({status}): {stderr}"),
            Self::NoArtifact { expected } => {
                format!("build reported success but {expected} is missing")
            },
        }
    }
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.words())
    }
}

impl std::error::Error for BuildError {}

/// Turns a project directory into a component. The only implementation here shells
/// out to cargo; another toolchain is another implementation.
pub trait ComponentBuilder {
    /// # Errors
    ///
    /// The reason no component was produced.
    fn build(&self, project: &Path) -> Result<BuiltComponent, BuildError>;
}

/// `cargo build --target wasm32-wasip2 --release` in the project directory.
#[derive(Clone, Debug)]
pub struct CargoBuilder {
    cargo: PathBuf,
    target_dir: Option<PathBuf>,
}

const TARGET: &str = "wasm32-wasip2";

impl Default for CargoBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl CargoBuilder {
    /// The `cargo` found on `PATH`.
    pub fn new() -> Self {
        Self {
            cargo: PathBuf::from("cargo"),
            target_dir: None,
        }
    }

    /// Use this cargo executable instead (also how a test says cargo is missing).
    #[must_use]
    pub fn with_cargo(mut self, cargo: impl Into<PathBuf>) -> Self {
        self.cargo = cargo.into();
        self
    }

    /// Build into this directory instead of `<project>/target`.
    #[must_use]
    pub fn with_target_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.target_dir = Some(dir.into());
        self
    }
}

/// The artifact stem: `[lib] name`, else the package name, dashes as underscores.
fn artifact_stem(project: &Path) -> Result<String, BuildError> {
    let manifest = project.join("Cargo.toml");
    let text = std::fs::read_to_string(&manifest)
        .map_err(|e| BuildError::BadProject(format!("{}: {e}", manifest.display())))?;
    let doc: toml::Table = text
        .parse()
        .map_err(|e| BuildError::BadProject(format!("{}: {e}", manifest.display())))?;
    let name = doc
        .get("lib")
        .and_then(|l| l.get("name"))
        .or_else(|| doc.get("package").and_then(|p| p.get("name")))
        .and_then(toml::Value::as_str)
        .ok_or_else(|| BuildError::BadProject("no package name".into()))?;
    Ok(name.replace('-', "_"))
}

impl ComponentBuilder for CargoBuilder {
    fn build(&self, project: &Path) -> Result<BuiltComponent, BuildError> {
        let stem = artifact_stem(project)?;
        let absent = |why: String| BuildError::ToolchainAbsent {
            tool: self.cargo.display().to_string(),
            why,
        };
        Command::new(&self.cargo)
            .arg("--version")
            .output()
            .map_err(|e| absent(e.to_string()))?;
        let mut cmd = Command::new(&self.cargo);
        cmd.current_dir(project)
            .args(["build", "--target", TARGET, "--release"]);
        if let Some(dir) = &self.target_dir {
            cmd.arg("--target-dir").arg(dir);
        }
        // The project resolves its own toolchain: an embedding cargo's variables
        // must not leak into it (as the fixture guests' build script does).
        for (key, _) in std::env::vars() {
            if key == "CARGO_HOME" || key == "CARGO_BUILD_JOBS" {
                continue;
            }
            if key.starts_with("CARGO")
                || key.starts_with("RUSTC")
                || key.starts_with("RUSTUP")
                || key == "RUSTFLAGS"
                || key == "RUSTDOCFLAGS"
            {
                cmd.env_remove(key);
            }
        }
        let out = cmd.output().map_err(|e| absent(e.to_string()))?;
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        if !out.status.success() {
            let target_gap = stderr.contains("can't find crate for `core`")
                || stderr.contains("can't find crate for `std`")
                || stderr.contains("toolchain") && stderr.contains("does not support target");
            return Err(if target_gap {
                BuildError::TargetMissing {
                    target: TARGET.into(),
                }
            } else {
                BuildError::Failed {
                    status: out.status.to_string(),
                    stderr,
                }
            });
        }
        let base = self
            .target_dir
            .clone()
            .unwrap_or_else(|| project.join("target"));
        let path = base.join(TARGET).join("release").join(format!("{stem}.wasm"));
        if !path.exists() {
            return Err(BuildError::NoArtifact {
                expected: path.display().to_string(),
            });
        }
        Ok(BuiltComponent { path, log: stderr })
    }
}
