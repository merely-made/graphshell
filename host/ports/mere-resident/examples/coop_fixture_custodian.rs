// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The co-op harness's custodian (Turnstone `support/graphshell-coop`,
//! engine-split D130): `djinn_custodian::fixture` served from this process
//! over a new synthetic passphrase vault, with the packaged Djinn grants at
//! consent `none`. A production `passphrase-fd` Djinn reaches it through the
//! client file printed here, handed to it as `--custodian`. Never an installed
//! custodian: no service manager, no confinement, no root-owned client file.
//!
//! Usage: coop_fixture_custodian <new-directory> <profile>
//! The first line of stdin is the vault passphrase (the same one Djinn reads
//! on `--passphrase-fd 0`). One JSON readiness line goes to stdout; the
//! custodian serves until stdin reaches EOF or reads `quit`.

use std::io::{BufRead, Write};
use std::path::PathBuf;

fn main() -> Result<(), String> {
    let mut args = std::env::args_os().skip(1);
    let usage = "usage: coop_fixture_custodian <new-directory> <profile>";
    let root = PathBuf::from(args.next().ok_or(usage)?);
    let profile = args
        .next()
        .ok_or(usage)?
        .into_string()
        .map_err(|_| "non-UTF-8 profile")?;
    if args.next().is_some() || !root.is_absolute() {
        return Err(usage.into());
    }
    // A new directory only: a fixture must never open an existing vault.
    std::fs::create_dir(&root).map_err(|e| format!("fixture requires a new directory: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| e.to_string())?;
    }
    let stdin = std::io::stdin();
    let mut stdin = stdin.lock();
    let mut passphrase = String::new();
    stdin
        .read_line(&mut passphrase)
        .map_err(|e| e.to_string())?;
    let passphrase = passphrase.trim_end_matches(['\r', '\n']);
    if passphrase.is_empty() {
        return Err("the first line of stdin must be the vault passphrase".into());
    }
    let grants: serde_json::Value = serde_json::from_str(
        &djinn_custodian::fixture::DJINN_GRANTS_TEMPLATE
            .replace("@CONSENT@", "none")
            .replace("@SIGN_CONSENT@", "none"),
    )
    .map_err(|e| e.to_string())?;
    let mut seed = [0u8; 32];
    seed[..16].copy_from_slice(uuid::Uuid::new_v4().as_bytes());
    seed[16..].copy_from_slice(uuid::Uuid::new_v4().as_bytes());
    let fixture = djinn_custodian::fixture::start(
        &root,
        djinn_custodian::fixture::Spec {
            passphrase,
            profiles: &[(profile.as_str(), profile.as_str(), seed)],
            grants,
            consent: "deny",
            mechanism: None,
            agent: false,
        },
    )?;
    let ready = serde_json::json!({
        "ready": true, "pid": std::process::id(), "profile": profile,
        "client_file": fixture.client_file, "custody": "fixture-custodian",
        "grants": "djinn-grants.json.in, consent none", "synthetic": true,
    });
    let mut out = std::io::stdout().lock();
    writeln!(out, "{ready}")
        .and_then(|()| out.flush())
        .map_err(|e| e.to_string())?;
    let mut line = String::new();
    while stdin.read_line(&mut line).map_err(|e| e.to_string())? > 0 && line.trim() != "quit" {
        line.clear();
    }
    Ok(())
}
