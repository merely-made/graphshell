use personae::delegation::Issue;
use std::sync::OnceLock;

use super::*;
use crate::admission::{CONNECT_ACTION, GRAPHSHELL_DOMAIN, connect_action, open_session};
use insigne::delegation::{
    CapabilityScope, DelegationCertificate, DelegationParent, SignedDelegationCertificate,
};
use notochord::{CarrierKind, RequestedAction, SessionReply, TrafficClass, initiate_session};
use personae::IdentityProvider;
use personae::InMemoryProvider;
use tokio::io::AsyncReadExt;
use transport::memory::MemoryTransport;
use transport::{
    IngressContext, P2pandaTransport, PeerID, initiator_binding, initiator_link_binding,
};

const NETWORK: NetworkId = NetworkId([3; 32]);
const ROOT_AUTHORITY: [u8; 32] = [7; 32];
const NOW_MS: u64 = 50;

fn p2panda_receipt_lock() -> &'static tokio::sync::Mutex<()> {
    static LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
}

fn owner() -> InMemoryProvider {
    InMemoryProvider::from_seed([1; 32])
}

fn viewer() -> InMemoryProvider {
    InMemoryProvider::from_seed([4; 32])
}

fn profile_ref() -> ProfileRef {
    ProfileRef {
        id: "mere.base".into(),
        revision: 1,
    }
}

/// A grant from the owner letting `subject` perform `action` under one
/// service triple.
fn grant_for(
    subject: [u8; 32],
    domain: &str,
    path: &str,
    action: &str,
) -> SignedDelegationCertificate {
    SignedDelegationCertificate::issue(
        &owner(),
        DelegationCertificate::new(
            DelegationParent::Root(ROOT_AUTHORITY),
            owner().master_public_key().to_bytes(),
            subject,
            CapabilityScope {
                domain: domain.into(),
                resource: NETWORK.0.to_vec(),
                path_prefix: path.into(),
                actions: [action.to_string()].into_iter().collect(),
            },
            5,
            10,
            Some(100),
            1,
            [1; 32],
        ),
    )
    .expect("issue certificate")
}

/// A grant from the owner letting `subject` perform `action` at the
/// projection service.
fn grant(subject: [u8; 32], action: &str) -> SignedDelegationCertificate {
    grant_for(subject, GRAPHSHELL_DOMAIN, PROJECTION_SERVICE, action)
}

fn policy() -> LocalNetworkPolicy {
    projection_policy(
        NETWORK,
        vec![TrustedRoot {
            authority: ROOT_AUTHORITY,
            issuer: owner().master_public_key().to_bytes(),
        }],
        vec![profile_ref()],
        None,
    )
}

