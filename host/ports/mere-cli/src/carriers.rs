// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The carriers, each behind its own feature (graphshell:mere-cli D16), all
//! through graphshell-host: the owner serves a [`query_host`] with
//! `accept_one`, and a reader dials with `dial_projection_session_bound` and
//! pages its rows over a [`NetworkCarrier`].
//!
//! [`query_host`]: graphshell_host::query_host::query_host

use std::net::SocketAddr;

use anyhow::{Context, Result, bail, ensure};
use chirograph::{
    CapabilityProfile, Carrier, CarrierRequestBody, CarrierResponseBody, ProjectionSession,
    ProtocolVersion, QueryRequest, QueryRow, SessionOpen,
};
use graphshell_host::carrier::ProjectionAcceptError;
use graphshell_host::network_carrier::{
    CarrierRuntime, NetworkCarrier, dial_projection_session_bound,
};
use graphshell_host::projection_host::{ResidentProjectionError, ResidentProjectionHost};
use notochord::ProfileRef;
use tokio::io::{AsyncRead, AsyncWrite};
use transport::{PeerID, Transport};

use crate::files::{InviteFile, Via};
use crate::identity::{Identity, TERMS};

pub fn profile() -> ProfileRef {
    ProfileRef {
        id: "mere.base".into(),
        revision: 1,
    }
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("the clock is after 1970")
        .as_millis() as u64
}

/// Where the owner listens, per carrier.
#[derive(Clone, Copy, Debug, Default)]
pub struct Listen {
    /// Reticulum's TCP server address.
    pub listen: Option<SocketAddr>,
    /// The loopback I2P SAM bridge.
    pub sam: Option<SocketAddr>,
}

fn refused(via: Via) -> anyhow::Error {
    anyhow::anyhow!(
        "this build has no {via:?} carrier: rebuild with `--features {}`",
        format!("{via:?}").to_lowercase()
    )
}

/// Bind `via` as `identity`, hand its ticket to `ready`, then admit readers
/// until the process stops.
pub async fn serve(
    via: Via,
    identity: &Identity,
    listen: Listen,
    host: &mut ResidentProjectionHost,
    ready: impl FnOnce(String) -> Result<()>,
) -> Result<()> {
    let key = identity
        .derived_keypair(TERMS.salt)
        .map_err(|error| anyhow::anyhow!("derive the transport key: {error}"))?;
    let alpn = graphshell_host::carrier::projection_alpn();
    let _ = (&key, &alpn, &listen);
    match via {
        #[cfg(feature = "iroh")]
        Via::Iroh => {
            let t = transport::P2pandaTransport::builder_for(&key)
                .alpns(vec![alpn])
                .bind()
                .await?;
            ready(t.ticket().await?)?;
            accept_forever(host, &t).await
        },
        #[cfg(feature = "i2p")]
        Via::I2p => {
            let t = transport::i2p_transport::I2pTransport::new(i2p_session(listen.sam, true).await?, &key, vec![alpn]);
            ready(t.ticket()?)?;
            accept_forever(host, &t).await
        },
        #[cfg(feature = "reticulum")]
        Via::Reticulum => {
            let address = listen
                .listen
                .context("Reticulum needs --listen <reachable-local-IP:port>")?;
            ensure!(
                !address.ip().is_unspecified() && address.port() > 0,
                "--listen needs a reachable IP and a nonzero port"
            );
            let t = reticulum(&key, transport::ReticulumInterface::TcpServer { bind: address }).await?;
            ready(address.to_string())?;
            accept_forever(host, &t).await
        },
        #[allow(unreachable_patterns)]
        other => {
            let _ = ready;
            Err(refused(other))
        },
    }
}

async fn accept_forever<T: Transport>(host: &mut ResidentProjectionHost, t: &T) -> Result<()> {
    loop {
        match host.accept_one(t, now_ms).await {
            Ok(Ok(_served)) => eprintln!("A reader was admitted."),
            Ok(Err(refusal)) => eprintln!("A session was refused: {refusal:?}"),
            Err(ResidentProjectionError::Accept(ProjectionAcceptError::Handshake(error))) => {
                eprintln!("A session failed: {error}")
            },
            Err(error) => return Err(error.into()),
        }
    }
}

