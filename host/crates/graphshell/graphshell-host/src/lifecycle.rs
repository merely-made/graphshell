// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! G5e: what happens to a projection session after it is admitted.
//!
//! Admission is a conclusion drawn once, at connect. Authority is not: a
//! certificate expires, an owner revokes a grant, a viewer reconnects. This
//! module is the join between the two, and it introduces no new vocabulary to
//! do it. `chirograph` already models every state involved
//! ([`SessionStatus`], [`CachePolicy`], [`IntentResult::Stale`]); what was
//! missing is anything that *drives* them from the authority the carrier
//! proved.
//!
//! The dependency wall holds: `graphshell-client` and `chirograph`
//! still declare no Personae and no `notochord`. The authority lives
//! here, in the application, and reaches the client only through the mutators
//! the client already exposed.
//!
//! ## A reconnect is a new admission when the link or the nonce is fresh
//!
//! [`projection_session`] derives the session id from
//! `AdmittedPrincipal::session_id`, the transcript digest over the client's
//! nonce, the subject, the transport identity and the shared link. A client
//! therefore cannot name the session it mounts under, and a reconnect over a
//! fresh link or with a fresh nonce lands on a different id, so cached scenes
//! from a previous admission are unreachable without a rule someone has to
//! enforce. Freshness comes from the carrier's link or the client: the WebRTC
//! carrier mixes a fresh host nonce into the link on every join, and on a
//! carrier without one a reused nonce lands on the same session. Admission
//! keeps no replay state. Resume within one admission stays exactly as G2
//! built it.
//!
//! ## Why the chain is retained
//!
//! [`AdmittedPrincipal`] is deliberately a conclusion and carries no claims,
//! so it cannot answer "is this still true?" later. [`AdmittedSession`]
//! retains the verified claims beside that conclusion, letting
//! [`SessionAuthority::retain_admitted`] preserve the chain for expiry and
//! revocation checks without decoding application bytes a second time.

use chirograph::{
    CachePolicy, IntentInvocation, IntentResult, ProjectionSession, Revision, SceneEpoch,
    SessionStatus,
};
use graphshell_client::ClientState;
use insigne::delegation::SignedDelegationCertificate;
use notochord::{
    AdmittedPrincipal, AdmittedSession, AuthorityLapse, RetainedAuthority, RevocationLedger,
};

use crate::admission::{CONNECT_ACTION, PROJECTION_SERVICE};

/// The projection session id for an admitted principal.
///
/// Derived from the transcript-bound `session_id` rather than chosen by the
/// client, so a cached scene cannot be re-mounted under an authority that did
/// not earn it.
///
/// Distinctness comes from the carrier's link or the client, and the responder
/// does not enforce it. `session_id` is a digest of the transcript, the
/// transcript carries the client's nonce and the shared link, and nothing
/// checks that either is fresh. Two admissions of the same subject reusing one
/// nonce over one link therefore land on the same projection session. That is
/// not an authority hole, because the transcript binds the subject and a peer
/// can only collide with itself, but a client mounting two projections in one
/// `ClientState` must mint per-session randomness or the second silently
/// replaces the first.
pub fn projection_session(principal: &AdmittedPrincipal) -> ProjectionSession {
    use base64::Engine;
    ProjectionSession(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(principal.session_id))
}

/// The service path a named score is disclosed under.
///
/// Scores hang below the service path so that Personae's scope grammar can
/// express "this viewer may see one score". The `/` boundary in
/// `path_covers` is what makes that safe: a grant for `spiral` does not reach
/// `spiral-admin`.
pub fn score_path(score: &str) -> String {
    format!("{PROJECTION_SERVICE}/{score}")
}

/// Graphshell's name for Notochord's retained-authority lapse.
pub type Lapse = AuthorityLapse;

/// The status a client may render for a retained-authority lapse.
fn lapse_status(lapse: Lapse) -> SessionStatus {
    match lapse {
        AuthorityLapse::Expired { .. } => SessionStatus::Expired,
        AuthorityLapse::Revoked => SessionStatus::Revoked,
    }
}