/// Drive one session over the paired memory carrier and return what the
/// responder decided beside the reply the client read.
///
/// The client half holds its stream until the responder has decided, which
/// is what a real client does and what the module docs require of a
/// service: neither side may drop the connection out from under a reply
/// that has been written.
/// `Ok(action)` means the session was served under that action.
async fn run(action: &str) -> (Result<String, ProjectionRefusal>, SessionReply) {
    let viewer = viewer();
    let subject = viewer.master_public_key().to_bytes();
    // The memory carrier authenticates its counterparty by construction,
    // so policy rule D6 requires the claimed subject to *be* the peer the
    // carrier proved. Giving the client node the viewer's own key is what
    // makes this an honest fixture rather than one that dodges the rule.
    let client_peer = PeerID::from_bytes(&subject).expect("client peer");
    let server_peer =
        PeerID::from_bytes(&owner().master_public_key().to_bytes()).expect("server peer");
    let (server, client) = MemoryTransport::pair(server_peer, client_peer);

    let mut requested = connect_action();
    requested.action = action.to_string();
    let delegations = vec![grant(subject, action)];
    let action_owned = action.to_string();
    let mut serving_policy = policy();
    serving_policy
        .services
        .get_mut(PROJECTION_SERVICE)
        .expect("projection rule")
        .actions
        .insert(action.to_string());

    let client_task = tokio::spawn(async move {
        let mut stream = client
            .connect(server_peer, projection_alpn())
            .await
            .expect("dial");
        let binding = initiator_binding(&projection_alpn(), client_peer);
        // The connect case goes through Graphshell's own helper, because
        // that is the path a real viewer takes. The other case cannot:
        // `open_session` fixes the action to `connect`, and editing a
        // signed hello afterwards would invalidate it. A peer asking for
        // something else builds its own hello, which is exactly what the
        // open action vocabulary allows and what this test needs to be
        // honest about.
        let hello = if action_owned == CONNECT_ACTION {
            open_session(
                &viewer,
                NETWORK,
                profile_ref(),
                TrafficClass::Interactive,
                [5; 32],
                &binding,
                delegations,
            )
        } else {
            SessionHello::issue(
                &viewer,
                NETWORK,
                profile_ref(),
                RequestedAction {
                    domain: GRAPHSHELL_DOMAIN.into(),
                    path: PROJECTION_SERVICE.into(),
                    action: action_owned,
                },
                TrafficClass::Interactive,
                [5; 32],
                &binding,
                delegations,
            )
        }
        .expect("issue hello");
        let reply = initiate_session(&mut stream, &hello, &policy().limits.clamped())
            .await
            .expect("the reply");
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        reply
    });

    let outcome = accept_projection_session(
        &server,
        &serving_policy,
        &RevocationLedger::default(),
        NOW_MS,
        0,
    )
    .await
    .expect("accept path");
    let reply = client_task.await.expect("client task");

    (
        outcome.map(|session| session.principal.action.action.clone()),
        reply,
    )
}

#[tokio::test]
async fn a_connect_grant_is_admitted_and_served() {
    let (outcome, reply) = run(CONNECT_ACTION).await;
    assert_eq!(outcome.expect("must be served"), CONNECT_ACTION);
    assert!(reply.is_accept(), "the client is told it was admitted");
}

/// Even when the owner document allows an action, the service remains
/// authoritative about what its implementation serves, and the client is
/// told so in the reply rather than seeing an accept and then a close.
#[tokio::test]
async fn an_admitted_action_this_service_does_not_serve_is_refused() {
    let (outcome, reply) = run("administer").await;
    assert_eq!(
        outcome.expect_err("must be refused"),
        ProjectionRefusal::NotAdmitted(DenyReason::ActionNotOffered)
    );
    assert_eq!(
        reply,
        SessionReply::Reject {
            reason: DenyReason::ActionNotOffered
        }
    );
}

/// C0's acceptance fixture: a listener-only carrier reaches admission.
///
/// The WebRTC lane has no bilateral `Transport` to accept from, so it
/// builds its own [`AcceptedSession`] and enters at
/// [`admit_accepted_session`]. Two facts make it the honest shape rather
/// than a p2panda fixture wearing a different label: the carrier
/// authenticates nobody, so `peer` is `None`, and the subject is instead
/// bound to the link the two ends derived from the host challenge. That is
/// the same binding Reticulum uses, which is the point — WebRTC needs no
/// new proof grammar, only a link to put in the existing one.
///
/// The link is derived through `webrtc-carrier` rather than pasted, so a
/// change to the transcript encoding fails here too.
#[tokio::test]
async fn a_webrtc_session_is_admitted_with_no_authenticated_peer() {
    use webrtc_carrier::{DtlsFingerprint, FingerprintRole, InviteId, LinkChallenge};

    let challenge = LinkChallenge::new(
        PROJECTION_PROTOCOL,
        "mere-graphshell",
        InviteId::from_bytes([9; 16]),
        [0x11; 32],
        [0x22; 32],
        DtlsFingerprint::new(FingerprintRole::Client, [0xAA; 32]),
        DtlsFingerprint::new(FingerprintRole::Server, [0xBB; 32]),
    )
    .expect("link challenge");
    let shared_link = challenge.shared_link();
    assert_ne!(
        shared_link, [0u8; 16],
        "a derived link of all zeroes would make the binding vacuous"
    );

    let viewer = viewer();
    let subject = viewer.master_public_key().to_bytes();
    let delegations = vec![grant(subject, CONNECT_ACTION)];
    let (client_half, server_half) = tokio::io::duplex(64 * 1024);

    let client_task = tokio::spawn(async move {
        let mut stream = client_half;
        // No transport identity to bind, so the shared link carries the
        // weight. `initiator_binding` would be a lie here: nothing
        // authenticated this peer.
        let binding = initiator_link_binding(&projection_alpn(), shared_link);
        let hello = open_session(
            &viewer,
            NETWORK,
            profile_ref(),
            TrafficClass::Interactive,
            [5; 32],
            &binding,
            delegations,
        )
        .expect("issue hello");
        let _ = initiate_session(&mut stream, &hello, &policy().limits.clamped()).await;
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    });

    let accepted = AcceptedSession::new(
        server_half,
        projection_alpn(),
        // The carrier authenticated no initiator, and says so.
        None,
        IngressContext::webrtc(shared_link),
    );
    assert!(
        !accepted.is_transport_authenticated(),
        "the WebRTC carrier must never look authenticated"
    );
    let facts = accepted.session_facts();
    assert_eq!(facts.authenticated_initiator, None);
    assert_eq!(facts.ingress.shared_link, Some(shared_link));
    assert_eq!(facts.ingress.local_interface, None);

    let outcome =
        admit_accepted_session(accepted, &policy(), &RevocationLedger::default(), NOW_MS, 0)
            .await
            .expect("admission path");
    client_task.abort();

    let session = outcome.expect("the webrtc session must be admitted");
    assert_eq!(session.principal.action.action, CONNECT_ACTION);
}

