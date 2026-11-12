use super::*;
use crate::admission::{CONNECT_ACTION, GRAPHSHELL_DOMAIN, PROJECTION_SERVICE};
use chirograph::{
    CapabilityProfile, CarrierRequestBody, EndpointDescriptor, IntentInvocation, IntentResult,
    ProjectionRequest, ProjectionSnapshot, ProtocolVersion, ResourceRequest, ResourceResponse,
    Revision, SceneEpoch, SessionOpen,
};
use insigne::delegation::{
    CapabilityScope, DelegationCertificate, DelegationParent, DelegationRevocation,
    SignedDelegationCertificate, SignedDelegationRevocation,
};
use notochord::{
    AdmittedPrincipal, CarrierKind, NetworkId, ProfileRef, RequestedAction, SessionClaims,
    SessionFacts, TrafficClass,
};
use personae::delegation::Issue;
use personae::{IdentityProvider, InMemoryProvider};
use sceno::InstanceId;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncBufReadExt, BufReader as TokioBufReader};

const NOW_MS: u64 = 50;
const EXPIRY_MS: u64 = 100;
const ROOT_AUTHORITY: [u8; 32] = [7; 32];
const NETWORK: [u8; 32] = [3; 32];

fn owner() -> InMemoryProvider {
    InMemoryProvider::from_seed([1; 32])
}

fn viewer() -> InMemoryProvider {
    InMemoryProvider::from_seed([4; 32])
}

/// The smallest endpoint satisfying the dispatcher's bounds. It never has
/// to answer: every test here is about the session plane, which is the
/// half `dispatch_common` declines.
struct StubEndpoint;

#[derive(Debug)]
struct StubError;

impl Display for StubError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "stub endpoint")
    }
}

impl ProjectionCatalog for StubEndpoint {
    fn describe(&self) -> EndpointDescriptor {
        EndpointDescriptor {
            label: "test".into(),
            projections: Vec::new(),
        }
    }
}

impl ProjectionSource for StubEndpoint {
    type Error = StubError;
    fn snapshot(&mut self, _: ProjectionRequest) -> Result<ProjectionSnapshot, StubError> {
        Err(StubError)
    }
}

impl PresentationSource for StubEndpoint {
    type Error = StubError;
    fn resource(&mut self, _: ResourceRequest) -> Result<ResourceResponse, StubError> {
        Err(StubError)
    }
}

impl IntentSink for StubEndpoint {
    type Error = StubError;
    fn invoke(&mut self, _: IntentInvocation) -> Result<IntentResult, StubError> {
        Err(StubError)
    }
}

fn grant() -> SignedDelegationCertificate {
    SignedDelegationCertificate::issue(
        &owner(),
        DelegationCertificate::new(
            DelegationParent::Root(ROOT_AUTHORITY),
            owner().master_public_key().to_bytes(),
            viewer().master_public_key().to_bytes(),
            CapabilityScope {
                domain: GRAPHSHELL_DOMAIN.into(),
                resource: NETWORK.to_vec(),
                path_prefix: PROJECTION_SERVICE.into(),
                actions: [CONNECT_ACTION.to_string()].into_iter().collect(),
            },
            5,
            10,
            Some(EXPIRY_MS),
            1,
            [1; 32],
        ),
    )
    .expect("issue certificate")
}

fn principal() -> AdmittedPrincipal {
    AdmittedPrincipal {
        subject: viewer().master_public_key().to_bytes(),
        class: TrafficClass::Interactive,
        session_id: [21; 32],
        action: RequestedAction {
            domain: GRAPHSHELL_DOMAIN.to_string(),
            path: PROJECTION_SERVICE.to_string(),
            action: CONNECT_ACTION.to_string(),
        },
    }
}

fn authority() -> SessionAuthority {
    SessionAuthority::retain(principal(), vec![grant()])
}

fn admitted<S>(stream: S) -> AdmittedSession<S> {
    AdmittedSession {
        stream,
        principal: principal(),
        claims: SessionClaims {
            wire_version: 1,
            network: NetworkId(NETWORK),
            profile: ProfileRef {
                id: "mere.base".into(),
                revision: 1,
            },
            action: principal().action,
            class: TrafficClass::Interactive,
            subject: viewer().master_public_key().to_bytes(),
            delegations: vec![grant()],
        },
        facts: SessionFacts::new(b"mere/graphshell/v1", CarrierKind::P2panda),
        limits: Default::default(),
    }
}

fn revoked() -> RevocationLedger {
    let mut ledger = RevocationLedger::new();
    let certificate = grant();
    let statement = SignedDelegationRevocation::issue(
        &owner(),
        DelegationRevocation::new(
            certificate.certificate.id(),
            owner().master_public_key().to_bytes(),
            certificate.certificate.scope.clone(),
            NOW_MS,
            [2; 32],
        ),
    )
    .expect("issue revocation");
    ledger.fold(statement.check().expect("an issued revocation checks"));
    ledger
}

fn request(id: u64, body: CarrierRequestBody) -> String {
    let mut line = serde_json::to_string(&carrier_request(id, body)).unwrap();
    line.push('\n');
    line
}

fn carrier_request(id: u64, body: CarrierRequestBody) -> CarrierRequest {
    CarrierRequest { id, body }
}

fn open_request(id: u64) -> String {
    request(
        id,
        CarrierRequestBody::Open(Box::new(SessionOpen {
            version: ProtocolVersion { major: 1, minor: 0 },
            capabilities: CapabilityProfile::default(),
        })),
    )
}