/// Why a score was not disclosed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScoreDenial {
    /// The session's authority has lapsed; nothing is disclosed under it.
    Lapsed(Lapse),
    /// The authority is live but does not reach this score.
    NotCovered { score: String },
}

/// A live session's retained authority.
///
/// Holds the chain the session was admitted on so the session can re-ask the
/// question later. Nothing here is serializable, for the same reason
/// `AdmittedPrincipal` is not: it is a local conclusion about a connection,
/// and a conclusion that travels is a claim.
#[derive(Clone, Debug)]
pub struct SessionAuthority {
    session: ProjectionSession,
    retained: RetainedAuthority,
}

/// The narrow identity and projection-session handoff an admitted endpoint
/// receives from its composing host.
///
/// This is deliberately not an admission proof or a bearer credential. It
/// carries neither delegation certificates nor a transport. The enclosing
/// [`SessionAuthority`] and session loop remain responsible for admission,
/// expiry, and revocation; a product endpoint uses this only to name the
/// already-admitted projection and attribute its own scoped work.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdmittedEndpointContext {
    session: ProjectionSession,
    subject: [u8; 32],
    application: Option<String>,
}

impl AdmittedEndpointContext {
    /// Construct one in-process composition handoff from already-verified
    /// facts. This constructor performs no authentication.
    pub fn new(session: ProjectionSession, subject: [u8; 32]) -> Self {
        Self {
            session,
            subject,
            application: None,
        }
    }

    /// This context, naming the first-party application the door admitted:
    /// the name its hello gave and the link transcript binds, never a later
    /// claim of the client's.
    pub fn with_application(mut self, application: impl Into<String>) -> Self {
        self.application = Some(application.into());
        self
    }

    /// The transcript-derived projection session the endpoint must serve.
    pub fn session(&self) -> &ProjectionSession {
        &self.session
    }

    /// The already-admitted public-key subject for product-scoped work.
    pub fn subject(&self) -> [u8; 32] {
        self.subject
    }

    /// The application the door admitted, for attributing its work. `None`
    /// where no door names one, such as the browser's.
    pub fn application(&self) -> Option<&str> {
        self.application.as_deref()
    }
}

/// Let a product endpoint bind itself to a session that Graphshell already
/// admitted.
///
/// Implementors must not treat [`AdmittedEndpointContext`] as a replacement
/// for carrier admission. It is a typed handoff inside a host that continues
/// to serve through [`SessionAuthority`].
pub trait BindAdmittedSession: Sized {
    fn bind_admitted_session(self, context: &AdmittedEndpointContext) -> Self;
}

impl SessionAuthority {
    /// Retain the authority carried by an admitted service session.
    ///
    /// This is the carrier path. The delegation chain comes from the verified
    /// claims retained by `notochord::admit_session`, so a long-lived
    /// session can notice a revocation after admission.
    pub fn retain_admitted<S>(session: &AdmittedSession<S>) -> Self {
        Self {
            session: projection_session(&session.principal),
            retained: RetainedAuthority::from_admitted(session),
        }
    }

    /// Retain the authority a session was admitted on.
    ///
    /// This remains useful to the sans-I/O path, where the caller already
    /// holds the verified hello. Carrier code should use
    /// [`Self::retain_admitted`] so it cannot accidentally discard the chain.
    pub fn retain(principal: AdmittedPrincipal, chain: Vec<SignedDelegationCertificate>) -> Self {
        Self {
            session: projection_session(&principal),
            retained: RetainedAuthority::new(principal, chain),
        }
    }

    /// The session id this authority admits, derived from the transcript.
    pub fn session(&self) -> &ProjectionSession {
        &self.session
    }

    /// Who was admitted, and for what.
    pub fn principal(&self) -> &AdmittedPrincipal {
        self.retained.principal()
    }

    /// Create the bounded product handoff for this already-admitted session.
    pub fn endpoint_context(&self) -> AdmittedEndpointContext {
        AdmittedEndpointContext::new(self.session.clone(), self.principal().subject)
    }

