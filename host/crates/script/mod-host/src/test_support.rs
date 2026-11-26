// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The fixture guests this crate builds for its tests (`guest-parser/`, `guest-viewer/`),
//! for other crates' tests to load through a dev dependency with the `test-support`
//! feature. The build script compiles them for `wasm32-wasip2`.

use std::path::PathBuf;

fn path(var: &str, built: &str) -> PathBuf {
    std::env::var_os(var).map_or_else(|| PathBuf::from(built), PathBuf::from)
}

/// The parser fixture guest: `MOD_HOST_PARSER_WASM` if set, else where the build script
/// places it. It parses `application/x-fixture-notes` and misbehaves on the other
/// `application/x-fixture-*` types (see its source).
#[must_use]
pub fn parser_guest_path() -> PathBuf {
    path("MOD_HOST_PARSER_WASM", env!("MOD_HOST_PARSER_FIXTURE_WASM"))
}

/// The viewer fixture guest: `MOD_HOST_VIEWER_WASM` if set, else where the build script
/// places it.
#[must_use]
pub fn viewer_guest_path() -> PathBuf {
    path("MOD_HOST_VIEWER_WASM", env!("MOD_HOST_VIEWER_FIXTURE_WASM"))
}

fn read(path: PathBuf, what: &str) -> Result<Vec<u8>, String> {
    std::fs::read(&path).map_err(|err| {
        format!(
            "mod-host's {what} fixture guest is missing at {} ({err}). Build it with \
             `rustup target add wasm32-wasip2` and a rebuild of mod-host, or by hand: \
             cd <mod-host>/guest-{what} && cargo build --target wasm32-wasip2 --release",
            path.display()
        )
    })
}

/// The parser fixture guest's bytes.
///
/// # Errors
///
/// The component is missing; the message names the command that builds it.
pub fn parser_guest_wasm() -> Result<Vec<u8>, String> {
    read(parser_guest_path(), "parser")
}

/// The viewer fixture guest's bytes.
///
/// # Errors
///
/// The component is missing; the message names the command that builds it.
pub fn viewer_guest_wasm() -> Result<Vec<u8>, String> {
    read(viewer_guest_path(), "viewer")
}
