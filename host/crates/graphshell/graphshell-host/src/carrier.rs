// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! G5d: Graphshell's projection carrier.
//!
//! The accept path that runs *before* a single `SessionOpen` byte is read. It
//! is the second of Notochord N2's two service carriers (the first is Murm's
//! real accept path), and it deliberately owns none of the machinery it uses:
//!
//! - the carrier facts come from [`transport::AcceptedSession::into_session`],
//!   the one audited adapter (N1), so this crate never hand-builds a
//!   [`SessionFacts`](notochord::SessionFacts);
//! - the decision comes from the sans-I/O [`crate::admission::admit_session_with`]
//!   and the frames from `notochord`'s framing, so the reply is written before
//!   anything closes, a refusal is finished rather than flushed, and a refused
//!   stream cannot reach an application;
//! - the delegation grammar stays in Personae, and the owner's rules stay in
//!   the policy the host supplies.
//!
//! What is Graphshell's, and lives here: the ALPN and the service path the owner
//! offers. Which actions this service serves is [`crate::admission`]'s.
//!
//! ## The transport must outlive the session
//!
//! [`accept_projection_session`] borrows its transport and never takes
//! ownership. That is load-bearing rather than stylistic. Both carriers drain
//! buffered writes when a *stream* is dropped, but by two unrelated
//! mechanisms sharing one escape hatch:
//!
//! - p2panda: quinn's `Drop for SendStream` finishes the stream, except when
//!   the connection is already errored, which is exactly what dropping the
//!   endpoint underneath it produces;
//! - Reticulum: the outbound relay reads its duplex to EOF, unless the
//!   endpoint has been torn down and the relay task aborted.
//!
//! So a short-lived accept task owning its transport would discard the reply
//! it had already written, on either arm, whenever it returned promptly.
//! Taking `&T` makes that unspellable here.
//!
//! The accept step is therefore the one part of this path that cannot move.
//! [`admit_accepted_session`] carries everything after it, for a caller that
//! already holds an accepted session and has no `&T` to borrow from.
//!
//! ## One admission entry
//!
//! Every stream-carried session is admitted by [`admit_stream`]: an admitted
//! byte stream plus the ingress facts the carrier can honestly report is the
//! seam every carrier reaches (transport agnosticism note, ruling T1). A
//! stream's facts come from its [`AcceptedSession`] through
//! [`admit_accepted_session`]: a transport's, or an in-process door's duplex
//! named `TransportKind::InProcess` (no peer, the derived link). A carrier
//! that hands over the hello as bytes rather than a stream (the WebRTC door)
//! uses the sans-I/O twin, [`crate::admission::admit_session`]. All of them
//! end in one action check: [`crate::admission::serves_action`], or the
//! host's own [`ActionCheck`] (`*_with`) for a host serving a vocabulary of
//! its own under the projection service.
//!
//! ## The served vocabulary is part of admission's answer
//!
//! The owner policy allows only named admission actions, and the host's
//! vocabulary ([`crate::admission::serves_action`], or the `*_with` check)
//! decides which of those it serves, because the service implementation
//! remains authoritative about what it serves even when the owner's
//! serialized vocabulary changes. Both stages are decided before the reply.
//! They are one answer to the peer, so an action the owner allows and the host
//! does not serve is written as `ActionNotOffered` and the stream finished,
//! exactly as a policy refusal is.

use notochord::{
    AdmittedSession, DenyReason, IoHandshakeError, LocalNetworkPolicy, NetworkId, ProfileRef,
    RevocationLedger, ServiceAccess, ServiceRule, SessionFacts, SessionHello, TrustedRoot,
    read_frame, write_frame,
};
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use transport::{AcceptedSession, Alpn, Transport, TransportError};

pub use crate::admission::ActionCheck;
use crate::admission::{
    CONNECT_ACTION, GRAPHSHELL_DOMAIN, PROJECTION_PROTOCOL, PROJECTION_SERVICE, admit_session_with,
    serves_admitted,
};

