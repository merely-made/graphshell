// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! An admitted native stream as wire items in and wire items out.
//!
//! The item-driven sessions in this crate and above it ([`crate::SessionDriver`],
//! `mere_overmap::route_session::MereRouteSession`) take the items that arrive
//! and give the items to send, and own no I/O. A browser feeds them from its
//! data channel. This is the native side of the same bargain (transport
//! agnosticism note, ruling T3): an admitted Tokio stream, read and written
//! as [`chirograph::wire`]'s CBOR Sequence, and a loop that runs an
//! [`ItemDriven`] session over it until the session settles.
//!
//! Behind the `pump` feature, so the default client keeps no runtime.

use crate::driver::ItemDriven;
use chirograph::wire::ItemAssembler;
use chirograph::{
    CapabilityProfile, CarrierOutput, CarrierRequest, CarrierRequestBody, CarrierResponseBody,
    ProtocolVersion, SessionOpen, SessionOpened,
};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadHalf, WriteHalf};

/// Why a pump stopped.
#[derive(Debug)]
pub enum PumpError {
    /// The stream failed.
    Io(std::io::Error),
    /// The stream ended while the session still waited for an item.
    Closed,
    /// The session, or the endpoint's answer to opening it, refused to go on.
    Session(String),
}

impl std::fmt::Display for PumpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "session stream failed: {error}"),
            Self::Closed => write!(f, "the session stream ended"),
            Self::Session(reason) => write!(f, "{reason}"),
        }
    }
}

impl std::error::Error for PumpError {}

impl From<std::io::Error> for PumpError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

/// The next item from an async stream; `Ok(None)` at a clean end. Cancel
/// safe, as `read` is: an abandoned read leaves what was buffered in place.
///
/// # Errors
///
/// A failed read, or `InvalidData` for a malformed head or a stream that
/// ends inside an item.
pub async fn read_item<R: AsyncRead + Unpin>(
    reader: &mut R,
    items: &mut ItemAssembler,
) -> std::io::Result<Option<Vec<u8>>> {
    let mut chunk = vec![0u8; 64 * 1024];
    loop {
        if let Some(item) = items.next_item().map_err(chirograph::wire::invalid)? {
            return Ok(Some(item));
        }
        let read = reader.read(&mut chunk).await?;
        if read == 0 {
            items.finish().map_err(chirograph::wire::invalid)?;
            return Ok(None);
        }
        items.push(&chunk[..read]);
    }
}

/// An admitted stream, split into an item reader and an item writer.
pub struct ItemPump<S> {
    reader: ReadHalf<S>,
    items: ItemAssembler,
    writer: WriteHalf<S>,
}

impl<S: AsyncRead + AsyncWrite> ItemPump<S> {
    /// Pump items over `stream`, which admission has already cleared.
    pub fn new(stream: S) -> Self {
        let (reader, writer) = tokio::io::split(stream);
        Self {
            reader,
            items: ItemAssembler::default(),
            writer,
        }
    }

    /// Send one item and flush it.
    pub async fn send(&mut self, item: &[u8]) -> Result<(), PumpError> {
        self.writer.write_all(item).await?;
        self.writer.flush().await?;
        Ok(())
    }

    /// The next item; `None` when the stream ends between items. Cancel safe:
    /// a read a timeout abandons loses nothing already buffered.
    pub async fn recv(&mut self) -> Result<Option<Vec<u8>>, PumpError> {
        Ok(read_item(&mut self.reader, &mut self.items).await?)
    }

    /// Open the session the stream carries and return what the endpoint
    /// described. The request id is 0, below any a [`crate::SessionDriver`]
    /// numbers its own requests from.
    pub async fn open(
        &mut self,
        capabilities: CapabilityProfile,
    ) -> Result<SessionOpened, PumpError> {
        let request = CarrierRequest {
            id: 0,
            body: CarrierRequestBody::Open(Box::new(SessionOpen {
                version: ProtocolVersion::V1,
                capabilities,
            })),
        };
        let item = chirograph::wire::encode(&request)
            .map_err(|error| PumpError::Session(format!("could not encode the open: {error}")))?;
        self.send(&item).await?;
        loop {
            let Some(item) = self.recv().await? else {
                return Err(PumpError::Closed);
            };
            let output = chirograph::wire::decode::<CarrierOutput>(&item).map_err(|error| {
                PumpError::Session(format!("the endpoint sent an unreadable item: {error}"))
            })?;
            match output {
                // A bell before the session is open has nothing to ring.
                CarrierOutput::Notice(_) => continue,
                CarrierOutput::Response(response) if response.id == 0 => {
                    return match response.body {
                        Ok(CarrierResponseBody::Opened(opened)) => Ok(*opened),
                        Ok(_) => Err(PumpError::Session(
                            "the endpoint answered the open with something else".into(),
                        )),
                        Err(failure) => Err(PumpError::Session(failure.message)),
                    };
                },
                CarrierOutput::Response(response) => {
                    return Err(PumpError::Session(format!(
                        "the endpoint answered request {} before the open",
                        response.id
                    )));
                },
            }
        }
    }

