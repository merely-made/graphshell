// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! A browser reader, through graphshell-host's WebRTC door (graphshell:mere-cli
//! D11, D18 to D20). The invite is the door's bearer fragment, granting the
//! one query by its digest; signaling is graphshell-host's, with `/query`
//! added so the page can fetch the text it must send. The reader opens
//! Turnstone's `query.html` with the signaling address and the fragment.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use graphshell_host::invitation::QUERY_ACTION;
use graphshell_host::projection_host::{ResidentProjectionHost, admit_webrtc_catalog};
use graphshell_host::signaling::{Reply, SignalingConfig, SignalingHooks, serve_signaling};
use graphshell_host::webrtc_door::{InviteTerms, issue_invite};
use graphshell_host::webrtc_session::HostedInvite;
use notochord::{NetworkId, RequestedAction};
use insigne::DerivedKeyAttestation;
use personae::{Ed25519Keypair, Ed25519PublicKey, IdentityError, IdentityProvider};
use serde::Serialize;
use tokio::sync::Mutex;
use webrtc_carrier::ReleaseRefV1;
use webrtc_carrier::native::CarrierConfig;

use crate::carriers::{now_ms, profile};
use crate::files::write_new;
use crate::identity::{Identity, TERMS};

/// How long one admitted browser session's grant lasts before it must rejoin.
const SESSION_TTL_MS: u64 = 60 * 60 * 1000;

/// C4's placeholder release, until Turnstone's web app publishes releases
/// (D19). A claim the host signs, never a grant of executable trust.
const RELEASE: ReleaseRefV1 = ReleaseRefV1 {
    manifest_blake3: [0x5a; 32],
    publisher_key_id: [0x6b; 32],
};

/// What the owner hands a browser reader.
#[derive(Serialize)]
struct BrowserInvite {
    version: u16,
    /// Where the page posts its offer and fetches `/query`.
    signal: String,
    /// The door's invite fragment.
    fragment: String,
    /// What to append to the query page's address so it opens filled in.
    page: String,
}

/// The door takes a sized provider; the CLI holds its identity behind a trait
/// object.
struct Provider(Identity);

impl IdentityProvider for Provider {
    fn master_public_key(&self) -> Ed25519PublicKey {
        self.0.master_public_key()
    }
    fn derive_keypair(&self, salt: &[u8]) -> Result<Ed25519Keypair, IdentityError> {
        self.0.derive_keypair(salt)
    }
    fn attest_derived_key(&self, salt: &[u8]) -> Result<DerivedKeyAttestation, IdentityError> {
        self.0.attest_derived_key(salt)
    }
}

pub struct Browser {
    pub listen: SocketAddr,
    pub uses: u32,
    pub expires_at_ms: u64,
    pub session: uuid::Uuid,
    pub digest: chirograph::ContentHash,
    pub allowed: String,
    pub invite: PathBuf,
}

/// Issue a door invite for the one query, write it, and serve signaling and
/// admitted browsers until the process stops.
pub async fn serve(browser: Browser, identity: Identity, host: ResidentProjectionHost) -> Result<()> {
    let root = identity.master_public_key().to_bytes();
    let provider = Arc::new(Provider(identity));
    let issue = issue_invite(
        provider.as_ref(),
        &InviteTerms {
            network: NetworkId(TERMS.network(root, browser.session)),
            profile: profile(),
            action: RequestedAction {
                domain: TERMS.domain.into(),
                path: TERMS.query_path(browser.session, &browser.digest),
                action: QUERY_ACTION.into(),
            },
            expires_at_ms: browser.expires_at_ms,
            max_uses: browser.uses,
            release: RELEASE,
        },
    )
    .map_err(|error| anyhow::anyhow!("issue the browser invite: {error}"))?;
    let fragment = issue.descriptor.invite.to_fragment();
    let hosted = Arc::new(Mutex::new(HostedInvite {
        invite: issue.descriptor.invite,
        redemption: issue.redemption,
    }));
    let host = Arc::new(Mutex::new(host));
    let listener = tokio::net::TcpListener::bind(browser.listen)
        .await
        .with_context(|| format!("listen for signaling on {}", browser.listen))?;
    write_new(&browser.invite, &BrowserInvite {
        version: 1,
        signal: format!("http://{}", browser.listen),
        fragment: fragment.clone(),
        page: format!("query.html#signal=http://{}&invite={fragment}", browser.listen),
    })?;
    eprintln!(
        "Ready. Share {} privately with the reader, who opens it in Turnstone's query page. \
         Press Ctrl-C to stop.",
        browser.invite.display()
    );
    let allowed = browser.allowed;
    let hooks = SignalingHooks::joined(move |id, answerer| {
        let (host, hosted, provider) = (Arc::clone(&host), Arc::clone(&hosted), Arc::clone(&provider));
        tokio::spawn(async move {
            let joined = async {
                let carrier = answerer.accept().await.map_err(|error| error.to_string())?;
                let mut hosted = hosted.lock().await;
                admit_webrtc_catalog(&host, carrier, provider.as_ref(), &mut hosted, root, SESSION_TTL_MS, now_ms)
                    .await
                    .map_err(|error| error.to_string())
            };
            match joined.await {
                Ok(served) => {
                    eprintln!("A browser reader was admitted.");
                    if let Err(error) = served.finish().await {
                        eprintln!("[session {id}] {error}");
                    }
                },
                Err(error) => eprintln!("[session {id}] refused: {error}"),
            }
        });
    })
    .with_route(move |method, path| {
        (method == "GET" && path == "/query").then(|| Reply::new("200 OK", allowed.clone()))
    })
    .with_report(|id, error| eprintln!("[signal {id}] {error}"));
    let config = SignalingConfig {
        bind: browser.listen,
        advertise: vec![browser.listen.ip()],
        fragment,
        carrier: CarrierConfig::default(),
    };
    serve_signaling(listener, config, hooks).await?;
    Ok(())
}
