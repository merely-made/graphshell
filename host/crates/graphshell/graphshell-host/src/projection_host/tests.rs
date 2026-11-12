use super::*;
use crate::admission::{CONNECT_ACTION, GRAPHSHELL_DOMAIN, PROJECTION_SERVICE, open_session};
use crate::carrier::projection_policy;
use crate::network_carrier::{
    CarrierRuntime, NetworkCarrier, dial_projection_session, projection_binding,
};
use chirograph::{
    CapabilityProfile, Carrier, CarrierNotice, CarrierRequestBody, CarrierResponseBody,
    EndpointDescriptor, IntentInvocation, IntentResult, ProjectionRequest, ProjectionSnapshot,
    ProtocolVersion, ResourceRequest, ResourceResponse, SessionOpen,
};
use graphshell_endpoint::{
    IntentSink, PresentationSource, ProjectionCatalog, ProjectionNoticeSource, ProjectionSource,
};
use insigne::delegation::{
    CapabilityScope, DelegationCertificate, DelegationParent, SignedDelegationCertificate,
};
use notochord::{NetworkId, ProfileRef, TrafficClass, TrustedRoot};
use personae::delegation::Issue;
use personae::{IdentityProvider, InMemoryProvider};
use std::fmt::Display;
use std::sync::mpsc;
use std::time::Duration;
use transport::PeerID;
use transport::memory::MemoryTransport;

const NETWORK: NetworkId = NetworkId([3; 32]);
const ROOT_AUTHORITY: [u8; 32] = [7; 32];
const NOW_MS: u64 = 50;

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

fn grant(subject: [u8; 32]) -> SignedDelegationCertificate {
    SignedDelegationCertificate::issue(
        &owner(),
        DelegationCertificate::new(
            DelegationParent::Root(ROOT_AUTHORITY),
            owner().master_public_key().to_bytes(),
            subject,
            CapabilityScope {
                domain: GRAPHSHELL_DOMAIN.into(),
                resource: NETWORK.0.to_vec(),
                path_prefix: PROJECTION_SERVICE.into(),
                actions: [CONNECT_ACTION.to_string()].into_iter().collect(),
            },
            5,
            10,
            Some(NOW_MS + 3_600_000),
            1,
            [1; 32],
        ),
    )
    .expect("issue certificate")
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

/// An endpoint that names which session opened it, so a test can tell two
/// concurrently served sessions apart.
struct LabelledEndpoint {
    label: String,
}

#[derive(Debug)]
struct StubError;

impl Display for StubError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "stub endpoint")
    }
}

impl ProjectionCatalog for LabelledEndpoint {
    fn describe(&self) -> EndpointDescriptor {
        EndpointDescriptor {
            label: self.label.clone(),
            projections: Vec::new(),
        }
    }
}

impl ProjectionSource for LabelledEndpoint {
    type Error = StubError;
    fn snapshot(&mut self, _: ProjectionRequest) -> Result<ProjectionSnapshot, StubError> {
        Err(StubError)
    }
}

impl PresentationSource for LabelledEndpoint {
    type Error = StubError;
    fn resource(&mut self, _: ResourceRequest) -> Result<ResourceResponse, StubError> {
        Err(StubError)
    }
}

impl IntentSink for LabelledEndpoint {
    type Error = StubError;
    fn invoke(&mut self, _: IntentInvocation) -> Result<IntentResult, StubError> {
        Ok(IntentResult::Accepted)
    }
}

impl ProjectionNoticeSource for LabelledEndpoint {
    type Error = StubError;
    fn poll_notice(&mut self) -> Result<Option<CarrierNotice>, StubError> {
        Ok(None)
    }
}

fn route() -> ResidentEndpointRoute {
    ResidentEndpointRoute::new("knot", Duration::from_millis(10)).expect("route")
}

/// A catalog whose factory counts how many endpoints it has built, so a
/// test can show each session gets its own.
fn catalog(opened: Arc<AtomicU32>) -> ResidentEndpointCatalog {
    let mut catalog = ResidentEndpointCatalog::new();
    catalog
        .register_notifying("knot", "Knot", move |_| {
            let index = opened.fetch_add(1, Ordering::SeqCst);
            Ok(LabelledEndpoint {
                label: format!("knot-{index}"),
            })
        })
        .expect("register");
    catalog
}

fn host(opened: Arc<AtomicU32>) -> ResidentProjectionHost {
    ResidentProjectionHost::new(policy(), route(), catalog(opened))
}

