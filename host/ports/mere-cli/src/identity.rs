// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Who this CLI speaks as: the custodian's persona, connected as app
//! `mere-cli` (graphshell:mere-cli D9, D15), or a seed file in a
//! `test-identity` build (D13).

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use graphshell_host::invitation::{InvitationTerms, Signer};
use ostiary::declaration::{
    AppDeclaration, CustodyOperation, CustodyUse, NamePattern, SaltPattern, SaltUse,
};

/// The label the custodian admits this CLI under.
pub const APP: &str = "mere-cli";
/// The delegation scope its invitations are signed under.
pub const DOMAIN: &str = "mere.query";

/// What a CLI invitation names: one query below one serve session (D7).
pub const TERMS: InvitationTerms = InvitationTerms {
    domain: DOMAIN,
    salt: b"mere-cli/endpoint-session/v1",
    network_label: b"mere-cli/v1",
    path_root: "/services/projection/mere-query",
};

/// The CLI's declaration to the custodian: its transport key salt and the one
/// delegation scope it signs, nothing of Graphshell's (D15).
pub fn declaration() -> AppDeclaration {
    use CustodyOperation as Op;
    AppDeclaration {
        routes: vec![NamePattern::exact("custody"), NamePattern::exact("identity")],
        custody: CustodyUse {
            operations: [Op::Status, Op::Release, Op::Attest, Op::Sign].into(),
            salts: vec![
                SaltUse::persona(SaltPattern::Prefix("mere-cli/".into())),
                // The WebRTC door signs its invites and challenges with this
                // salt's key, whichever app runs the door.
                SaltUse::persona(SaltPattern::Exact(
                    "mere.graphshell/webrtc-host-signing/v1".into(),
                )),
            ],
            acts: vec![SaltUse::persona(SaltPattern::DelegationScope(DOMAIN.into()))],
            ..CustodyUse::default()
        },
        ..AppDeclaration::default()
    }
}

pub type Identity = Arc<dyn Signer + Send + Sync>;

/// The custodian's persona, or the seed file a test build names.
pub fn identity(seed: Option<&Path>) -> Result<Identity> {
    if let Some(path) = seed {
        return seeded(path);
    }
    let identity = castellan_client::custody_identity::CustodyIdentity::connect_declared_timeout(
        APP,
        declaration(),
        Duration::from_secs(10),
    )
    .map_err(|error| anyhow::anyhow!("the custodian is not available: {error}"))?;
    Ok(Arc::new(identity))
}

#[cfg(feature = "test-identity")]
fn seeded(path: &Path) -> Result<Identity> {
    use anyhow::Context;
    let text = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let bytes: Vec<u8> = (0..text.trim().len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text.trim()[i..i + 2], 16))
        .collect::<Result<_, _>>()
        .context("the seed file holds 64 hex digits")?;
    let seed: [u8; 32] = bytes.try_into().map_err(|_| anyhow::anyhow!("the seed is 32 bytes"))?;
    Ok(Arc::new(personae::InMemoryProvider::from_seed(seed)))
}

#[cfg(not(feature = "test-identity"))]
fn seeded(_: &Path) -> Result<Identity> {
    anyhow::bail!("--identity-seed needs a test-identity build")
}
