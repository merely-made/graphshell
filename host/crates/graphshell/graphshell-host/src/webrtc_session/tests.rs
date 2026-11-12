//! The join sequence with both roles live, over in-memory frames.
//!
//! The door's fail-closed matrix already proves every rule; the pump test
//! in `webrtc-carrier` already proves frames-over-DTLS become a byte
//! stream. What is proven here is the piece neither can: the two roles
//! agree on the sequence, a refusal is *told* to the peer, and a completed
//! join carries a real Graphshell session driven by the event-driven
//! adapter against the real serve loop.

use std::sync::RwLock;

use super::*;
use crate::carrier::projection_policy;
use crate::lifecycle::SessionAuthority;
use crate::resume::ResumeFixtureEndpoint;
use crate::session_loop::serve_admitted_session;
use crate::webrtc_door::RedemptionRefusal;
use crate::webrtc_door::{InviteTerms, issue_invite};
use graphshell_client::{Advance, Outcome, SessionDriver};
use graphshell_endpoint::ResumableProjectionSource;
use notochord::TrustedRoot;
use notochord::{NetworkId, ProfileRef};
use personae::InMemoryProvider;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;
use webrtc_carrier::{DTLS_FINGERPRINT_BYTES, FingerprintRole, ReleaseRefV1};

const NETWORK: NetworkId = NetworkId([3; 32]);
const ROOT_AUTHORITY: [u8; 32] = [7; 32];
const NOW_MS: u64 = 50;
const TTL_MS: u64 = 10_000;
const INVITE_EXPIRY_MS: u64 = 20_000;
const LABEL: &str = "mere-graphshell";

/// One direction of an in-memory frame channel.
struct MemFrames {
    tx: mpsc::Sender<Vec<u8>>,
    rx: mpsc::Receiver<Vec<u8>>,
}

impl JoinFrames for MemFrames {
    async fn recv(&mut self) -> Result<Option<Vec<u8>>, String> {
        Ok(self.rx.recv().await)
    }

    async fn send(&mut self, payload: &[u8]) -> Result<(), String> {
        self.tx
            .send(payload.to_vec())
            .await
            .map_err(|_| "the far end hung up".to_string())
    }
}

fn frame_pair() -> (MemFrames, MemFrames) {
    let (a_tx, a_rx) = mpsc::channel(8);
    let (b_tx, b_rx) = mpsc::channel(8);
    (
        MemFrames { tx: a_tx, rx: b_rx },
        MemFrames { tx: b_tx, rx: a_rx },
    )
}

fn fingerprints() -> (DtlsFingerprint, DtlsFingerprint) {
    (
        DtlsFingerprint::new(FingerprintRole::Client, [0xaa; DTLS_FINGERPRINT_BYTES]),
        DtlsFingerprint::new(FingerprintRole::Server, [0xbb; DTLS_FINGERPRINT_BYTES]),
    )
}

fn profile() -> ProfileRef {
    ProfileRef {
        id: "mere.base".into(),
        revision: 1,
    }
}

fn release() -> ReleaseRefV1 {
    ReleaseRefV1 {
        manifest_blake3: [0x5a; 32],
        publisher_key_id: [0x6b; 32],
    }
}

fn host() -> InMemoryProvider {
    InMemoryProvider::from_seed([1; 32])
}

fn policy() -> LocalNetworkPolicy {
    projection_policy(
        NETWORK,
        vec![TrustedRoot {
            authority: ROOT_AUTHORITY,
            issuer: host().master_public_key().to_bytes(),
        }],
        vec![profile()],
        None,
    )
}

/// Issue an invitation and split it the way a real join splits it: the
/// fragment travels, the host keeps its own copy plus the redemption.
fn issued() -> (HostedInvite, InviteV1) {
    let issue = issue_invite(
        &host(),
        &InviteTerms::projection(NETWORK, profile(), INVITE_EXPIRY_MS, 1, release()),
    )
    .expect("the invitation issues");
    let fragment = issue.descriptor.invite.to_fragment();
    let peer_copy = InviteV1::parse_fragment(&fragment).expect("the fragment parses");
    (
        HostedInvite {
            invite: issue.descriptor.invite,
            redemption: issue.redemption,
        },
        peer_copy,
    )
}