async fn p2panda_pair() -> (P2pandaTransport, P2pandaTransport, PeerID, PeerID) {
    let client = P2pandaTransport::builder_from_seed_ref(&viewer().master_keypair().to_seed())
        .alpns(vec![projection_alpn()])
        .bind()
        .await
        .expect("bind projection client");
    let server = P2pandaTransport::builder_from_seed_ref(&owner().master_keypair().to_seed())
        .alpns(vec![projection_alpn()])
        .bind()
        .await
        .expect("bind projection server");
    let client_peer = client.local_peer_id();
    let server_peer = server.local_peer_id();

    let client_addr =
        tokio::time::timeout(std::time::Duration::from_secs(10), client.endpoint_addr())
            .await
            .expect("client endpoint address timeout")
            .expect("client endpoint address");
    let server_addr =
        tokio::time::timeout(std::time::Duration::from_secs(10), server.endpoint_addr())
            .await
            .expect("server endpoint address timeout")
            .expect("server endpoint address");
    client
        .add_peer(server_addr)
        .await
        .expect("client registers server");
    server
        .add_peer(client_addr)
        .await
        .expect("server registers client");

    (server, client, server_peer, client_peer)
}

/// The literal G5d receipt. The first carrier tests exercised the generic
/// accept path through `MemoryTransport`; this one proves the same path
/// against p2panda-net's authenticated Iroh connection and carries
/// application bytes only after admission.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn p2panda_admits_the_authenticated_viewer_before_application_bytes() {
    let _receipt_guard = p2panda_receipt_lock().lock().await;
    const APPLICATION_BYTES: &[u8] = b"graphshell-session-open-may-start";

    let viewer = viewer();
    let subject = viewer.master_public_key().to_bytes();
    let (server, client, server_peer, client_peer) = p2panda_pair().await;
    let (server_finished_tx, server_finished_rx) = tokio::sync::oneshot::channel();
    assert_eq!(
        client_peer.to_bytes(),
        subject,
        "the carrier identity is the Personae subject that signs the hello"
    );

    let client_task = tokio::spawn(async move {
        let alpn = projection_alpn();
        let mut stream = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            client.connect(server_peer, alpn.clone()),
        )
        .await
        .expect("projection dial timeout")
        .expect("projection dial");
        let binding = initiator_binding(&alpn, client_peer);
        let hello = open_session(
            &viewer,
            NETWORK,
            profile_ref(),
            TrafficClass::Interactive,
            [5; 32],
            &binding,
            vec![grant(subject, CONNECT_ACTION)],
        )
        .expect("issue projection hello");
        let reply = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            initiate_session(&mut stream, &hello, &policy().limits.clamped()),
        )
        .await
        .expect("projection handshake timeout")
        .expect("projection handshake");
        assert!(reply.is_accept(), "the initiator sees admission");

        let mut application = vec![0; APPLICATION_BYTES.len()];
        tokio::time::timeout(
            std::time::Duration::from_secs(10),
            stream.read_exact(&mut application),
        )
        .await
        .expect("application read timeout")
        .expect("application read");
        let _ = server_finished_rx.await;
        application
    });

    let mut session = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        accept_projection_session(&server, &policy(), &RevocationLedger::default(), NOW_MS, 0),
    )
    .await
    .expect("projection accept timeout")
    .expect("projection accept path")
    .expect("projection session admitted");
    assert_eq!(session.principal.subject, subject);
    assert_eq!(session.facts.transport, CarrierKind::P2panda);
    assert_eq!(
        session.facts.authenticated_initiator,
        Some(client_peer.to_bytes()),
        "the admitted session retains the peer p2panda authenticated"
    );
    let retained = crate::lifecycle::SessionAuthority::retain_admitted(&session);
    assert_eq!(
        retained.deadline_ms(),
        Some(100),
        "the carrier also retains the verified chain needed for later revocation checks"
    );
    session
        .stream
        .write_all(APPLICATION_BYTES)
        .await
        .expect("application write");
    session.stream.shutdown().await.expect("finish application");
    let _ = server_finished_tx.send(());
    let principal = session.principal;

    let application = tokio::time::timeout(std::time::Duration::from_secs(10), client_task)
        .await
        .expect("projection client timeout")
        .expect("projection client task");
    assert_eq!(application, APPLICATION_BYTES);
    assert_eq!(principal.action, connect_action());
}