/// Dial the owner `invite` names and read every row of `query`.
pub async fn ask(identity: &Identity, invite: &InviteFile, query: String, sam: Option<SocketAddr>) -> Result<Vec<QueryRow>> {
    invite.check()?;
    ensure!(
        identity.master_public_key().to_bytes() == invite.invitation.guest.root(),
        "this invitation was issued to another persona"
    );
    let key = identity
        .derived_keypair(TERMS.salt)
        .map_err(|error| anyhow::anyhow!("derive the transport key: {error}"))?;
    let owner = PeerID::from_bytes(&invite.card.owner.peer())?;
    let alpn = graphshell_host::carrier::projection_alpn();
    let _ = (&key, &alpn, &owner, &sam);
    match invite.via {
        #[cfg(feature = "iroh")]
        Via::Iroh => {
            let t = transport::P2pandaTransport::builder_for(&key).bind().await?;
            let peer = t.add_peer_ticket(&invite.card.ticket).await?;
            ensure!(peer == owner, "the card's ticket names another peer");
            let rows = dial_and_read(&t, owner, identity, invite, query).await;
            t.close().await?;
            rows
        },
        #[cfg(feature = "i2p")]
        Via::I2p => {
            let t = transport::i2p_transport::I2pTransport::new(i2p_session(sam, false).await?, &key, vec![]);
            let peer = t.add_peer_ticket(&invite.card.ticket)?;
            ensure!(peer == owner, "the card's ticket names another peer");
            dial_and_read(&t, owner, identity, invite, query).await
        },
        #[cfg(feature = "reticulum")]
        Via::Reticulum => {
            let addr: SocketAddr = invite.card.ticket.parse().context("the card's Reticulum address")?;
            let t = reticulum(&key, transport::ReticulumInterface::TcpClient { addr }).await?;
            dial_and_read(&t, owner, identity, invite, query).await
        },
        #[allow(unreachable_patterns)]
        other => Err(refused(other)),
    }
}

async fn dial_and_read<T: Transport>(
    t: &T,
    owner: PeerID,
    identity: &Identity,
    invite: &InviteFile,
    query: String,
) -> Result<Vec<QueryRow>> {
    let invitation = &invite.invitation;
    let stream = dial_projection_session_bound(
        t,
        owner,
        |binding| invitation.hello_bound(&TERMS, identity.as_ref(), profile(), binding),
        &Default::default(),
    )
    .await?
    .map_err(|reason| anyhow::anyhow!("the owner refused this invitation: {reason:?}"))?;
    let handle = tokio::runtime::Handle::current();
    tokio::task::spawn_blocking(move || read_rows(stream, query, handle)).await?
}

fn read_rows<S>(stream: S, query: String, handle: tokio::runtime::Handle) -> Result<Vec<QueryRow>>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut carrier = NetworkCarrier::over(stream, CarrierRuntime::borrowed(handle));
    let open = CarrierRequestBody::Open(Box::new(SessionOpen {
        version: ProtocolVersion { major: 1, minor: 0 },
        capabilities: CapabilityProfile::default(),
    }));
    match carrier.request(open).map_err(|error| anyhow::anyhow!("{error:?}"))? {
        CarrierResponseBody::Opened(_) => {},
        other => bail!("the owner answered the opening with {other:?}"),
    }
    let mut request = QueryRequest::first(ProjectionSession("reader".into()), query);
    let mut rows = Vec::new();
    loop {
        let body = CarrierRequestBody::Query(request.clone());
        let page = match carrier.request(body) {
            Ok(CarrierResponseBody::Query(page)) => page,
            Ok(other) => bail!("the owner answered the query with {other:?}"),
            Err(chirograph::CarrierError::Refused(message)) => bail!("peer: {message}"),
            Err(error) => bail!("peer: {error:?}"),
        };
        rows.extend(page.rows.iter().cloned());
        match request.next(&page) {
            Some(next) => request = next,
            None => break,
        }
    }
    let _ = carrier.request(CarrierRequestBody::Close);
    Ok(rows)
}

#[cfg(feature = "i2p")]
async fn i2p_session(sam: Option<SocketAddr>, publish: bool) -> Result<transport::i2p::I2pSession> {
    use std::net::Ipv4Addr;
    use std::time::Duration;
    let bridge = sam.unwrap_or(SocketAddr::from((Ipv4Addr::LOCALHOST, 7656)));
    ensure!(
        bridge.ip().is_loopback() && bridge.port() > 0,
        "--sam needs a loopback address and port"
    );
    transport::i2p::I2pSession::create(
        transport::i2p::Bridge::new(bridge),
        transport::i2p::Key::Transient,
        transport::i2p::TunnelProfile::new(2, 2, 2, publish)?,
        Duration::from_secs(285),
    )
    .await
    .context("create an I2P session through the local SAM router")
}

#[cfg(feature = "reticulum")]
async fn reticulum(
    key: &personae::DerivedKeypair,
    interface: transport::ReticulumInterface,
) -> Result<transport::ReticulumTransport> {
    use std::time::Duration;
    Ok(transport::ReticulumTransport::builder_for(key)
        .interfaces(vec![interface])
        .alpns(vec![graphshell_host::carrier::projection_alpn()])
        .announce_interval(Duration::from_millis(750))
        .connect_timeout(Duration::from_secs(90))
        .bind()
        .await?)
}
