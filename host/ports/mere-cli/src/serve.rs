// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The owner's half: issue an invitation to one contact for one query
//! (graphshell:mere-cli D7, D8), serve it, and fold any revocation written
//! beside it (D17).

use std::collections::BTreeSet;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use chirograph::ContentHash;
use graphshell_host::invitation::{Card, Contact, Invitation};
use graphshell_host::query_host::{query_host, query_policy};
use insigne::delegation::SignedDelegationRevocation;
use notochord::{NetworkId, RevocationLedger};

use crate::carriers::{self, Listen, now_ms, profile};
use crate::files::{InviteFile, Via, read_json, read_query, revocations_beside, write_new};
use crate::identity::{Identity, TERMS};

#[derive(clap::Args)]
pub struct ServeArgs {
    /// The RDF dataset to serve.
    #[arg(long)]
    graph: PathBuf,
    /// The one query the invitation permits. A reader's query must match it
    /// byte for byte.
    #[arg(long)]
    allow_query: PathBuf,
    /// The reader's contact (`mere contact`). Not for `--via webrtc`, whose
    /// invite is a bearer link for a browser.
    #[arg(long)]
    contact: Option<PathBuf>,
    /// Where to write the new invitation for the reader.
    #[arg(long)]
    invite: PathBuf,
    #[arg(long, value_enum)]
    via: Serving,
    /// How long the invitation lasts: `30m`, `12h`, `7d`; at most `30d`.
    #[arg(long, default_value = "7d")]
    expires: String,
    /// Reticulum's reachable local IP and port, or WebRTC's signaling and
    /// UDP address (the address the browser reaches).
    #[arg(long)]
    listen: Option<SocketAddr>,
    /// How many browser sessions a WebRTC invite admits.
    #[arg(long, default_value_t = 8)]
    uses: u32,
    /// The loopback I2P SAM bridge (default 127.0.0.1:7656).
    #[arg(long)]
    sam: Option<SocketAddr>,
    /// The directory to watch for revocations (default: `<invite>.revocations`).
    #[arg(long)]
    revocations: Option<PathBuf>,
}

/// The carriers `serve` offers: the native ones, and a browser.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum Serving {
    Iroh,
    I2p,
    Reticulum,
    Webrtc,
}

/// `30m`, `12h` or `7d` in milliseconds.
pub fn lifetime_ms(text: &str) -> Result<u64> {
    let (count, unit) = text.split_at(text.len().saturating_sub(1));
    let count: u64 = count.parse().with_context(|| format!("{text:?} is not like 12h or 7d"))?;
    let unit_ms = match unit {
        "m" => 60_000,
        "h" => 3_600_000,
        "d" => 86_400_000,
        _ => bail!("{text:?} is not like 30m, 12h or 7d"),
    };
    count.checked_mul(unit_ms).context("that lifetime overflows")
}

pub async fn serve(args: ServeArgs, identity: Identity) -> Result<()> {
    if args.invite.exists() {
        bail!("{} exists; choose a new invitation file", args.invite.display());
    }
    let dataset = Arc::new(crate::local::dataset(&args.graph)?);
    let allowed = read_query(&args.allow_query)?;
    let now = now_ms();
    let session = uuid::Uuid::new_v4();
    let expires_at_ms = now + lifetime_ms(&args.expires)?;
    let digest = ContentHash::of(allowed.as_bytes());
    let via = match args.via {
        Serving::Iroh => Via::Iroh,
        Serving::I2p => Via::I2p,
        Serving::Reticulum => Via::Reticulum,
        Serving::Webrtc => {
            return serve_browser(args, identity, dataset, allowed, session, digest, expires_at_ms).await;
        },
    };
    let contact: Contact = read_json(
        args.contact
            .as_deref()
            .context("--contact names the reader to invite")?,
    )?;
    let invitation = Invitation::issue_query(
        identity.as_ref(),
        &TERMS,
        session,
        contact,
        digest,
        now,
        expires_at_ms,
    )
    .map_err(anyhow::Error::msg)?;
    let owner = invitation.owner.clone();
    let root = owner.root();
    let policy = query_policy(
        &TERMS,
        NetworkId(TERMS.network(root, session)),
        root,
        vec![profile()],
        Some(8),
    );
    let issued = invitation.clone();
    let mut host = query_host(policy, TERMS, dataset, move |principal, chain| {
        issued.admits(&TERMS, principal, chain, false, now_ms())
    })
    .map_err(anyhow::Error::msg)?;
    let watched = args
        .revocations
        .clone()
        .unwrap_or_else(|| revocations_beside(&args.invite));
    tokio::spawn(watch_revocations(watched, Arc::clone(host.revocations())));

    let listen = Listen {
        listen: args.listen,
        sam: args.sam,
    };
    let path = args.invite.clone();
    carriers::serve(via, &identity, listen, &mut host, move |ticket| {
        let card = Card {
            version: 1,
            session,
            owner,
            ticket,
        };
        write_new(&path, &InviteFile {
            version: 1,
            via,
            invitation,
            card,
        })?;
        eprintln!(
            "Ready. Share {} privately with the reader. Press Ctrl-C to stop.",
            path.display()
        );
        Ok(())
    })
    .await
}

