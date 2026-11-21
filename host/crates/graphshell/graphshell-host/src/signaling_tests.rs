use super::*;
use crate::carrier::projection_policy;
use crate::webrtc_door::{InviteTerms, issue_invite};
use crate::webrtc_session::{HostedInvite, peer_join, serve_webrtc_join};
use notochord::{NetworkId, ProfileRef, RevocationLedger, TrustedRoot};
use personae::{IdentityProvider, InMemoryProvider};
use std::time::Duration;
use tokio::sync::mpsc;
use webrtc_carrier::native::{LoopbackOfferer, serve};
use webrtc_carrier::{FingerprintRole, InviteV1, ReleaseRefV1};

const NETWORK: NetworkId = NetworkId([3; 32]);
const ROOT_AUTHORITY: [u8; 32] = [7; 32];
const NOW_MS: u64 = 50;

fn profile() -> ProfileRef {
    ProfileRef {
        id: "mere.base".into(),
        revision: 1,
    }
}

/// One HTTP request over a bare socket: the status line and the body.
async fn http(addr: SocketAddr, method: &str, path: &str, body: &str) -> (String, String) {
    declared(addr, method, path, body.len(), body).await
}

/// A request whose Content-Length is `length`, whatever the body is.
async fn declared(
    addr: SocketAddr,
    method: &str,
    path: &str,
    length: usize,
    body: &str,
) -> (String, String) {
    let mut stream = TcpStream::connect(addr).await.unwrap();
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: {addr}\r\nContent-Length: {length}\r\n\
         Connection: close\r\n\r\n{body}"
    );
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).await.unwrap();
    let (head, body) = response.split_once("\r\n\r\n").unwrap();
    (head.lines().next().unwrap().to_string(), body.to_string())
}

async fn listen(
    fragment: &str,
) -> (SocketAddr, mpsc::UnboundedReceiver<Answerer>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (joined, answerers) = mpsc::unbounded_channel();
    let hooks = SignalingHooks::joined(move |_, answerer| {
        let _ = joined.send(answerer);
    })
    .with_route(|method, path| {
        (method == "POST" && path == "/nudge").then(|| Reply::new("200 OK", "2"))
    });
    let config = SignalingConfig {
        bind: "127.0.0.1:0".parse().unwrap(),
        advertise: vec!["127.0.0.1".parse().unwrap()],
        fragment: fragment.into(),
        carrier: CarrierConfig::default(),
    };
    tokio::spawn(serve_signaling(listener, config, hooks));
    (addr, answerers)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_routes_answer_and_an_oversized_offer_is_refused() {
    let (addr, _) = listen("mwi1.fragment").await;
    assert_eq!(http(addr, "GET", "/health", "").await.1, "ok");
    assert_eq!(http(addr, "GET", "/invite", "").await.1, "mwi1.fragment");
    assert_eq!(http(addr, "POST", "/nudge", "").await.1, "2", "a product route");
    assert!(http(addr, "GET", "/elsewhere", "").await.0.contains("404"));
    assert!(http(addr, "OPTIONS", "/offer", "").await.0.contains("204"));
    // Refused on the declared length, before any of the body is read.
    let refused = declared(addr, "POST", "/offer", MAX_OFFER_BYTES + 1, "").await;
    assert!(refused.0.contains("413"), "{refused:?}");
    assert!(http(addr, "POST", "/offer", "").await.0.contains("400"));
}

/// A browser-shaped peer posts its offer through the module and joins
/// through the door over the channel the answer opened.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_peer_signals_through_the_module_and_joins_through_the_door() {
    let outcome = tokio::time::timeout(Duration::from_secs(30), async {
        let host_provider = InMemoryProvider::from_seed([1; 32]);
        let issue = issue_invite(
            &host_provider,
            &InviteTerms::projection(
                NETWORK,
                profile(),
                20_000,
                1,
                ReleaseRefV1 {
                    manifest_blake3: [0x5a; 32],
                    publisher_key_id: [0x6b; 32],
                },
            ),
        )
        .unwrap();
        let fragment = issue.descriptor.invite.to_fragment();
        let mut hosted = HostedInvite {
            invite: issue.descriptor.invite,
            redemption: issue.redemption,
        };
        let policy = projection_policy(
            NETWORK,
            vec![TrustedRoot {
                authority: ROOT_AUTHORITY,
                issuer: host_provider.master_public_key().to_bytes(),
            }],
            vec![profile()],
            None,
        );
        let limits = policy.limits.clamped();
        let (addr, mut answerers) = listen(&fragment).await;

        let served_fragment = http(addr, "GET", "/invite", "").await.1;
        let invite = InviteV1::parse_fragment(&served_fragment).unwrap();
        let config = CarrierConfig::default();
        let mut offerer = LoopbackOfferer::create(&config.channel_label).await;
        let (status, answer) = http(addr, "POST", "/offer", &offerer.offer).await;
        assert!(status.contains("200"), "{status}: {answer}");
        offerer.accept_answer(&answer);
        let answerer = answerers.recv().await.expect("the module hands over the answerer");

        let host = async move {
            let carrier = answerer.accept().await.expect("the host's channel opens");
            serve_webrtc_join(
                carrier,
                &host_provider,
                &mut hosted,
                &policy,
                &RevocationLedger::new(),
                ROOT_AUTHORITY,
                10_000,
                NOW_MS,
                0,
            )
            .await
            .map(|_| ())
            .map_err(|error| error.to_string())
        };
        let peer = async {
            let peer_config = CarrierConfig {
                local_dtls_role: FingerprintRole::Client,
                ..config
            };
            let mut carrier = serve(offerer.rtc, offerer.socket, peer_config)
                .await
                .expect("the peer's channel opens");
            let fingerprints = carrier.fingerprints().expect("the handshake was observed");
            let label = carrier.channel_label().to_string();
            peer_join(
                &mut carrier,
                &InMemoryProvider::from_seed([9; 32]),
                &invite,
                &label,
                *fingerprints.client(),
                *fingerprints.server(),
                &limits,
            )
            .await
            .map(|_| ())
            .map_err(|error| error.to_string())
        };
        let (host, peer) = tokio::join!(host, peer);
        host.expect("the door admits the peer");
        peer.expect("the peer is admitted");
    })
    .await;
    outcome.expect("the join completes inside its budget");
}
