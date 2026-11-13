// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The live half of the WebRTC door: the join sequence over a real channel,
//! and the admitted session it produces.
//!
//! [`crate::webrtc_door`] is deliberately sans-I/O — bytes in, bytes out,
//! every rule testable without a socket. This module is where those bytes
//! actually travel. It stays as thin as that division demands: the sequence
//! below contains no signature check, no use-count, no policy rule. Every one
//! of those lives in the door and is exercised by the door's fail-closed
//! matrix; what is tested here is only that the messages arrive in order and
//! that each refusal is *written to the peer* before the channel closes.
//!
//! ## Frames for the join, a stream for the application
//!
//! The join travels as carrier frames — four JSON messages and two binary
//! ones — because frames give the door's bytes-in/bytes-out functions their
//! message boundaries for free. Only after admission does
//! [`stream_over_frames`] start,
//! and the byte stream it presents carries the ordinary NDJSON session that
//! [`crate::session_loop::serve_admitted_session`] already speaks. The pump
//! starting *after* the admission reply is what keeps the plan's rule — a
//! refused stream never reaches Graphshell — structural rather than
//! disciplinary: before that point there is no stream for an application
//! byte to be on.
//!
//! ## The sequence
//!
//! ```text
//! browser                                   host
//!   |-- Open { invite, client_nonce } ------->|
//!   |<------ Challenge { server_nonce, sig } -|   host signs the transcript
//!   |   (browser verifies against the         |
//!   |    invitation's expected host key)      |
//!   |-- Redeem { subject, proof } ----------->|   one use spent, or Refused
//!   |<------------- Grant { delegation } -----|
//!   |-- hello bytes (binary frame) ---------->|   Notochord admission
//!   |<---------- reply bytes (binary frame) --|   accept, or deny and close
//!   |            ... NDJSON session ...       |
//! ```
//!
//! Position defines interpretation: the frame after `Grant` is hello bytes,
//! the frame after that is reply bytes, and neither is JSON. A peer that
//! sends anything else where a message is expected is refused as malformed.
//!
//! ## Where the other role went
//!
//! The peer half — [`peer_join`], [`peer_rejoin`], the wire messages, and
//! [`JoinFrames`] itself — lives in [`crate::webrtc_join`], with no runtime
//! beneath it, so the browser can run it. It is re-exported here, and the
//! tests below still drive both ends of the real sequence against each other,
//! which is what keeps the two from drifting.

use notochord::{
    AdmittedPrincipal, AdmittedSession, LocalNetworkPolicy, RevocationLedger, SessionHello,
};
use personae::IdentityProvider;
use tokio::io::DuplexStream;
use tokio::task::JoinHandle;
use webrtc_carrier::native::{Carrier, CarrierControl, PumpEnd, stream_over_frames};
use webrtc_carrier::{DtlsFingerprint, InviteId, InviteV1, LinkChallenge};

use crate::admission::PROJECTION_PROTOCOL;
use crate::webrtc_door::{
    RedemptionState, admit_webrtc_session, mint_delegation, redeem, sign_challenge,
    webrtc_session_facts,
};
use crate::webrtc_join::{ToHost, ToPeer, recv_binary, recv_json, send_json, unexpected_message};

/// The peer half and the shared vocabulary, re-exported from where they now
/// live so a caller that learned this module's name keeps working.
pub use crate::webrtc_join::{JoinError, JoinFrames, PeerJoin, peer_join, peer_rejoin};

/// One frame channel, whatever carries it.
///
/// [`Carrier`] is the real one. Tests use an in-memory pair, which is what
/// keeps the sequence's rules runnable on every `cargo test` rather than only
/// where a DTLS handshake can happen — frames-over-DTLS is already the
/// carrier suite's receipt, and composing the two is the fixture's.
impl JoinFrames for Carrier {
    async fn recv(&mut self) -> Result<Option<Vec<u8>>, String> {
        self.recv_frame().await.map_err(|error| error.to_string())
    }

    async fn send(&mut self, payload: &[u8]) -> Result<(), String> {
        self.send_frame(payload)
            .await
            .map_err(|error| error.to_string())
    }
}

/// The host's in-memory record of one outstanding invitation.
///
/// Holds a whole [`InviteV1`] — including its seed — which is exactly what the
/// door's storage note forbids *persisting*. This type is the reason that note
/// can be honest: the pairing lives in the accepting process for the life of
/// the offer and is never what a host writes down. A host that persists
/// invitations keeps [`RedemptionState`] plus the invitation's public terms,
/// and rebuilds this only in memory.
#[derive(Debug)]
pub struct HostedInvite {
    /// The host's own copy of the invitation. [`mint_delegation`] scopes the
    /// grant from this copy, never from anything the peer presented.
    pub invite: InviteV1,
    /// The verifier, use count and expiry the redemption spends against.
    pub redemption: RedemptionState,
}