/// Run both roles to completion and return what each concluded.
async fn joined() -> (JoinConclusion, PeerJoin, InMemoryProvider) {
    let (mut host_frames, mut peer_frames) = frame_pair();
    let (client_fp, server_fp) = fingerprints();
    let (mut hosted, peer_invite) = issued();
    let host_provider = host();
    let ephemeral = InMemoryProvider::from_seed([9; 32]);
    let policy = policy();
    let ledger = RevocationLedger::new();
    let limits = policy.limits.clamped();

    let host_side = host_join(
        &mut host_frames,
        &host_provider,
        &mut hosted,
        LABEL,
        client_fp,
        server_fp,
        &policy,
        &ledger,
        ROOT_AUTHORITY,
        TTL_MS,
        NOW_MS,
        0,
    );
    let peer_side = peer_join(
        &mut peer_frames,
        &ephemeral,
        &peer_invite,
        LABEL,
        client_fp,
        server_fp,
        &limits,
    );
    let (host_end, peer_end) = tokio::join!(host_side, peer_side);
    (
        host_end.expect("the host admits"),
        peer_end.expect("the peer is admitted"),
        ephemeral,
    )
}

/// Reconnect is a new link with fresh admission and no second redemption.
///
/// The first join spends the invitation's only use. The rejoin runs on a
/// fresh channel with fresh fingerprints — a genuinely different DTLS link
/// — presenting the retained delegation, and is admitted even though the
/// invitation has nothing left to spend. That is C2's one-use ceiling and
/// C3's fresh-admission rule holding at the same time.
#[tokio::test]
async fn a_rejoin_reuses_the_delegation_and_spends_nothing() {
    let (mut host_frames, mut peer_frames) = frame_pair();
    let (client_fp, server_fp) = fingerprints();
    let (mut hosted, peer_invite) = issued();
    let host_provider = host();
    let ephemeral = InMemoryProvider::from_seed([9; 32]);
    let policy = policy();
    let ledger = RevocationLedger::new();
    let limits = policy.limits.clamped();

    // First join, spending the one use.
    let first = {
        let host_side = host_join(
            &mut host_frames,
            &host_provider,
            &mut hosted,
            LABEL,
            client_fp,
            server_fp,
            &policy,
            &ledger,
            ROOT_AUTHORITY,
            TTL_MS,
            NOW_MS,
            0,
        );
        let peer_side = peer_join(
            &mut peer_frames,
            &ephemeral,
            &peer_invite,
            LABEL,
            client_fp,
            server_fp,
            &limits,
        );
        let (host_end, peer_end) = tokio::join!(host_side, peer_side);
        host_end.expect("the first join admits");
        peer_end.expect("the peer is admitted")
    };
    assert_eq!(hosted.redemption.remaining_uses(), 0, "the use is spent");

    // A new link: new frames, new fingerprints, same delegation.
    let (mut host_frames, mut peer_frames) = frame_pair();
    let client_fp =
        DtlsFingerprint::new(FingerprintRole::Client, [0xcc; DTLS_FINGERPRINT_BYTES]);
    let server_fp =
        DtlsFingerprint::new(FingerprintRole::Server, [0xdd; DTLS_FINGERPRINT_BYTES]);

    let host_side = host_join(
        &mut host_frames,
        &host_provider,
        &mut hosted,
        LABEL,
        client_fp,
        server_fp,
        &policy,
        &ledger,
        ROOT_AUTHORITY,
        TTL_MS,
        NOW_MS,
        0,
    );
    let peer_side = peer_rejoin(
        &mut peer_frames,
        &ephemeral,
        &peer_invite,
        first.delegation,
        LABEL,
        client_fp,
        server_fp,
        &limits,
    );
    let (host_end, peer_end) = tokio::join!(host_side, peer_side);
    let conclusion = host_end.expect("the rejoin admits over the spent invitation");
    let rejoined = peer_end.expect("the peer is admitted again");

    assert_eq!(
        hosted.redemption.remaining_uses(),
        0,
        "a rejoin spends nothing and cannot go negative"
    );
    assert_ne!(
        first.shared_link, rejoined.shared_link,
        "a new link is a new link — different fingerprints, different transcript"
    );
    assert_eq!(conclusion.principal.session_id, rejoined.session_id);
    assert_eq!(
        conclusion.principal.subject,
        ephemeral.master_public_key().to_bytes(),
        "the same subject, admitted afresh"
    );
}

