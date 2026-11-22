// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The WebRTC door's signaling listener: somewhere a browser can post an
//! offer. Lifted from Turnstone's C4 host (graphshell:mere-cli D5), and still
//! the dumbest thing that works (T8, "keep HTTP for now"): one POST carries
//! the offer, its response carries the answer, and there is no trickle.
//!
//! Nothing here authenticates. The door does, over the data channel this
//! answer opens ([`crate::webrtc_session::serve_webrtc_join`]); signaling is a
//! relay nobody trusts, which is what the door is built to be carried over.
//!
//! | Route | Answer |
//! |---|---|
//! | `GET /health` | `ok` |
//! | `GET /invite` | the invite fragment |
//! | `POST /offer` | the answer SDP; the opened channel goes to [`SignalingHooks::joined`] |
//! | `OPTIONS *` | `204`, for a browser's preflight |
//! | anything else | [`SignalingHooks::route`], else `404` |

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use webrtc_carrier::native::{Answerer, AnswererConfig, CarrierConfig};

/// The largest offer accepted. A browser's offer SDP is a few kilobytes.
pub const MAX_OFFER_BYTES: usize = 64 * 1024;
/// The most header bytes read before a request is refused.
pub const MAX_HEADER_BYTES: usize = 16 * 1024;

/// Where answers bind, and what `/invite` serves.
#[derive(Clone, Debug)]
pub struct SignalingConfig {
    /// The UDP address each answerer binds.
    pub bind: SocketAddr,
    /// The addresses an answer advertises. Declare exactly what the browser
    /// can reach: the carrier labels inbound datagrams with the first one.
    pub advertise: Vec<IpAddr>,
    /// The invite fragment `GET /invite` serves.
    pub fragment: String,
    pub carrier: CarrierConfig,
}

/// One reply a product route gives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reply {
    pub status: &'static str,
    pub body: String,
}

impl Reply {
    pub fn new(status: &'static str, body: impl Into<String>) -> Self {
        Self {
            status,
            body: body.into(),
        }
    }
}

type Joined = dyn Fn(u64, Answerer) + Send + Sync;
type Route = dyn Fn(&str, &str) -> Option<Reply> + Send + Sync;
type Report = dyn Fn(u64, &str) + Send + Sync;

/// What a product does with the listener's events.
#[derive(Clone)]
pub struct SignalingHooks {
    /// An offer was answered; `joined` owns the answerer from here (accept
    /// the channel, run the door). Called on the request's task, so spawn
    /// anything long.
    pub joined: Arc<Joined>,
    /// A route the product adds, by method and path.
    pub route: Option<Arc<Route>>,
    /// A request that failed, for the product's log.
    pub report: Option<Arc<Report>>,
}

impl SignalingHooks {
    pub fn joined(joined: impl Fn(u64, Answerer) + Send + Sync + 'static) -> Self {
        Self {
            joined: Arc::new(joined),
            route: None,
            report: None,
        }
    }

    pub fn with_route(
        mut self,
        route: impl Fn(&str, &str) -> Option<Reply> + Send + Sync + 'static,
    ) -> Self {
        self.route = Some(Arc::new(route));
        self
    }

    pub fn with_report(mut self, report: impl Fn(u64, &str) + Send + Sync + 'static) -> Self {
        self.report = Some(Arc::new(report));
        self
    }
}

/// Serve signaling on `listener` until it fails. Each request runs on its
/// own task, so one slow visitor never queues the next.
pub async fn serve_signaling(
    listener: TcpListener,
    config: SignalingConfig,
    hooks: SignalingHooks,
) -> std::io::Result<()> {
    let config = Arc::new(config);
    let requests = Arc::new(AtomicU64::new(0));
    loop {
        let (stream, _) = listener.accept().await?;
        let id = requests.fetch_add(1, Ordering::SeqCst) + 1;
        let config = Arc::clone(&config);
        let hooks = hooks.clone();
        tokio::spawn(async move {
            if let Err(error) = serve_request(stream, id, &config, &hooks).await
                && let Some(report) = &hooks.report
            {
                report(id, &error);
            }
        });
    }
}

