// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The Graphshell request/response loop over an admitted stream.
//!
//! [`crate::carrier`] gets a peer admitted; this is what happens next. The
//! wire stays NDJSON ([`crate::line`]), the same lines the stdio carrier
//! speaks, because the carriers differ in who they can prove and how long
//! they last, not in what a `Snapshot` request means. Converting Graphshell to length-prefixed frames
//! merely to match Murm and Notochord would be alignment for its own sake.
//!
//! ## Where the two carriers actually differ
//!
//! The verbs that mean the same thing everywhere come from
//! [`graphshell_endpoint::dispatch_common`]. Only the session plane is this
//! module's own, and its answers are the exact inverse of stdio's:
//!
//! | verb | stdio | admitted carrier |
//! |---|---|---|
//! | `Open` | refused: inherited pipes prove no peer | answered: the handshake proved one |
//! | `Suspend` | refused: no session outlives the process | answered: this one can reconnect |
//! | `Close` | terminal | terminal |
//!
//! ## Authority is rechecked, not assumed
//!
//! Admission happened once. A grant can expire or be revoked while a session
//! is open, so every request is checked against the retained authority before
//! it reaches the endpoint. This is where [`crate::lifecycle`] stops being a
//! set of unit-tested rules and starts governing live traffic: a lapsed
//! session is refused and the loop ends, rather than serving scenes on
//! authority that stopped holding.

use chirograph::{
    CarrierFailure, CarrierNotice, CarrierOutput, CarrierRequest, CarrierResponse,
    CarrierResponseBody, ResumeReply, ResumeRequest, SessionOpened, SessionStatus,
};
use graphshell_endpoint::{
    IntentSink, PresentationSource, ProjectionCatalog, ProjectionSource, SessionPlaneVerb,
    dispatch_common,
};
use notochord::{AdmittedSession, RevocationLedger};
use std::fmt::Display;
use std::sync::RwLock;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};

use crate::lifecycle::{Lapse, SessionAuthority};
use crate::line::{self, LineReader};

/// Why a session loop stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionEnd {
    /// The peer sent `Close`.
    Closed,
    /// The peer sent `Suspend`; the session may resume later.
    Suspended,
    /// The peer stopped sending without saying goodbye.
    Disconnected,
    /// The session's authority stopped holding, and it was told so.
    Lapsed(Lapse),
}

impl SessionEnd {
    /// The status a client may render for this ending.
    pub fn status(self) -> SessionStatus {
        match self {
            SessionEnd::Closed | SessionEnd::Suspended => SessionStatus::Stale,
            SessionEnd::Disconnected => SessionStatus::Disconnected,
            SessionEnd::Lapsed(Lapse::Expired { .. }) => SessionStatus::Expired,
            SessionEnd::Lapsed(Lapse::Revoked) => SessionStatus::Revoked,
        }
    }
}

/// What one served session did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionSummary {
    /// Requests answered, including refusals.
    pub answered: u64,
    /// Why the loop stopped.
    pub end: SessionEnd,
}

