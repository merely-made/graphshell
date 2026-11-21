use super::*;
use crate::invitation::{Contact, Invitation};
use crate::network_carrier::{CarrierRuntime, NetworkCarrier, dial_projection_session};
use chirograph::{
    CapabilityProfile, Carrier, CarrierRequestBody, CarrierResponseBody, ProjectionSession,
    ProtocolVersion, SessionOpen,
};
use personae::InMemoryProvider;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use transport::PeerID;
use transport::memory::MemoryTransport;
use uuid::Uuid;

const TERMS: InvitationTerms = InvitationTerms {
    domain: "mere.query",
    salt: b"test/mere-query/v1",
    network_label: b"test.mere-query/v1",
    path_root: "/services/projection/mere-query",
};
const SESSION: Uuid = Uuid::from_bytes([7; 16]);
const NOW: u64 = 1_000_000;
const EXPIRES: u64 = NOW + 60_000;
const DATA: &str = "<urn:book> <urn:title> \"Ada | Grace\\nVol. 1\" .\n\
                    <urn:book> <urn:by> <urn:ada> .\n<urn:paper> <urn:by> <urn:grace> .";
const ALLOWED: &str = "SELECT ?s ?o WHERE { ?s <urn:by> ?o } ORDER BY ?s";
const OTHER: &str = "SELECT * WHERE { ?s ?p ?o }";

fn owner_identity() -> InMemoryProvider {
    InMemoryProvider::from_seed([1; 32])
}

fn guest() -> InMemoryProvider {
    InMemoryProvider::from_seed([2; 32])
}

fn profile() -> ProfileRef {
    ProfileRef {
        id: "mere.base".into(),
        revision: 1,
    }
}

fn invitation() -> Invitation {
    let contact = Contact::create(&guest(), &TERMS).unwrap();
    let digest = ContentHash::of(ALLOWED.as_bytes());
    Invitation::issue_query(&owner_identity(), &TERMS, SESSION, contact, digest, NOW, EXPIRES).unwrap()
}

fn dataset() -> Arc<IndexedDataset> {
    let format = mere::formats::RdfFormat::from_extension("ttl").unwrap();
    Arc::new(
        mere::formats::parse::parse(format, DATA.as_bytes(), None, mere::ContextCache::new())
            .unwrap(),
    )
}

/// The owner's side: the host, its end of the transport and its clock.
struct Owner {
    host: ResidentProjectionHost,
    server: MemoryTransport,
    clock: Arc<AtomicU64>,
    issued: Invitation,
}

impl Owner {
    async fn accept(&mut self) -> Result<crate::projection_host::ServedProjection, String> {
        let clock = Arc::clone(&self.clock);
        self.host
            .accept_one(&self.server, move || clock.load(Ordering::SeqCst))
            .await
            .map_err(|error| error.to_string())?
            .map_err(|refusal| format!("{refusal:?}"))
    }
}

/// The guest's side: its end of the transport and its invitation.
#[derive(Clone)]
struct Dialer {
    client: Arc<MemoryTransport>,
    server_peer: PeerID,
    client_peer: PeerID,
    invitation: Invitation,
}

fn fixture() -> (Owner, Dialer) {
    let invitation = invitation();
    let root = invitation.owner.root();
    let clock = Arc::new(AtomicU64::new(NOW));
    let policy = query_policy(
        &TERMS,
        NetworkId(TERMS.network(root, SESSION)),
        root,
        vec![profile()],
        Some(4),
    );
    let issued = invitation.clone();
    let admit_clock = Arc::clone(&clock);
    let host = query_host(policy, TERMS, dataset(), move |principal, chain| {
        issued.admits(&TERMS, principal, chain, false, admit_clock.load(Ordering::SeqCst))
    })
    .unwrap();
    let server_peer = PeerID::from_bytes(&invitation.owner.peer()).unwrap();
    let client_peer = PeerID::from_bytes(&invitation.guest.peer()).unwrap();
    let (server, client) = MemoryTransport::pair(server_peer, client_peer);
    let owner = Owner {
        host,
        server,
        clock,
        issued: invitation.clone(),
    };
    let dialer = Dialer {
        client: Arc::new(client),
        server_peer,
        client_peer,
        invitation,
    };
    (owner, dialer)
}

type Guest = NetworkCarrier<<MemoryTransport as transport::Transport>::Stream>;

/// Dial with the invitation and open the session, or say why admission refused.
fn dial(fixture: &Dialer, handle: &tokio::runtime::Handle) -> Result<Guest, String> {
    let hello = fixture
        .invitation
        .hello(&TERMS, &guest(), profile(), fixture.client_peer)?;
    let stream = handle
        .block_on(dial_projection_session(
            fixture.client.as_ref(),
            fixture.server_peer,
            &hello,
            &Default::default(),
        ))
        .map_err(|error| error.to_string())?
        .map_err(|reason| format!("refused: {reason:?}"))?;
    let mut carrier = NetworkCarrier::over(stream, CarrierRuntime::borrowed(handle.clone()));
    let open = CarrierRequestBody::Open(Box::new(SessionOpen {
        version: ProtocolVersion { major: 1, minor: 0 },
        capabilities: CapabilityProfile::default(),
    }));
    match carrier.request(open).map_err(|error| error.to_string())? {
        CarrierResponseBody::Opened(_) => Ok(carrier),
        other => Err(format!("expected an opened session, got {other:?}")),
    }
}