/// The ALPN a projection session is accepted for.
///
/// The same bytes are the protocol in the signed transcript, so a proof minted
/// for another protocol on the same connection does not verify here.
pub fn projection_alpn() -> Alpn {
    Alpn::from_bytes(PROJECTION_PROTOCOL)
}

/// A policy offering exactly one service: Graphshell projections.
///
/// A convenience for a host that serves projections and nothing else; a host
/// with more services builds its own and inserts the same rule. The value is
/// that the service path and its default posture are stated once.
///
/// `MemberOnly` is the default on purpose. A projection session hands out a
/// live view of a graph, so it wants a delegation chain rather than an open
/// door.
pub fn projection_policy(
    network: NetworkId,
    trusted_roots: Vec<TrustedRoot>,
    accepted_profiles: Vec<ProfileRef>,
    max_sessions: Option<u32>,
) -> LocalNetworkPolicy {
    let mut policy = LocalNetworkPolicy::closed(network);
    policy.trusted_roots = trusted_roots;
    policy.accepted_profiles = accepted_profiles;
    policy.services.insert(
        PROJECTION_SERVICE.to_string(),
        ServiceRule::new(
            ServiceAccess::MemberOnly,
            GRAPHSHELL_DOMAIN,
            [CONNECT_ACTION],
            false,
            max_sessions,
        ),
    );
    policy
}

/// Why a projection session was not served.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProjectionRefusal {
    /// Admission refused the session, and the peer has been told why before
    /// the stream was finished. That covers an action the owner allows and
    /// this host does not serve: `ActionNotOffered`.
    NotAdmitted(DenyReason),
}

/// Errors that stop the accept path before it reaches a decision.
#[derive(Debug, thiserror::Error)]
pub enum ProjectionAcceptError {
    /// The carrier could not accept a session.
    #[error("projection carrier accept failed: {0}")]
    Carrier(#[from] TransportError),
    /// The handshake could not complete on the accepted stream.
    #[error(transparent)]
    Handshake(#[from] IoHandshakeError),
}

/// Accept one projection session, admitted and action-checked.
///
/// On `Ok(Ok(session))` the stream has cleared admission *and* carries an
/// action this service serves, so it is ready for `SessionOpen`. Nothing in
/// `chirograph` restates any of it: the protocol negotiates version
/// and capabilities only, because the principal was settled here.
///
/// `transport` is borrowed; see the module docs on why owning it would lose
/// the reply.
pub async fn accept_projection_session<T: Transport>(
    transport: &T,
    policy: &LocalNetworkPolicy,
    ledger: &RevocationLedger,
    now_ms: u64,
    active_sessions: u32,
) -> Result<Result<AdmittedSession<T::Stream>, ProjectionRefusal>, ProjectionAcceptError> {
    accept_projection_session_with(
        transport,
        policy,
        ledger,
        now_ms,
        active_sessions,
        &serves_admitted,
    )
    .await
}

/// [`accept_projection_session`] with the host's own action check in place
/// of [`serves_action`](crate::admission::serves_action). For a host that
/// serves a vocabulary of its own under the projection service, such as a
/// route-selecting host
/// ([`crate::projection_host::RouteSelector`]).
pub async fn accept_projection_session_with<T: Transport>(
    transport: &T,
    policy: &LocalNetworkPolicy,
    ledger: &RevocationLedger,
    now_ms: u64,
    active_sessions: u32,
    serves: ActionCheck<'_>,
) -> Result<Result<AdmittedSession<T::Stream>, ProjectionRefusal>, ProjectionAcceptError> {
    let accepted = transport.accept(projection_alpn()).await?;
    let (stream, facts) = accepted.into_session();
    admit_stream_with(
        stream,
        &facts,
        policy,
        ledger,
        now_ms,
        active_sessions,
        serves,
    )
    .await
}

/// Admit and action-check a session this carrier did not accept itself.
///
/// [`accept_projection_session`] is [`admit_accepted_session`] with the
/// accept step glued on the front, for a caller that dials `&T` and lets a
/// [`Transport`] impl do the accepting. Not every carrier can do that: the
/// coming browser WebRTC lane accepts its own stream off a signalling
/// channel and has no bilateral `Transport` impl to hand a `&T` for — there
/// is nothing to `.accept(alpn)` on. Rather than make it fabricate one, this
/// function is the seam such a carrier calls directly, taking the
/// [`AcceptedSession`] it already produced and running exactly the same
/// admission `accept_projection_session` runs after its own accept: N1's
/// facts, then [`crate::admission::admit_session_with`].
///
/// Same refusal semantics as `accept_projection_session`: a refusal is
/// finished, never dropped, and a refused stream never reaches the caller.
pub async fn admit_accepted_session<S>(
    accepted: AcceptedSession<S>,
    policy: &LocalNetworkPolicy,
    ledger: &RevocationLedger,
    now_ms: u64,
    active_sessions: u32,
) -> Result<Result<AdmittedSession<S>, ProjectionRefusal>, ProjectionAcceptError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    // N1's adapter, not a local copy: every fact below is read off the
    // acceptance record before any application byte is read.
    let (stream, facts) = accepted.into_session();
    admit_stream(stream, &facts, policy, ledger, now_ms, active_sessions).await
}

/// Admit and action-check a session over `stream`, given the facts its
/// carrier reports.
///
/// The one entry every stream-carried admission runs (module docs). `facts`
/// must be what the carrier can honestly say, which is what
/// [`AcceptedSession::into_session`] gives (use [`admit_accepted_session`]).
/// Nothing here trusts a fact the hello carries.
///
/// A refusal is finished, never dropped, and a refused stream never reaches
/// the caller: on `Ok(Ok(session))` the stream cleared admission and carries
/// an action this service serves.
pub async fn admit_stream<S>(
    stream: S,
    facts: &SessionFacts,
    policy: &LocalNetworkPolicy,
    ledger: &RevocationLedger,
    now_ms: u64,
    active_sessions: u32,
) -> Result<Result<AdmittedSession<S>, ProjectionRefusal>, ProjectionAcceptError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    admit_stream_with(
        stream,
        facts,
        policy,
        ledger,
        now_ms,
        active_sessions,
        &serves_admitted,
    )
    .await
}