/// The same real carrier must preserve the cross-service boundary. The
/// grant is valid and belongs to the authenticated peer, but Murm
/// authority cannot open Graphshell and not one projection byte follows
/// the denial.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn p2panda_murm_grant_is_refused_before_projection_bytes() {
    let _receipt_guard = p2panda_receipt_lock().lock().await;
    let viewer = viewer();
    let subject = viewer.master_public_key().to_bytes();
    let (server, client, server_peer, client_peer) = p2panda_pair().await;

    let client_task = tokio::spawn(async move {
        let alpn = projection_alpn();
        let mut stream = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            client.connect(server_peer, alpn.clone()),
        )
        .await
        .expect("projection dial timeout")
        .expect("projection dial");
        let binding = initiator_binding(&alpn, client_peer);
        let hello = open_session(
            &viewer,
            NETWORK,
            profile_ref(),
            TrafficClass::Interactive,
            [5; 32],
            &binding,
            vec![grant_for(
                subject,
                "mere.network",
                "/services/murm",
                CONNECT_ACTION,
            )],
        )
        .expect("issue projection hello with foreign grant");
        let reply = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            initiate_session(&mut stream, &hello, &policy().limits.clamped()),
        )
        .await
        .expect("projection refusal timeout")
        .expect("projection refusal");
        assert!(!reply.is_accept(), "the initiator sees refusal");

        let mut application = Vec::new();
        tokio::time::timeout(
            std::time::Duration::from_secs(10),
            stream.read_to_end(&mut application),
        )
        .await
        .expect("refused stream close timeout")
        .expect("read refused stream");
        application
    });

    let refusal = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        accept_projection_session(&server, &policy(), &RevocationLedger::default(), NOW_MS, 0),
    )
    .await
    .expect("projection accept timeout")
    .expect("projection accept path")
    .expect_err("Murm authority must not open Graphshell");
    let application = tokio::time::timeout(std::time::Duration::from_secs(10), client_task)
        .await
        .expect("projection client timeout")
        .expect("projection client task");
    assert!(
        application.is_empty(),
        "a refusal exposes no application bytes"
    );
    assert_eq!(
        refusal,
        ProjectionRefusal::NotAdmitted(DenyReason::ActionNotCovered)
    );
}