/// Every row of `query`, a small page at a time.
fn ask(carrier: &mut Guest, query: &str) -> Result<Vec<QueryRow>, String> {
    let mut request = QueryRequest::first(ProjectionSession("guest".into()), query);
    request.max_bytes = 40;
    let mut rows = Vec::new();
    loop {
        let body = CarrierRequestBody::Query(request.clone());
        let page = match carrier.request(body).map_err(|error| error.to_string())? {
            CarrierResponseBody::Query(page) => page,
            other => return Err(format!("expected a query page, got {other:?}")),
        };
        rows.extend(page.rows.iter().cloned());
        match request.next(&page) {
            Some(next) => request = next,
            None => return Ok(rows),
        }
    }
}

fn expected() -> Vec<QueryRow> {
    vec![
        vec![Some("urn:book".into()), Some("urn:ada".into())],
        vec![Some("urn:paper".into()), Some("urn:grace".into())],
    ]
}

#[test]
fn a_query_grant_names_its_digest_and_round_trips() {
    let invitation = invitation();
    invitation.check(&TERMS).unwrap();
    let action = invitation.action(&TERMS);
    assert_eq!(action.action, QUERY_ACTION);
    assert_eq!(
        TERMS.granted_query(&action),
        Some((SESSION, ContentHash::of(ALLOWED.as_bytes())))
    );
    let saved: Invitation =
        serde_json::from_slice(&serde_json::to_vec(&invitation).unwrap()).unwrap();
    assert_eq!(saved, invitation);
    let mut widened = invitation.clone();
    widened.query = None;
    assert!(widened.check(&TERMS).is_err(), "a query grant is not a session grant");
    let mut connect = action.clone();
    connect.action = "connect".into();
    assert_eq!(TERMS.granted_query(&connect), None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_granted_query_answers_and_another_is_refused() {
    let (mut owner, dialer) = fixture();
    let handle = tokio::runtime::Handle::current();
    let visitor = tokio::task::spawn_blocking(move || {
        let mut carrier = dial(&dialer, &handle).unwrap();
        let granted = ask(&mut carrier, ALLOWED);
        let other = ask(&mut carrier, OTHER);
        let again = ask(&mut carrier, ALLOWED);
        let _ = carrier.request(CarrierRequestBody::Close);
        (granted, other, again)
    });
    let served = owner.accept().await.expect("the invited guest is admitted");
    let (granted, other, again) = visitor.await.unwrap();
    assert_eq!(granted.unwrap(), expected());
    assert!(other.unwrap_err().contains("not enabled"));
    assert_eq!(again.unwrap(), expected(), "a refusal does not end the session");
    served.finished().await.unwrap().unwrap();
}

/// Ask once, let the owner withdraw the grant (`withdraw`), then ask on the
/// open session and dial again: both must be refused.
async fn withdrawn(withdraw: impl FnOnce(&mut Owner)) {
    let (mut owner, dialer) = fixture();
    let handle = tokio::runtime::Handle::current();
    let (asked, first_asked) = mpsc::channel();
    let (withdrew, was_withdrawn) = mpsc::channel::<()>();
    let visitor = tokio::task::spawn_blocking(move || {
        let mut carrier = dial(&dialer, &handle).unwrap();
        asked.send(ask(&mut carrier, ALLOWED)).unwrap();
        was_withdrawn.recv().unwrap();
        let open = ask(&mut carrier, ALLOWED);
        let redial = dial(&dialer, &handle).map(|_| ());
        (open, redial)
    });
    let served = owner.accept().await.expect("admitted before withdrawal");
    let first = tokio::task::spawn_blocking(move || first_asked.recv().unwrap())
        .await
        .unwrap();
    assert_eq!(first.unwrap(), expected());
    withdraw(&mut owner);
    withdrew.send(()).unwrap();
    let readmitted = owner.accept().await;
    let (open, redial) = visitor.await.unwrap();
    assert!(open.is_err(), "the open session stops answering: {open:?}");
    assert!(redial.is_err() && readmitted.is_err(), "and a new dial is refused");
    let _ = served.finished().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_revoked_invitation_is_refused() {
    withdrawn(|owner| {
        let revocation = owner.issued.revoke(&owner_identity(), NOW).unwrap();
        owner
            .host
            .revocations()
            .write()
            .unwrap()
            .fold(revocation.check().unwrap());
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_expired_invitation_is_refused() {
    withdrawn(|owner| owner.clock.store(EXPIRES + 1, Ordering::SeqCst)).await;
}
