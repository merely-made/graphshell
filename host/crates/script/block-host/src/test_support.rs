// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The fixture guest this crate builds for its tests (`guest/`, a component that claims
//! the class `fixture`), for other crates' tests to load through a dev dependency with
//! the `test-support` feature. The build script compiles it for `wasm32-wasip2`; no
//! caller names a path into this crate.

use std::path::PathBuf;

/// Where the fixture guest component is: `BLOCK_HOST_GUEST_WASM` if set (as in this
/// crate's own tests), otherwise where this crate's build script places it.
#[must_use]
pub fn fixture_guest_path() -> PathBuf {
    std::env::var_os("BLOCK_HOST_GUEST_WASM").map_or_else(
        || PathBuf::from(env!("BLOCK_HOST_FIXTURE_GUEST_WASM")),
        PathBuf::from,
    )
}

/// The fixture guest component's bytes.
///
/// # Errors
///
/// When the component is missing: the build script skips it without the `wasm32-wasip2`
/// target or with `BLOCK_HOST_SKIP_FIXTURE` set. The message names the command that
/// builds it.
pub fn fixture_guest_wasm() -> Result<Vec<u8>, String> {
    let path = fixture_guest_path();
    std::fs::read(&path).map_err(|err| {
        format!(
            "block-host's fixture guest is missing at {} ({err}). Build it with \
             `rustup target add wasm32-wasip2` and a rebuild of block-host, or by hand: \
             cd <block-host>/guest && cargo build --locked --target wasm32-wasip2 --release",
            path.display()
        )
    })
}