async fn serve_request(
    mut stream: TcpStream,
    id: u64,
    config: &SignalingConfig,
    hooks: &SignalingHooks,
) -> Result<(), String> {
    let (read, mut write) = stream.split();
    let mut reader = BufReader::new(read).take(MAX_HEADER_BYTES as u64);
    let mut request_line = String::new();
    reader
        .read_line(&mut request_line)
        .await
        .map_err(|error| format!("request line: {error}"))?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let path = parts.next().unwrap_or_default().to_string();
    let mut content_length = 0usize;
    loop {
        let mut header = String::new();
        let read = reader
            .read_line(&mut header)
            .await
            .map_err(|error| format!("header: {error}"))?;
        if read == 0 {
            return respond(&mut write, &Reply::new("431 Request Header Fields Too Large", "")).await;
        }
        let header = header.trim_end();
        if header.is_empty() {
            break;
        }
        if let Some(value) = header
            .to_ascii_lowercase()
            .strip_prefix("content-length:")
            .map(str::trim)
            .and_then(|value| value.parse::<usize>().ok())
        {
            content_length = value;
        }
    }
    let reply = match (method.as_str(), path.as_str()) {
        ("OPTIONS", _) => Reply::new("204 No Content", ""),
        ("GET", "/health") => Reply::new("200 OK", "ok"),
        ("GET", "/invite") => Reply::new("200 OK", config.fragment.clone()),
        ("POST", "/offer") if content_length > MAX_OFFER_BYTES => {
            Reply::new("413 Content Too Large", format!("offers are at most {MAX_OFFER_BYTES} bytes"))
        },
        ("POST", "/offer") => {
            let mut body = vec![0u8; content_length];
            let mut reader = reader.into_inner();
            reader
                .read_exact(&mut body)
                .await
                .map_err(|error| format!("body: {error}"))?;
            let offer = String::from_utf8_lossy(&body).into_owned();
            if offer.trim().is_empty() {
                return respond(&mut write, &Reply::new("400 Bad Request", "empty offer")).await;
            }
            // A fresh answerer per offer: `accept` consumes it, and each
            // session gets its own socket and certificate.
            let mut answerer = Answerer::bind(AnswererConfig {
                bind: config.bind,
                advertise: config.advertise.clone(),
                carrier: config.carrier.clone(),
            })
            .await
            .map_err(|error| format!("bind: {error}"))?;
            match answerer.answer(&offer) {
                Ok(answer) => {
                    respond(&mut write, &Reply::new("200 OK", answer)).await?;
                    (hooks.joined)(id, answerer);
                    return Ok(());
                },
                Err(error) => Reply::new("400 Bad Request", error.to_string()),
            }
        },
        (method, path) => hooks
            .route
            .as_ref()
            .and_then(|route| route(method, path))
            .unwrap_or_else(|| Reply::new("404 Not Found", "try POST /offer, GET /invite, GET /health")),
    };
    respond(&mut write, &reply).await
}

async fn respond(write: &mut (impl AsyncWriteExt + Unpin), reply: &Reply) -> Result<(), String> {
    let response = format!(
        "HTTP/1.1 {}\r\n\
         Content-Type: text/plain; charset=utf-8\r\n\
         Content-Length: {}\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Access-Control-Allow-Headers: content-type\r\n\
         Connection: close\r\n\
         \r\n{}",
        reply.status,
        reply.body.len(),
        reply.body
    );
    write
        .write_all(response.as_bytes())
        .await
        .map_err(|error| format!("response: {error}"))?;
    write.flush().await.map_err(|error| format!("flush: {error}"))
}

#[cfg(test)]
#[path = "signaling_tests.rs"]
mod tests;