/// The positive control: both roles, one sequence, agreeing conclusions.
#[tokio::test]
async fn a_join_over_frames_is_admitted_end_to_end() {
    let (conclusion, peer, ephemeral) = joined().await;
    assert_eq!(
        conclusion.principal.subject,
        ephemeral.master_public_key().to_bytes(),
        "the admitted subject is the ephemeral key the peer generated"
    );
    assert_eq!(
        conclusion.principal.session_id, peer.session_id,
        "both ends name the same transcript"
    );
    assert_eq!(
        conclusion.shared_link, peer.shared_link,
        "both ends derived the same link"
    );
}

/// A second redemption of a one-use invitation is refused, and the peer is
/// told rather than left watching a dead channel.
#[tokio::test]
async fn a_spent_invitation_refuses_and_says_so() {
    let (mut host_frames, mut peer_frames) = frame_pair();
    let (client_fp, server_fp) = fingerprints();
    let (mut hosted, peer_invite) = issued();
    // Spend the single use before the join.
    hosted.redemption = RedemptionState::new(hosted.redemption.verifier(), 0, INVITE_EXPIRY_MS);
    let host_provider = host();
    let ephemeral = InMemoryProvider::from_seed([9; 32]);
    let policy = policy();
    let ledger = RevocationLedger::new();
    let limits = policy.limits.clamped();

    let host_side = host_join(
        &mut host_frames,
        &host_provider,
        &mut hosted,
        LABEL,
        client_fp,
        server_fp,
        &policy,
        &ledger,
        ROOT_AUTHORITY,
        TTL_MS,
        NOW_MS,
        0,
    );
    let peer_side = peer_join(
        &mut peer_frames,
        &ephemeral,
        &peer_invite,
        LABEL,
        client_fp,
        server_fp,
        &limits,
    );
    let (host_end, peer_end) = tokio::join!(host_side, peer_side);
    assert!(
        matches!(
            host_end,
            Err(JoinError::Redemption(RedemptionRefusal::Exhausted))
        ),
        "the host records the exhaustion: {host_end:?}"
    );
    let refused = peer_end.expect_err("the peer is refused");
    assert!(
        refused.to_string().contains("no remaining uses"),
        "the peer is told why: {refused}"
    );
}

/// A host that cannot prove the invitation's key gets no redemption proof.
///
/// The peer walks away after the challenge, so the invitation's use count
/// is never spent — which is the property that makes a relay-in-the-middle
/// unable to exhaust an invitation it cannot sign for.
#[tokio::test]
async fn a_wrong_host_is_refused_before_any_secret_crosses() {
    let (mut host_frames, peer_frames) = frame_pair();
    let (client_fp, server_fp) = fingerprints();
    let (mut hosted, peer_invite) = issued();
    let ephemeral = InMemoryProvider::from_seed([9; 32]);
    let policy = policy();
    let ledger = RevocationLedger::new();
    let limits = policy.limits.clamped();
    // The signer is not the host the invitation names.
    let impostor = InMemoryProvider::from_seed([2; 32]);

    // The peer runs in its own task and *drops its channel* on return —
    // walking away is the refusal, and the host has to cope with a peer
    // that says nothing more, not one that politely explains.
    let peer = tokio::spawn(async move {
        let mut frames = peer_frames;
        peer_join(
            &mut frames,
            &ephemeral,
            &peer_invite,
            LABEL,
            client_fp,
            server_fp,
            &limits,
        )
        .await
    });
    let host_end = host_join(
        &mut host_frames,
        &impostor,
        &mut hosted,
        LABEL,
        client_fp,
        server_fp,
        &policy,
        &ledger,
        ROOT_AUTHORITY,
        TTL_MS,
        NOW_MS,
        0,
    )
    .await;
    let peer_end = peer.await.expect("the peer task does not panic");
    assert!(
        matches!(peer_end, Err(JoinError::HostUnverified)),
        "the peer refuses the channel: {peer_end:?}"
    );
    // The peer hung up without redeeming, so the host ends on the closed
    // channel — and critically, the use count is intact.
    assert!(
        matches!(host_end, Err(JoinError::Channel(_))),
        "{host_end:?}"
    );
    assert_eq!(
        hosted.redemption.remaining_uses(),
        1,
        "an unverified host cannot cost the invitation anything"
    );
}