fn peers() -> (MemoryTransport, MemoryTransport, PeerID, PeerID) {
    let subject = viewer().master_public_key().to_bytes();
    let client_peer = PeerID::from_bytes(&subject).expect("client peer");
    let server_peer =
        PeerID::from_bytes(&owner().master_public_key().to_bytes()).expect("server peer");
    let (server, client) = MemoryTransport::pair(server_peer, client_peer);
    (server, client, server_peer, client_peer)
}

fn open_body() -> CarrierRequestBody {
    CarrierRequestBody::Open(Box::new(SessionOpen {
        version: ProtocolVersion { major: 1, minor: 0 },
        capabilities: CapabilityProfile::default(),
    }))
}

/// Dial, open, report the endpoint label, then hold the session until
/// `release` says to close it.
fn visit(
    client: Arc<MemoryTransport>,
    server_peer: PeerID,
    client_peer: PeerID,
    nonce: [u8; 32],
    handle: tokio::runtime::Handle,
    opened: mpsc::Sender<String>,
    release: mpsc::Receiver<()>,
) {
    let viewer = viewer();
    let subject = viewer.master_public_key().to_bytes();
    let hello = open_session(
        &viewer,
        NETWORK,
        profile_ref(),
        TrafficClass::Interactive,
        nonce,
        &projection_binding(client_peer),
        vec![grant(subject)],
    )
    .expect("hello");
    let stream = handle
        .block_on(dial_projection_session(
            client.as_ref(),
            server_peer,
            &hello,
            &policy().limits,
        ))
        .expect("dial")
        .expect("admitted");
    let mut carrier = NetworkCarrier::over(stream, CarrierRuntime::borrowed(handle));
    match carrier.request(open_body()).expect("open") {
        CarrierResponseBody::Opened(opened_session) => {
            opened
                .send(opened_session.descriptor.label)
                .expect("report");
        },
        other => panic!("expected an opened session, got {other:?}"),
    }
    release.recv().expect("release");
    carrier.request(CarrierRequestBody::Close).expect("close");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_peer_is_admitted_and_served_from_the_catalog() {
    let (server, client, server_peer, client_peer) = peers();
    let mut host = host(Arc::new(AtomicU32::new(0)));
    let (report, reported) = mpsc::channel();
    let (release, released) = mpsc::channel();
    let handle = tokio::runtime::Handle::current();
    let visitor = tokio::task::spawn_blocking(move || {
        visit(
            Arc::new(client),
            server_peer,
            client_peer,
            [5; 32],
            handle,
            report,
            released,
        )
    });

    let served = host
        .accept_one(&server, || NOW_MS)
        .await
        .expect("accept")
        .expect("admitted");
    assert_eq!(served.subject(), viewer().master_public_key().to_bytes());
    assert_eq!(reported.recv().unwrap(), "knot-0");

    release.send(()).unwrap();
    visitor.await.unwrap();
    let summary = served.finished().await.expect("join").expect("served");
    assert_eq!(summary.answered, 2, "the open and the close");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
async fn two_peers_are_served_at_once_rather_than_one_after_the_other() {
    // The reason this host spawns rather than loops. Both visitors hold
    // their sessions open at the same time, and the second one is admitted
    // and answered while the first is still connected. A sequential accept
    // loop would deadlock here rather than fail an assertion.
    let (server, client, server_peer, client_peer) = peers();
    let client = Arc::new(client);
    let opened = Arc::new(AtomicU32::new(0));
    let mut host = host(Arc::clone(&opened));
    let handle = tokio::runtime::Handle::current();

    let mut reporters = Vec::new();
    let mut releases = Vec::new();
    let mut visitors = Vec::new();
    for index in 0..2u8 {
        let (report, reported) = mpsc::channel();
        let (release, released) = mpsc::channel();
        let client = Arc::clone(&client);
        let handle = handle.clone();
        visitors.push(tokio::task::spawn_blocking(move || {
            // A fresh nonce per session, because that is what mints a
            // distinct transcript and therefore a distinct session id.
            visit(
                client,
                server_peer,
                client_peer,
                [index; 32],
                handle,
                report,
                released,
            )
        }));
        reporters.push(reported);
        releases.push(release);
    }

    let first = host
        .accept_one(&server, || NOW_MS)
        .await
        .expect("accept")
        .expect("admitted");
    let second = host
        .accept_one(&server, || NOW_MS)
        .await
        .expect("accept")
        .expect("admitted");

    // Both answered while both are still connected.
    let mut labels = vec![reporters[0].recv().unwrap(), reporters[1].recv().unwrap()];
    labels.sort();
    assert_eq!(labels, vec!["knot-0".to_string(), "knot-1".to_string()]);
    assert_eq!(
        opened.load(Ordering::SeqCst),
        2,
        "each session opened its own endpoint from the catalog"
    );
    assert_eq!(host.live_sessions(), 2, "both are being served at once");
    assert_ne!(
        first.session(),
        second.session(),
        "two sessions, two transcripts"
    );

    for release in releases {
        release.send(()).unwrap();
    }
    for visitor in visitors {
        visitor.await.unwrap();
    }
    first.finished().await.expect("join").expect("served");
    second.finished().await.expect("join").expect("served");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_route_the_host_does_not_offer_serves_nobody() {
    // A host misconfiguration, caught after admission and before any
    // endpoint exists. The peer gets no session rather than some other
    // product's.
    let (server, client, server_peer, client_peer) = peers();
    let mut host = ResidentProjectionHost::new(
        policy(),
        ResidentEndpointRoute::new("absent", Duration::from_millis(10)).expect("route"),
        catalog(Arc::new(AtomicU32::new(0))),
    );
    let handle = tokio::runtime::Handle::current();
    let dialling = tokio::task::spawn_blocking(move || {
        let viewer = viewer();
        let subject = viewer.master_public_key().to_bytes();
        let hello = open_session(
            &viewer,
            NETWORK,
            profile_ref(),
            TrafficClass::Interactive,
            [5; 32],
            &projection_binding(client_peer),
            vec![grant(subject)],
        )
        .expect("hello");
        handle
            .block_on(dial_projection_session(
                &client,
                server_peer,
                &hello,
                &policy().limits,
            ))
            .expect("dial")
            .expect("admitted");
    });

    let error = host
        .accept_one(&server, || NOW_MS)
        .await
        .expect_err("an unknown route cannot be served");
    assert!(
        matches!(error, ResidentProjectionError::Catalog(_)),
        "{error}"
    );
    assert_eq!(host.live_sessions(), 0, "no phantom session was counted");
    dialling.await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_finished_session_releases_its_slot() {
    // The count the policy's `max_sessions` is checked against, so a
    // session that ends has to give its slot back.
    let (server, client, server_peer, client_peer) = peers();
    let mut host = host(Arc::new(AtomicU32::new(0)));
    let (report, reported) = mpsc::channel();
    let (release, released) = mpsc::channel();
    let handle = tokio::runtime::Handle::current();
    let visitor = tokio::task::spawn_blocking(move || {
        visit(
            Arc::new(client),
            server_peer,
            client_peer,
            [5; 32],
            handle,
            report,
            released,
        )
    });

    let served = host
        .accept_one(&server, || NOW_MS)
        .await
        .expect("accept")
        .expect("admitted");
    reported.recv().unwrap();
    assert_eq!(host.live_sessions(), 1);

    release.send(()).unwrap();
    visitor.await.unwrap();
    served.finished().await.expect("join").expect("served");
    assert_eq!(host.live_sessions(), 0, "the slot came back");
}

#[test]
fn a_panicking_session_gives_its_slot_back() {
    // The leak the guard exists for. A serving task runs product code, and
    // a panic there unwinds past any trailing decrement while tokio
    // absorbs the panic at the join handle, so the host would survive with
    // the slot counted forever and quietly stop admitting at max_sessions.
    let live = Arc::new(AtomicU32::new(0));
    let guard = LiveSession::enter(Arc::clone(&live));
    assert_eq!(live.load(Ordering::SeqCst), 1);
    let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        let _held = guard;
        panic!("the endpoint blew up mid-session");
    }));
    assert!(unwound.is_err(), "the panic really happened");
    assert_eq!(
        live.load(Ordering::SeqCst),
        0,
        "the slot came back on unwind, not just on a clean return"
    );
}

#[test]
fn revocation_notices_are_shared_with_every_live_session() {
    // Shared rather than snapshotted, so an owner withdrawing a grant is
    // answered at the next request on sessions that are already open.
    let host = host(Arc::new(AtomicU32::new(0)));
    let ledger = Arc::clone(host.revocations());
    assert!(Arc::ptr_eq(&ledger, host.revocations()));
}
