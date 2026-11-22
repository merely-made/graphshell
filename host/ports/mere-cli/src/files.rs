// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The small JSON files passed between owner and reader: a contact, an
//! invitation with the card that reaches its owner, and a revocation.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use chirograph::MAX_QUERY_TEXT_BYTES;
use graphshell_host::invitation::{Card, Invitation};
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;

use crate::identity::TERMS;

/// Handoff files are small; anything larger is not one of ours.
const MAX_HANDOFF: usize = 64 * 1024;

/// Which carrier an invitation's card reaches its owner over.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Via {
    Iroh,
    I2p,
    Reticulum,
}

/// What an owner hands a reader: the grant, and where to present it.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InviteFile {
    pub version: u16,
    pub via: Via,
    pub invitation: Invitation,
    /// The owner's contact and its carrier ticket: an iroh ticket, an I2P peer
    /// ticket, or a Reticulum TCP address.
    pub card: Card,
}

impl InviteFile {
    pub fn check(&self) -> Result<()> {
        ensure!(self.version == 1, "unsupported invitation file version");
        self.card
            .check(&TERMS, &self.invitation)
            .map_err(anyhow::Error::msg)?;
        ensure!(self.invitation.query.is_some(), "this invitation grants no query");
        Ok(())
    }
}

pub fn read_small(path: &Path, limit: usize) -> Result<Vec<u8>> {
    let mut data = Vec::new();
    std::fs::File::open(path)
        .with_context(|| format!("open {}", path.display()))?
        .take(limit as u64 + 1)
        .read_to_end(&mut data)?;
    ensure!(data.len() <= limit, "{} exceeds {limit} bytes", path.display());
    Ok(data)
}

/// A UTF-8 query file, line endings normalised so a grant made on one system
/// matches the same file read on another.
pub fn read_query(path: &Path) -> Result<String> {
    let bytes = read_small(path, MAX_QUERY_TEXT_BYTES as usize)?;
    let text = String::from_utf8(bytes).context("the query must be UTF-8")?;
    ensure!(!text.trim().is_empty(), "the query is empty");
    Ok(text.replace("\r\n", "\n"))
}

pub fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    serde_json::from_slice(&read_small(path, MAX_HANDOFF)?)
        .with_context(|| format!("{} is not the expected file", path.display()))
}

/// Write a new file, never replacing one: a handoff file is private and its
/// name is the owner's choice.
pub fn write_new(path: &Path, value: &impl Serialize) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| format!("create {} (choose a new file name)", path.display()))?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    Ok(())
}

/// Where a serve looks for revocations of the invitation written at `invite`,
/// unless told otherwise (D17).
pub fn revocations_beside(invite: &Path) -> PathBuf {
    let mut name = invite.file_name().unwrap_or_default().to_os_string();
    name.push(".revocations");
    invite.with_file_name(name)
}