/// Failure that ends a session loop without an answer.
#[derive(Debug, thiserror::Error)]
pub enum SessionLoopError {
    /// The socket failed.
    #[error("session transport failed: {0}")]
    Transport(#[from] std::io::Error),
    /// The endpoint could not say whether it had a notice to send.
    ///
    /// Distinct from a transport failure because it is the endpoint that
    /// broke, not the link, and a log that conflates the two sends whoever
    /// reads it to the wrong half.
    #[error("endpoint notice source failed: {0}")]
    Notices(String),
}

/// The poller a session without a notice lane passes.
///
/// Never called; a bare function pointer is only there to give the absent
/// case a concrete type.
pub(crate) type NoNotices<E> = fn(&mut E) -> Result<Option<CarrierNotice>, String>;

/// Serve one admitted session until it closes, suspends, or lapses.
///
/// `now_ms` is called per request rather than once, so a session that outlives
/// its grant notices at the next request instead of at the next reconnect.
/// `revocations` is shared for the same reason in the other direction: an
/// owner who withdraws a grant mid-session is answered at the very next
/// request, not at the next reconnect.
pub async fn serve_admitted_session<E, S, F, N>(
    session: &mut AdmittedSession<S>,
    authority: &SessionAuthority,
    revocations: &RwLock<RevocationLedger>,
    endpoint: &mut E,
    resume: &mut F,
    now_ms: N,
) -> Result<SessionSummary, SessionLoopError>
where
    E: ProjectionCatalog + ProjectionSource + PresentationSource + IntentSink,
    <E as ProjectionSource>::Error: Display,
    <E as PresentationSource>::Error: Display,
    <E as IntentSink>::Error: Display,
    S: AsyncRead + AsyncWrite + Unpin,
    F: FnMut(&mut E, ResumeRequest) -> Result<ResumeReply, String>,
    N: Fn() -> u64,
{
    serve_admitted_session_with(
        session,
        authority,
        revocations,
        endpoint,
        resume,
        now_ms,
        None::<(Duration, NoNotices<E>)>,
        |_| true,
    )
    .await
}

/// The loop both entry points share.
///
/// `notices` is the only difference between a session that can ring and one
/// that cannot: given a poll interval and a poller, the loop writes a notice
/// frame before each read and again whenever the peer falls quiet for the
/// interval. Polling in both places is what makes a busy client and an idle
/// one behave the same: a peer sending faster than the interval would
/// otherwise never let the timeout arm fire.
pub(crate) async fn serve_admitted_session_with<E, S, F, N, P, L>(
    session: &mut AdmittedSession<S>,
    authority: &SessionAuthority,
    revocations: &RwLock<RevocationLedger>,
    endpoint: &mut E,
    resume: &mut F,
    now_ms: N,
    mut notices: Option<(Duration, P)>,
    is_live: L,
) -> Result<SessionSummary, SessionLoopError>
where
    E: ProjectionCatalog + ProjectionSource + PresentationSource + IntentSink,
    <E as ProjectionSource>::Error: Display,
    <E as PresentationSource>::Error: Display,
    <E as IntentSink>::Error: Display,
    S: AsyncRead + AsyncWrite + Unpin,
    F: FnMut(&mut E, ResumeRequest) -> Result<ResumeReply, String>,
    N: Fn() -> u64,
    P: FnMut(&mut E) -> Result<Option<CarrierNotice>, String>,
    L: Fn(&E) -> bool,
{
    let (reader, mut writer) = tokio::io::split(&mut session.stream);
    let mut lines = LineReader::new(reader);
    let mut answered = 0u64;

    let end = loop {
        if !is_live(endpoint) {
            break SessionEnd::Disconnected;
        }
        // Before blocking on the peer: anything the endpoint produced while
        // the last request was being served goes out now.
        if let Some((_, poll)) = notices.as_mut() {
            while let Some(notice) = poll(endpoint).map_err(SessionLoopError::Notices)? {
                if !is_live(endpoint) {
                    break;
                }
                write_notice(&mut writer, &notice).await?;
            }
        }

        let read = match notices.as_ref() {
            None => lines.next::<CarrierRequest>().await?,
            // `LineReader::next` is cancel safe, so a timed-out read keeps
            // whatever it had buffered and the next call resumes the same
            // line. A partial frame is never lost or re-read.
            Some((interval, _)) => {
                match tokio::time::timeout(*interval, lines.next::<CarrierRequest>()).await {
                    Ok(read) => read?,
                    Err(_elapsed) => continue,
                }
            },
        };
        let Some(read) = read else {
            // The peer stopped without a verb. Not an error: a dropped link
            // and a rude client look identical from here, and neither is this
            // module's to judge.
            break SessionEnd::Disconnected;
        };

        let request = match read {
            Ok(request) => request,
            Err(error) => {
                write_line(
                    &mut writer,
                    &failure(0, format!("invalid carrier request: {error}")),
                )
                .await?;
                answered += 1;
                continue;
            },
        };
        let id = request.id;

        // Before the endpoint sees it. A revoked grant must not buy one more
        // snapshot on its way out.
        //
        // Read fresh each request rather than closed over once: the owner may
        // revoke *during* a session, and a loop holding one snapshot of the
        // ledger would serve the rest of that session on authority that had
        // already been withdrawn. The lock is taken and dropped without an
        // await in between.
        let lapse = {
            let ledger = revocations
                .read()
                .expect("the revocation ledger lock is never poisoned by this loop");
            authority.lapse(&ledger, now_ms())
        };
        if let Some(lapse) = lapse {
            write_line(&mut writer, &failure(id, lapse_message(lapse))).await?;
            answered += 1;
            break SessionEnd::Lapsed(lapse);
        }

        let session_plane = match dispatch_common(endpoint, request, resume) {
            Ok(response) => {
                if !is_live(endpoint) {
                    break SessionEnd::Disconnected;
                }
                write_line(&mut writer, &response).await?;
                answered += 1;
                continue;
            },
            Err(session_plane) => session_plane,
        };

        answered += 1;
        match session_plane.verb {
            // Answerable here precisely because stdio cannot: the carrier
            // authenticated this peer before a byte of this reached us.
            SessionPlaneVerb::Open(open) => {
                let opened = SessionOpened {
                    version: open.version,
                    descriptor: endpoint.describe(),
                    status: SessionStatus::Live,
                    // A hint for pre-emptive renewal. The endpoint still
                    // rechecks every request, because a grant can be revoked
                    // long before it expires.
                    expires_at_ms: authority.deadline_ms(),
                };
                if !is_live(endpoint) {
                    break SessionEnd::Disconnected;
                }
                write_line(
                    &mut writer,
                    &ok(id, CarrierResponseBody::Opened(Box::new(opened))),
                )
                .await?;
            },
            SessionPlaneVerb::Close => {
                write_line(&mut writer, &ok(id, CarrierResponseBody::Closed)).await?;
                break SessionEnd::Closed;
            },
            SessionPlaneVerb::Suspend => {
                write_line(&mut writer, &ok(id, CarrierResponseBody::Suspended)).await?;
                break SessionEnd::Suspended;
            },
        }
    };

    // Finish rather than drop, for the reason recorded in `crate::carrier`:
    // both arms happen to drain today through Drop implementations nobody
    // chose, and an ending that says so explicitly is a guarantee instead.
    let _ = writer.shutdown().await;
    Ok(SessionSummary { answered, end })
}

fn ok(id: u64, body: CarrierResponseBody) -> CarrierResponse {
    CarrierResponse { id, body: Ok(body) }
}

fn failure(id: u64, message: String) -> CarrierResponse {
    CarrierResponse {
        id,
        body: Err(CarrierFailure { message }),
    }
}

fn lapse_message(lapse: Lapse) -> String {
    match lapse {
        Lapse::Expired { at_ms } => format!("session authority expired at {at_ms}"),
        Lapse::Revoked => "session authority was revoked".to_string(),
    }
}

async fn write_line<W>(writer: &mut W, response: &CarrierResponse) -> Result<(), SessionLoopError>
where
    W: AsyncWrite + Unpin,
{
    Ok(line::write(writer, response).await?)
}

/// Write one revision bell.
///
/// The envelope is stated rather than implied. `CarrierOutput` is untagged, so
/// these are the same bytes a bare notice would produce; naming the variant is
/// what tells the next reader that a notice is a distinct kind of frame and
/// not a response that lost its id.
async fn write_notice<W>(writer: &mut W, notice: &CarrierNotice) -> Result<(), SessionLoopError>
where
    W: AsyncWrite + Unpin,
{
    Ok(line::write(writer, &CarrierOutput::Notice(notice.clone())).await?)
}

#[cfg(test)]
mod tests;