/// The flagship: a completed join carries a real Graphshell session.
///
/// The host end is the real `serve_admitted_session` over a real
/// `ResumeFixtureEndpoint`; the peer end is the event-driven
/// `SessionDriver` — the browser's adapter — speaking NDJSON lines. This
/// is every C4a seam except DTLS itself composed in one test: join,
/// admission, authority, serve loop, discovery, mount, close.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_completed_join_serves_a_real_session_to_the_event_driver() {
    let (conclusion, _peer, _ephemeral) = joined().await;
    let policy = policy();

    let (host_stream, peer_stream) = tokio::io::duplex(64 * 1024);
    let mut admitted = conclusion.admitted_over(host_stream, &policy);
    let authority = SessionAuthority::retain_admitted(&admitted);
    let revocations = RwLock::new(RevocationLedger::new());
    let mut endpoint = ResumeFixtureEndpoint::new();

    let serving = async {
        let mut resume = |endpoint: &mut ResumeFixtureEndpoint, request| {
            ResumableProjectionSource::resume(endpoint, request)
                .map_err(|error| error.to_string())
        };
        serve_admitted_session(
            &mut admitted,
            &authority,
            &revocations,
            &mut endpoint,
            &mut resume,
            || NOW_MS,
        )
        .await
    };

    let driving = async {
        let (read, mut write) = tokio::io::split(peer_stream);
        let mut lines = BufReader::new(read).lines();
        let mut driver = SessionDriver::new(chirograph::CapabilityProfile::default());

        // Carry one Advance to the wire, then feed lines back until the
        // operation concludes — the event-driven loop a browser runs, with
        // a duplex where the data channel will be.
        async fn drive<R, W>(
            driver: &mut SessionDriver,
            write: &mut W,
            lines: &mut tokio::io::Lines<BufReader<R>>,
            start: Advance,
        ) -> Result<Outcome, String>
        where
            R: tokio::io::AsyncRead + Unpin,
            W: tokio::io::AsyncWrite + Unpin,
        {
            let mut advance = start;
            loop {
                match advance {
                    Advance::Done(outcome) => return Ok(outcome),
                    Advance::Noted => {
                        let line = lines
                            .next_line()
                            .await
                            .map_err(|error| error.to_string())?
                            .ok_or("the session ended mid-operation")?;
                        advance = driver.on_line(&line)?;
                    },
                    Advance::Send(line) => {
                        write
                            .write_all(line.as_bytes())
                            .await
                            .map_err(|error| error.to_string())?;
                        write
                            .write_all(b"\n")
                            .await
                            .map_err(|error| error.to_string())?;
                        let line = lines
                            .next_line()
                            .await
                            .map_err(|error| error.to_string())?
                            .ok_or("the session ended mid-operation")?;
                        advance = driver.on_line(&line)?;
                    },
                }
            }
        }

        let start = driver.discover()?;
        let discovered = drive(&mut driver, &mut write, &mut lines, start).await?;
        let Outcome::Descriptor(descriptor) = discovered else {
            return Err(format!("expected a descriptor, got {discovered:?}"));
        };
        assert_eq!(descriptor.projections.len(), 1, "the fixture offers one");

        let start = driver
            .core_mut()
            .ok_or("no core after discovery")?
            .mount(0)?;
        let start = driver.begin(start)?;
        let mounted = drive(&mut driver, &mut write, &mut lines, start).await?;
        let Outcome::Mounted(session) = mounted else {
            return Err(format!("expected a mount, got {mounted:?}"));
        };
        assert!(
            driver
                .core()
                .and_then(|core| core.client().mounted(&session))
                .is_some(),
            "the driver's client state holds the mounted scene"
        );

        let close = driver.core_mut().ok_or("no core")?.close();
        let close = driver.begin(close)?;
        let closed = drive(&mut driver, &mut write, &mut lines, close).await?;
        assert!(matches!(closed, Outcome::Closed));
        Ok::<(), String>(())
    };

    let (summary, drove) = tokio::join!(serving, driving);
    drove.expect("the event driver completes discovery, mount, and close");
    let summary = summary.expect("the serve loop ends without a session error");
    assert_eq!(
        summary.answered, 3,
        "discover, snapshot, close — and nothing else"
    );
}