    /// When this session's authority runs out, if it does.
    ///
    /// The earliest expiry in the chain: a session lives no longer than the
    /// shortest-lived link in the authority that opened it.
    pub fn deadline_ms(&self) -> Option<u64> {
        self.retained.deadline_ms()
    }

    /// Whether the authority still holds, and if not, why.
    ///
    /// Revocation is checked before expiry: both can be true at once, and
    /// "the owner withdrew this" is the more useful thing to report.
    pub fn lapse(&self, ledger: &RevocationLedger, now_ms: u64) -> Option<Lapse> {
        self.retained.lapse(ledger, now_ms)
    }

    /// The status a client may render without inferring authority itself.
    pub fn status(&self, ledger: &RevocationLedger, now_ms: u64) -> SessionStatus {
        self.lapse(ledger, now_ms)
            .map_or(SessionStatus::Live, lapse_status)
    }

    /// Whether this authority reaches one named score.
    ///
    /// A grant scoped to the whole service discloses every score; a grant
    /// scoped to one score discloses only that one. Both are the same check,
    /// because the scope grammar already draws the line.
    pub fn authorize_score(
        &self,
        ledger: &RevocationLedger,
        now_ms: u64,
        score: &str,
    ) -> Result<(), ScoreDenial> {
        if let Some(lapse) = self.lapse(ledger, now_ms) {
            return Err(ScoreDenial::Lapsed(lapse));
        }
        let path = score_path(score);
        let covered = self.retained.covers(&path, CONNECT_ACTION, now_ms);
        if covered {
            Ok(())
        } else {
            Err(ScoreDenial::NotCovered {
                score: score.to_string(),
            })
        }
    }
}

/// Adjudicate one intent against both the authority and the scene.
///
/// Two ways to be stale, and they are answered in this order on purpose. An
/// intent from a lapsed session is refused outright: replying `Stale` with the
/// current revision would tell a peer whose authority has ended what the
/// current revision *is*, which is a disclosure the session no longer earns.
pub fn adjudicate_intent(
    authority: &SessionAuthority,
    ledger: &RevocationLedger,
    now_ms: u64,
    invocation: &IntentInvocation,
    current_epoch: SceneEpoch,
    current_revision: Revision,
) -> IntentResult {
    if invocation.session != *authority.session() {
        return IntentResult::Rejected {
            reason: "intent names a session this authority does not admit".to_string(),
        };
    }
    if let Some(lapse) = authority.lapse(ledger, now_ms) {
        return IntentResult::Rejected {
            reason: match lapse {
                Lapse::Expired { at_ms } => format!("session authority expired at {at_ms}"),
                Lapse::Revoked => "session authority was revoked".to_string(),
            },
        };
    }
    if invocation.observed_epoch != current_epoch
        || invocation.observed_revision != current_revision
    {
        return IntentResult::Stale {
            current_epoch,
            current_revision,
        };
    }
    IntentResult::Accepted
}

/// Apply a lapse to the client's mounted state.
///
/// Honours the mounted scene's own [`CachePolicy`]: `purge_on_revocation`
/// forgets the session outright, which drops both the scene and every resource
/// cached under it. Otherwise the scene stays mounted under a status that says
/// plainly it is no longer live, which is what an `EncryptedPersistent`
/// retention is for.
///
/// Returns whether the cache was purged.
pub fn apply_lapse(client: &mut ClientState, session: &ProjectionSession, lapse: Lapse) -> bool {
    let purge = client
        .mounted(session)
        .is_some_and(|mounted| purges_on(&mounted.cache_policy));
    if purge {
        client.forget_session(session);
    } else {
        match lapse {
            Lapse::Expired { .. } => client.mark_expired(session),
            Lapse::Revoked => client.mark_revoked(session),
        }
    }
    purge
}

/// Whether a cache policy purges when authority lapses.
///
/// `purge_on_revocation` is the owner's instruction about withdrawn authority.
/// An expiry is authority withdrawing on schedule rather than by hand, so it
/// purges under the same flag rather than a second one nobody would remember
/// to set.
fn purges_on(policy: &CachePolicy) -> bool {
    policy.purge_on_revocation
}

#[cfg(test)]
mod tests;