/// Everything the host concluded from one completed join.
#[derive(Debug)]
pub struct JoinConclusion {
    /// Who was admitted, and under what.
    pub principal: AdmittedPrincipal,
    /// The verified claims behind the principal, re-decoded from the accepted
    /// hello exactly as `notochord::admit` re-decodes its own.
    pub claims: notochord::SessionClaims,
    /// The link both ends derived. Already inside the admission transcript;
    /// kept for receipts.
    pub shared_link: [u8; 16],
}

impl JoinConclusion {
    /// Assemble the admitted session over whatever stream now carries the
    /// application bytes.
    ///
    /// Split from [`host_join`] so the stream can be anything — the pumped
    /// carrier in production, a plain duplex in a test — without the join
    /// sequence caring.
    pub fn admitted_over<S>(self, stream: S, policy: &LocalNetworkPolicy) -> AdmittedSession<S> {
        AdmittedSession {
            stream,
            principal: self.principal,
            claims: self.claims,
            facts: webrtc_session_facts(self.shared_link),
            limits: policy.limits.clamped(),
        }
    }
}

/// Run the host side of the join over one connected channel.
///
/// `client_fingerprint` and `server_fingerprint` are this connection's, read
/// off the carrier after DTLS; `channel_label` likewise. A refusal is written
/// to the peer before this returns, so the caller only has to close the
/// channel, not explain it: an unknown invitation, a failed redemption, and an
/// admission denial, including an action the owner allows and this host does
/// not serve. A message out of sequence or a broken channel returns its error
/// unannounced.
#[allow(clippy::too_many_arguments)]
pub async fn host_join<P: IdentityProvider, F: JoinFrames>(
    frames: &mut F,
    provider: &P,
    hosted: &mut HostedInvite,
    channel_label: &str,
    client_fingerprint: DtlsFingerprint,
    server_fingerprint: DtlsFingerprint,
    policy: &LocalNetworkPolicy,
    ledger: &RevocationLedger,
    root_authority: [u8; 32],
    delegation_ttl_ms: u64,
    now_ms: u64,
    active_sessions: u32,
) -> Result<JoinConclusion, JoinError> {
    // Open: which invitation, and the client half of the transcript.
    let (invite_id, client_nonce) = match recv_json::<ToHost, _>(frames).await? {
        ToHost::Open {
            invite,
            client_nonce,
        } => (InviteId::from_bytes(invite), client_nonce),
        other => return Err(unexpected_message("Open", &other)),
    };
    if invite_id != hosted.invite.rendezvous() {
        // Named before any signature: an unknown invitation is not a secret,
        // and a peer probing ids learns only that this host is not offering
        // the one it guessed.
        refuse(frames, "unknown invitation").await;
        return Err(JoinError::UnknownInvite);
    }

    // Challenge: the host contributes its nonce and signs the transcript.
    let mut server_nonce = [0u8; 32];
    getrandom::fill(&mut server_nonce).expect("operating system random source");
    let challenge = LinkChallenge::new(
        PROJECTION_PROTOCOL,
        channel_label,
        invite_id,
        client_nonce,
        server_nonce,
        client_fingerprint,
        server_fingerprint,
    )?;
    let signature = sign_challenge(provider, &challenge)?;
    send_json(
        frames,
        &ToPeer::Challenge {
            server_nonce,
            signature,
        },
    )
    .await?;

    // Redeem, or resume. A first join spends a use and is granted a
    // delegation; a reconnecting peer already holds one and sends `Resume`,
    // spending nothing — its delegation is judged by admission below, where
    // revocation and expiry already live.
    match recv_json::<ToHost, _>(frames).await? {
        ToHost::Redeem { subject, proof } => {
            let proof: [u8; 64] = proof
                .try_into()
                .map_err(|_| JoinError::Malformed("the redemption proof is not 64 bytes".into()))?;
            if let Err(refusal) =
                redeem(&mut hosted.redemption, &challenge, &subject, &proof, now_ms)
            {
                refuse(frames, &refusal.to_string()).await;
                return Err(refusal.into());
            }
            // Grant: the delegation the redemption bought, scoped from the
            // host's own copy of the invitation.
            let mut nonce = [0u8; 32];
            getrandom::fill(&mut nonce).expect("operating system random source");
            let delegation = mint_delegation(
                provider,
                root_authority,
                subject,
                &hosted.invite,
                now_ms,
                delegation_ttl_ms,
                nonce,
            )?;
            send_json(frames, &ToPeer::Grant { delegation }).await?;
        },
        ToHost::Resume {} => {},
        other => return Err(unexpected_message("Redeem or Resume", &other)),
    }

    // Hello and reply: Notochord admission, bytes in and bytes out, exactly
    // as the door declares. The reply travels on the deny path too, whether the
    // owner's rule or the host's vocabulary denied — that is the "write the
    // refusal, then close" rule.
    let hello = recv_binary(frames, "a Notochord hello").await?;
    let shared_link = challenge.shared_link();
    let (reply, outcome) =
        admit_webrtc_session(policy, ledger, &hello, shared_link, now_ms, active_sessions);
    frames.send(&reply).await.map_err(JoinError::Channel)?;
    let principal = outcome.map_err(JoinError::Denied)?;

    // Post-accept re-decode, notochord's own precedent: `respond` accepted,
    // so this frame decoded once already, and a failure here is a bug rather
    // than a hostile frame.
    let claims = SessionHello::decode(&hello, &policy.limits.clamped())
        .map_err(|error| {
            JoinError::Malformed(format!("accepted hello did not re-decode: {error}"))
        })?
        .claims();

    Ok(JoinConclusion {
        principal,
        claims,
        shared_link,
    })
}