/// A browser invite through the door: a bearer link with a use count (D11).
#[cfg(feature = "webrtc")]
#[allow(clippy::too_many_arguments)]
async fn serve_browser(
    args: ServeArgs,
    identity: Identity,
    dataset: Arc<mere::dataset::IndexedDataset>,
    allowed: String,
    session: uuid::Uuid,
    digest: ContentHash,
    expires_at_ms: u64,
) -> Result<()> {
    let listen = args
        .listen
        .context("WebRTC needs --listen <IP:port the browser reaches>")?;
    let root = identity.master_public_key().to_bytes();
    let policy = query_policy(
        &TERMS,
        NetworkId(TERMS.network(root, session)),
        root,
        vec![profile()],
        Some(8),
    );
    // The door minted the one delegation a browser presents, on this owner's key.
    let host = query_host(policy, TERMS, dataset, move |_, chain| match chain {
        [grant] if grant.certificate.issuer == root => Ok(()),
        _ => Err("not a grant this owner's door minted".into()),
    })
    .map_err(anyhow::Error::msg)?;
    crate::browser::serve(
        crate::browser::Browser {
            listen,
            uses: args.uses,
            expires_at_ms,
            session,
            digest,
            allowed,
            invite: args.invite,
        },
        identity,
        host,
    )
    .await
}

#[cfg(not(feature = "webrtc"))]
#[allow(clippy::too_many_arguments)]
async fn serve_browser(
    _: ServeArgs,
    _: Identity,
    _: Arc<mere::dataset::IndexedDataset>,
    _: String,
    _: uuid::Uuid,
    _: ContentHash,
    _: u64,
) -> Result<()> {
    bail!("this build has no Webrtc carrier: rebuild with `--features webrtc`")
}

/// Fold each signed revocation that appears in `dir`. A file that does not
/// check is reported and skipped; the ledger only honours a revocation from
/// the issuer of the grant it names.
async fn watch_revocations(dir: PathBuf, ledger: Arc<RwLock<RevocationLedger>>) {
    let mut seen = BTreeSet::new();
    loop {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for path in entries.flatten().map(|entry| entry.path()) {
                if seen.insert(path.clone()) {
                    fold(&path, &ledger);
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

fn fold(path: &Path, ledger: &RwLock<RevocationLedger>) {
    let revocation: SignedDelegationRevocation = match read_json(path) {
        Ok(revocation) => revocation,
        Err(error) => return eprintln!("Skipping {}: {error}", path.display()),
    };
    match revocation.check() {
        Ok(checked) => {
            ledger.write().expect("the ledger lock is never poisoned").fold(checked);
            eprintln!("Revocation {} applied.", path.display());
        },
        Err(error) => eprintln!("Skipping {}: {error:?}", path.display()),
    }
}

#[cfg(test)]
mod tests {
    use super::lifetime_ms;

    #[test]
    fn lifetimes_read_minutes_hours_and_days() {
        assert_eq!(lifetime_ms("30m").unwrap(), 1_800_000);
        assert_eq!(lifetime_ms("12h").unwrap(), 43_200_000);
        assert_eq!(lifetime_ms("7d").unwrap(), 604_800_000);
        assert!(lifetime_ms("7w").is_err() && lifetime_ms("d").is_err());
    }
}
