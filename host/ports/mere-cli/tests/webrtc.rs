// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! A browser-shaped reader, played by str0m: it fetches the invite and the
//! query from the CLI's signaling, posts its offer, joins through the door,
//! and pages the granted query's rows. Everything a browser does, minus the
//! browser (graphshell:mere-cli D11, D18).
#![cfg(all(feature = "webrtc", feature = "test-identity"))]

use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use chirograph::{
    CapabilityProfile, Carrier, CarrierRequestBody, CarrierResponseBody, ProjectionSession,
    ProtocolVersion, QueryRequest, SessionOpen,
};
use graphshell_host::network_carrier::{CarrierRuntime, NetworkCarrier};
use graphshell_host::webrtc_session::peer_join;
use personae::InMemoryProvider;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use webrtc_carrier::native::{CarrierConfig, LoopbackOfferer, serve, stream_over_frames};
use webrtc_carrier::{FingerprintRole, InviteV1};

const MERE: &str = env!("CARGO_BIN_EXE_mere");
const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

struct Server(Child);
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

async fn http(addr: &str, method: &str, path: &str, body: &str) -> (String, String) {
    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: {addr}\r\nContent-Length: {}\r\n\
         Connection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).await.unwrap();
    let (head, body) = response.split_once("\r\n\r\n").unwrap();
    (head.lines().next().unwrap().to_string(), body.to_string())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_browser_reader_joins_through_the_door_and_reads_the_granted_query() {
    let dir = tempfile::tempdir().unwrap();
    let file = |name| dir.path().join(name);
    std::fs::write(file("owner.seed"), "01".repeat(32)).unwrap();
    std::fs::copy(Path::new(FIXTURES).join("paper-authors.rq"), file("allowed.rq")).unwrap();
    let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let listen = format!("127.0.0.1:{port}");
    let mut server = Server(
        Command::new(MERE)
            .current_dir(dir.path())
            .args(["--identity-seed", "owner.seed", "serve", "--graph"])
            .arg(Path::new(FIXTURES).join("paper-authors.trig"))
            .args(["--via", "webrtc", "--allow-query", "allowed.rq", "--invite", "paper.invite"])
            .args(["--listen", &listen])
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(25);
    while !file("paper.invite").exists() {
        assert!(server.0.try_wait().unwrap().is_none(), "the owner exited early");
        assert!(Instant::now() < deadline, "the owner failed to advertise");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let written: serde_json::Value =
        serde_json::from_slice(&std::fs::read(file("paper.invite")).unwrap()).unwrap();
    assert_eq!(written["signal"], format!("http://{listen}"));

    let outcome = tokio::time::timeout(Duration::from_secs(60), async {
        let (_, fragment) = http(&listen, "GET", "/invite", "").await;
        assert_eq!(fragment, written["fragment"].as_str().unwrap());
        let (_, query) = http(&listen, "GET", "/query", "").await;
        assert_eq!(query, std::fs::read_to_string(file("allowed.rq")).unwrap().replace("\r\n", "\n"));
        let invite = InviteV1::parse_fragment(&fragment).unwrap();

        let config = CarrierConfig::default();
        let mut offerer = LoopbackOfferer::create(&config.channel_label).await;
        let (status, answer) = http(&listen, "POST", "/offer", &offerer.offer).await;
        assert!(status.contains("200"), "{status}: {answer}");
        offerer.accept_answer(&answer);
        let peer_config = CarrierConfig {
            local_dtls_role: FingerprintRole::Client,
            ..config
        };
        let mut carrier = serve(offerer.rtc, offerer.socket, peer_config).await.unwrap();
        let fingerprints = carrier.fingerprints().unwrap();
        let label = carrier.channel_label().to_string();
        peer_join(
            &mut carrier,
            &InMemoryProvider::from_seed([9; 32]),
            &invite,
            &label,
            *fingerprints.client(),
            *fingerprints.server(),
            &notochord::HandshakeLimits::default().clamped(),
        )
        .await
        .expect("the browser is admitted for the query its invite grants");
        let (reader, writer, control) = carrier.into_parts();
        let (stream, pump) = stream_over_frames(reader, writer);
        let handle = tokio::runtime::Handle::current();
        let rows = tokio::task::spawn_blocking(move || {
            let mut carrier = NetworkCarrier::over(stream, CarrierRuntime::borrowed(handle));
            let open = CarrierRequestBody::Open(Box::new(SessionOpen {
                version: ProtocolVersion { major: 1, minor: 0 },
                capabilities: CapabilityProfile::default(),
            }));
            assert!(matches!(carrier.request(open).unwrap(), CarrierResponseBody::Opened(_)));
            let other = QueryRequest::first(ProjectionSession("b".into()), "SELECT * { ?s ?p ?o }");
            let refused = carrier.request(CarrierRequestBody::Query(other));
            assert!(format!("{refused:?}").contains("not enabled"), "{refused:?}");
            let mut request = QueryRequest::first(ProjectionSession("b".into()), query);
            request.max_bytes = 30;
            let mut rows = Vec::new();
            loop {
                let CarrierResponseBody::Query(page) =
                    carrier.request(CarrierRequestBody::Query(request.clone())).unwrap()
                else {
                    panic!("expected a query page");
                };
                rows.extend(page.rows.iter().cloned());
                match request.next(&page) {
                    Some(next) => request = next,
                    None => break,
                }
            }
            let _ = carrier.request(CarrierRequestBody::Close);
            rows
        })
        .await
        .unwrap();
        let _ = pump.await;
        let _ = control.close().await;
        rows
    })
    .await
    .expect("the browser path completes inside its budget");
    let cell = |s: &str| Some(s.to_string());
    assert_eq!(
        outcome,
        vec![
            vec![cell("Ada"), cell("Library catalog")],
            vec![cell("Ada"), cell("Publisher")],
            vec![cell("Grace"), cell("Publisher")],
        ]
    );
}