/// Best-effort: a refusal the peer never sees still refuses, so the send
/// error is not the story here.
async fn refuse<F: JoinFrames>(frames: &mut F, reason: &str) {
    let _ = send_json(
        frames,
        &ToPeer::Refused {
            reason: reason.to_string(),
        },
    )
    .await;
}

/// A served join: the admitted session, and the two handles that keep it
/// alive.
///
/// `control` is not optional bookkeeping. [`CarrierControl`] cancels the
/// driver when dropped — the deliberate dead-man's switch that stops a leaked
/// carrier from spinning forever — so a caller that lets it fall ends the
/// session it just admitted. It surfaced exactly that way: the first
/// composition test hung with both joins green, because `serve_webrtc_join`
/// originally dropped the control on return and the driver cancelled under a
/// perfectly admitted session.
#[derive(Debug)]
pub struct ServedJoin {
    /// The admitted session, over the pumped byte stream.
    pub session: AdmittedSession<DuplexStream>,
    /// The frame pump. Ends when the peer closes or the stream is dropped.
    pub pump: JoinHandle<PumpEnd>,
    /// The carrier's driver handle. Hold it for the session's life; drop it
    /// to cancel.
    pub control: CarrierControl,
}

impl ServedJoin {
    /// End a served session politely, flushing what the peer is still owed.
    ///
    /// The ordering is the whole function. The serve loop's final answer —
    /// `Closed`, or a refusal — may still be sitting in the duplex when the
    /// loop returns, so: drop the stream, which lets the pump drain those
    /// bytes into the carrier and then observe end-of-stream; wait for the
    /// pump to finish carrying; only then close the carrier, whose own close
    /// flushes the outbound queue to the wire before shutting down. Dropping
    /// a [`ServedJoin`] instead of calling this cancels the driver
    /// immediately, and the peer's last answer loses the race — found live,
    /// as a close reply that reproducibly never arrived.
    pub async fn finish(self) -> Result<(), JoinError> {
        drop(self.session);
        match self.pump.await {
            Ok(end) if end.is_clean() => {},
            Ok(end) => return Err(JoinError::Channel(format!("the pump ended badly: {end}"))),
            Err(join) => return Err(JoinError::Channel(format!("the pump panicked: {join}"))),
        }
        self.control
            .close()
            .await
            .map_err(|error| JoinError::Channel(format!("the carrier did not close: {error}")))
    }
}

/// Run the whole host side over a live carrier and hand back the session the
/// application serves.
///
/// The one function a host fixture needs: join over frames, then start the
/// pump and assemble the [`AdmittedSession`] over its stream. On a refusing
/// join the refusal has already been written; this closes the carrier and
/// returns the error.
#[allow(clippy::too_many_arguments)]
pub async fn serve_webrtc_join<P: IdentityProvider>(
    mut carrier: Carrier,
    provider: &P,
    hosted: &mut HostedInvite,
    policy: &LocalNetworkPolicy,
    ledger: &RevocationLedger,
    root_authority: [u8; 32],
    delegation_ttl_ms: u64,
    now_ms: u64,
    active_sessions: u32,
) -> Result<ServedJoin, JoinError> {
    let fingerprints = carrier
        .fingerprints()
        .ok_or_else(|| JoinError::Channel("the carrier has no DTLS fingerprints yet".into()))?;
    let channel_label = carrier.channel_label().to_string();

    let joined = host_join(
        &mut carrier,
        provider,
        hosted,
        &channel_label,
        *fingerprints.client(),
        *fingerprints.server(),
        policy,
        ledger,
        root_authority,
        delegation_ttl_ms,
        now_ms,
        active_sessions,
    )
    .await;

    let conclusion = match joined {
        Ok(conclusion) => conclusion,
        Err(error) => {
            // The refusal frame is already written; all that is left is to
            // stop carrying.
            let _ = carrier.close().await;
            return Err(error);
        },
    };

    let (reader, writer, control) = carrier.into_parts();
    let (stream, pump) = stream_over_frames(reader, writer);
    Ok(ServedJoin {
        session: conclusion.admitted_over(stream, policy),
        pump,
        control,
    })
}

#[cfg(test)]
mod tests;