fn intent_request(id: u64) -> String {
    let authority = authority();
    request(
        id,
        CarrierRequestBody::Intent(IntentInvocation {
            session: authority.session().clone(),
            target: InstanceId(1),
            observed_epoch: SceneEpoch(1),
            observed_revision: Revision(1),
            intent: "fixture.inspect".to_string(),
            payload: Vec::new(),
        }),
    )
}

/// Drive the loop against `script` under `clock`, returning what the peer
/// read back and how the loop ended.
async fn run_with(
    script: &str,
    ledger: RevocationLedger,
    clock: impl Fn() -> u64,
) -> (Vec<CarrierResponse>, SessionSummary) {
    let (mut peer, server) = tokio::io::duplex(64 * 1024);
    let script = script.to_string();
    let peer_task = tokio::spawn(async move {
        peer.write_all(script.as_bytes()).await.unwrap();
        let mut lines = TokioBufReader::new(peer).lines();
        let mut out = Vec::new();
        while let Some(line) = lines.next_line().await.unwrap() {
            if !line.trim().is_empty() {
                out.push(serde_json::from_str::<CarrierResponse>(&line).unwrap());
            }
        }
        out
    });

    let mut session = admitted(server);
    let mut endpoint = StubEndpoint;
    let mut resume = |_: &mut StubEndpoint, _: ResumeRequest| Err("no resume".to_string());
    let summary = serve_admitted_session(
        &mut session,
        &authority(),
        &RwLock::new(ledger),
        &mut endpoint,
        &mut resume,
        clock,
    )
    .await
    .expect("loop");
    (peer_task.await.unwrap(), summary)
}

async fn run(script: &str, ledger: RevocationLedger) -> (Vec<CarrierResponse>, SessionSummary) {
    run_with(script, ledger, || NOW_MS).await
}

/// The inverse of stdio's refusal, and the reason this carrier exists: the
/// handshake proved the peer before a byte of this reached us.
#[tokio::test]
async fn open_is_answered_because_this_carrier_authenticated_its_peer() {
    let script = format!(
        "{}{}",
        open_request(1),
        request(2, CarrierRequestBody::Close)
    );
    let (responses, summary) = run(&script, RevocationLedger::default()).await;
    match &responses[0].body {
        Ok(CarrierResponseBody::Opened(opened)) => {
            assert_eq!(opened.status, SessionStatus::Live);
            assert_eq!(
                opened.expires_at_ms,
                Some(EXPIRY_MS),
                "the grant's deadline is offered as a renewal hint"
            );
        },
        other => panic!("expected Opened, got {other:?}"),
    }
    assert_eq!(summary.end, SessionEnd::Closed);
}

#[tokio::test]
async fn suspend_is_answered_because_this_session_can_reconnect() {
    let (responses, summary) = run(
        &request(3, CarrierRequestBody::Suspend),
        RevocationLedger::default(),
    )
    .await;
    assert_eq!(responses[0].body, Ok(CarrierResponseBody::Suspended));
    assert_eq!(summary.end, SessionEnd::Suspended);
}

#[tokio::test]
async fn close_is_terminal_and_nothing_after_it_is_served() {
    let script = format!(
        "{}{}",
        request(4, CarrierRequestBody::Close),
        open_request(5)
    );
    let (responses, summary) = run(&script, RevocationLedger::default()).await;
    assert_eq!(responses.len(), 1, "the request after Close is not served");
    assert_eq!(responses[0].body, Ok(CarrierResponseBody::Closed));
    assert_eq!(summary.end, SessionEnd::Closed);
}

/// Where G5e's rules stop being unit tests and start governing traffic.
#[tokio::test]
async fn a_revoked_grant_buys_no_further_requests() {
    let (responses, summary) = run(&open_request(6), revoked()).await;
    assert_eq!(responses.len(), 1);
    match &responses[0].body {
        Err(failure) => assert!(
            failure.message.contains("revoked"),
            "the peer is told why: {}",
            failure.message
        ),
        other => panic!("a revoked session must not be opened: {other:?}"),
    }
    assert_eq!(summary.end, SessionEnd::Lapsed(Lapse::Revoked));
}

/// G5 names the application verb explicitly. Keep a literal invocation at
/// the gate so a future request-loop rewrite cannot prove only `Open`.
#[tokio::test]
async fn a_revoked_grant_refuses_a_literal_intent_before_dispatch() {
    let (responses, summary) = run(&intent_request(7), revoked()).await;
    assert_eq!(responses.len(), 1);
    match &responses[0].body {
        Err(failure) => assert!(
            failure.message.contains("revoked"),
            "the intent refusal names the lapsed authority: {}",
            failure.message
        ),
        other => panic!("a revoked intent must not reach the endpoint: {other:?}"),
    }
    assert_eq!(summary.end, SessionEnd::Lapsed(Lapse::Revoked));
}

/// The check is per request, not per connection: a session that outlives
/// its grant stops at the next thing it asks for.
#[tokio::test]
async fn expiry_mid_session_stops_the_next_request() {
    let script = format!("{}{}", open_request(7), open_request(8));
    let calls = AtomicU64::new(0);
    let (responses, summary) = run_with(&script, RevocationLedger::default(), || {
        if calls.fetch_add(1, Ordering::SeqCst) == 0 {
            NOW_MS
        } else {
            EXPIRY_MS + 1
        }
    })
    .await;

    assert!(
        matches!(responses[0].body, Ok(CarrierResponseBody::Opened(_))),
        "the first request is inside the grant"
    );
    assert!(
        responses[1].body.is_err(),
        "the second is not, and is refused"
    );
    assert_eq!(
        summary.end,
        SessionEnd::Lapsed(Lapse::Expired { at_ms: EXPIRY_MS })
    );
}