/// [`admit_stream`] with the host's own [`ActionCheck`].
pub async fn admit_stream_with<S>(
    mut stream: S,
    facts: &SessionFacts,
    policy: &LocalNetworkPolicy,
    ledger: &RevocationLedger,
    now_ms: u64,
    active_sessions: u32,
    serves: ActionCheck<'_>,
) -> Result<Result<AdmittedSession<S>, ProjectionRefusal>, ProjectionAcceptError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let limits = policy.limits.clamped();
    let hello = read_frame(&mut stream, limits.max_hello_bytes)
        .await
        .map_err(IoHandshakeError::from)?;
    // Decided before anything is written: the owner's rule and the host's
    // vocabulary give the one answer the reply carries.
    let (reply, outcome) = admit_session_with(
        policy,
        ledger,
        &hello,
        facts,
        now_ms,
        active_sessions,
        serves,
    );
    write_frame(&mut stream, &reply, limits.max_reply_bytes)
        .await
        .map_err(IoHandshakeError::from)?;

    match outcome {
        Ok(principal) => {
            // `admit_session_with` accepted this exact bounded frame, so
            // decoding it again cannot turn unverified bytes into claims.
            let claims = SessionHello::decode(&hello, &limits)
                .map_err(IoHandshakeError::from)?
                .claims();
            Ok(Ok(AdmittedSession {
                stream,
                principal,
                claims,
                facts: facts.clone(),
                limits,
            }))
        },
        Err(reason) => {
            // Finish rather than drop. Both arms happen to drain on drop
            // today, through two Drop impls nobody chose; saying it
            // explicitly is the difference between a guarantee and an
            // accident.
            let _ = stream.shutdown().await;
            Ok(Err(ProjectionRefusal::NotAdmitted(reason)))
        },
    }
}

#[cfg(test)]
mod tests;