    /// Run `session` over the stream until `settled` says it is: send what it
    /// has, and while it is not settled, hand it the next item. Returns once
    /// it is settled with nothing left to send.
    pub async fn drive<L, F>(&mut self, session: &mut L, settled: F) -> Result<(), PumpError>
    where
        L: ItemDriven,
        F: Fn(&L) -> bool,
    {
        loop {
            for item in session.take_outgoing() {
                self.send(&item).await?;
            }
            if settled(session) {
                return Ok(());
            }
            let Some(item) = self.recv().await? else {
                return Err(PumpError::Closed);
            };
            session.on_item(&item).map_err(PumpError::Session)?;
        }
    }

    /// Close the writing half, so the endpoint sees the end of the stream.
    pub async fn shutdown(mut self) -> Result<(), PumpError> {
        self.writer.shutdown().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chirograph::{CarrierResponse, EndpointDescriptor, SessionStatus};

    /// A session that sends `count` requests one at a time and settles once
    /// each has an answer.
    struct Counting {
        sent: u64,
        answered: u64,
        count: u64,
        outbox: Vec<Vec<u8>>,
    }

    impl ItemDriven for Counting {
        fn take_outgoing(&mut self) -> Vec<Vec<u8>> {
            if self.outbox.is_empty() && self.sent == self.answered && self.sent < self.count {
                self.sent += 1;
                let request = CarrierRequest {
                    id: self.sent,
                    body: CarrierRequestBody::Close,
                };
                self.outbox
                    .push(chirograph::wire::encode(&request).unwrap());
            }
            std::mem::take(&mut self.outbox)
        }

        fn on_item(&mut self, item: &[u8]) -> Result<(), String> {
            match chirograph::wire::decode::<CarrierOutput>(item).map_err(|e| e.to_string())? {
                CarrierOutput::Response(response) if response.id == self.sent => {
                    self.answered += 1;
                    Ok(())
                },
                CarrierOutput::Notice(_) => Ok(()),
                other => Err(format!("unexpected {other:?}")),
            }
        }
    }

    async fn echo_endpoint(stream: tokio::io::DuplexStream) {
        let mut pump = ItemPump::new(stream);
        while let Some(item) = pump.recv().await.unwrap() {
            let request: CarrierRequest = chirograph::wire::decode(&item).unwrap();
            let body = match request.body {
                CarrierRequestBody::Open(open) => {
                    CarrierResponseBody::Opened(Box::new(SessionOpened {
                        version: open.version,
                        descriptor: EndpointDescriptor {
                            label: "echo".into(),
                            projections: Vec::new(),
                        },
                        status: SessionStatus::Live,
                        expires_at_ms: None,
                    }))
                },
                _ => CarrierResponseBody::Closed,
            };
            let output = chirograph::wire::encode(&CarrierOutput::Response(CarrierResponse {
                id: request.id,
                body: Ok(body),
            }))
            .unwrap();
            // Written in two pieces: the reader reassembles the item.
            let (head, tail) = output.split_at(output.len() / 2);
            pump.send(head).await.unwrap();
            pump.send(tail).await.unwrap();
        }
    }

    #[tokio::test]
    async fn a_session_opens_and_runs_until_settled_over_one_stream() {
        let (client, server) = tokio::io::duplex(4096);
        let endpoint = tokio::spawn(echo_endpoint(server));
        let mut pump = ItemPump::new(client);
        let opened = pump.open(CapabilityProfile::default()).await.unwrap();
        assert_eq!(opened.descriptor.label, "echo");

        let mut session = Counting {
            sent: 0,
            answered: 0,
            count: 3,
            outbox: Vec::new(),
        };
        pump.drive(&mut session, |session| session.answered == session.count)
            .await
            .unwrap();
        assert_eq!((session.sent, session.answered), (3, 3));
        pump.shutdown().await.unwrap();
        endpoint.await.unwrap();
    }

    #[tokio::test]
    async fn a_stream_that_ends_early_is_reported() {
        let (client, server) = tokio::io::duplex(4096);
        drop(server);
        let mut pump = ItemPump::new(client);
        assert!(matches!(
            pump.open(CapabilityProfile::default()).await,
            Err(PumpError::Io(_) | PumpError::Closed)
        ));
    }
}
