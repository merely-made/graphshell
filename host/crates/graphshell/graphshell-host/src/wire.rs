// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The CBOR Sequence wire over an admitted Tokio stream.
//!
//! Inside every admitted stream the wire is one canonical CBOR item per
//! message, keyframes and diffs sealed in their envelopes (engine split
//! D210). The codec itself is [`chirograph::wire`], which every end shares;
//! this module is that codec over an async reader and writer, for both ends of
//! a native stream: the session loop reads requests and writes responses and
//! notices, and an in-process client (the local doors' duplex) writes requests
//! and reads outputs through the same two functions.
//!
//! [`ItemReader::next`] is cancel safe: a read that a timeout abandons keeps
//! the partial item buffered and the next call resumes it. The session loop's
//! notice arm depends on that.

use chirograph::wire::{ItemAssembler, WireError};
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};

/// Reads one item at a time from an async stream.
pub struct ItemReader<R> {
    reader: R,
    items: ItemAssembler,
}

impl<R: AsyncRead + Unpin> ItemReader<R> {
    /// Read items from `reader`.
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            items: ItemAssembler::default(),
        }
    }

    /// The next item's bytes; `Ok(None)` at the end of the stream.
    ///
    /// # Errors
    ///
    /// A failed read, a malformed head, or a stream that ends inside an item.
    pub async fn next_item(&mut self) -> std::io::Result<Option<Vec<u8>>> {
        graphshell_client::pump::read_item(&mut self.reader, &mut self.items).await
    }

    /// The next item, decoded.
    ///
    /// `Ok(None)` is the end of the stream. A well-formed item that is not a
    /// `T` is `Ok(Some(Err(_)))`, so the caller decides whether it ends the
    /// session (a client) or is answered and skipped (the session loop). A
    /// failed read, a malformed head or a stream that ends inside an item is
    /// an `Err`: a sequence cannot find its place again.
    pub async fn next<T: DeserializeOwned>(
        &mut self,
    ) -> std::io::Result<Option<Result<T, WireError>>> {
        let item = graphshell_client::pump::read_item(&mut self.reader, &mut self.items).await?;
        Ok(item.map(|item| chirograph::wire::decode(&item)))
    }
}

/// Write `value` as one item and flush it.
///
/// Flushed per item on purpose: a response or a notice the peer is waiting on
/// must not sit in a buffer until the next one pushes it out.
pub async fn write<W, T>(writer: &mut W, value: &T) -> std::io::Result<()>
where
    W: AsyncWrite + Unpin,
    T: Serialize,
{
    let bytes = chirograph::wire::encode(value).map_err(std::io::Error::other)?;
    writer.write_all(&bytes).await?;
    writer.flush().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use chirograph::{CarrierOutput, CarrierRequest, CarrierRequestBody, CarrierResponse};

    #[tokio::test]
    async fn requests_and_outputs_cross_one_duplex_in_both_directions() {
        let (client, server) = tokio::io::duplex(1024);
        let (client_read, mut client_write) = tokio::io::split(client);
        let (server_read, mut server_write) = tokio::io::split(server);
        let mut client_items = ItemReader::new(client_read);
        let mut server_items = ItemReader::new(server_read);

        write(
            &mut client_write,
            &CarrierRequest {
                id: 7,
                body: CarrierRequestBody::Close,
            },
        )
        .await
        .unwrap();
        // A well-formed item that is not a request is reported, not swallowed.
        write(&mut client_write, &"not a request").await.unwrap();

        let request: CarrierRequest = server_items.next().await.unwrap().unwrap().unwrap();
        assert_eq!(request.id, 7);
        assert!(
            server_items
                .next::<CarrierRequest>()
                .await
                .unwrap()
                .unwrap()
                .is_err(),
            "a bad item is reported, not swallowed"
        );

        write(
            &mut server_write,
            &CarrierOutput::Response(CarrierResponse {
                id: 7,
                body: Ok(chirograph::CarrierResponseBody::Closed),
            }),
        )
        .await
        .unwrap();
        drop(server_write);
        drop(server_items);
        let output: CarrierOutput = client_items.next().await.unwrap().unwrap().unwrap();
        assert!(matches!(output, CarrierOutput::Response(response) if response.id == 7));
        assert!(
            client_items
                .next::<CarrierOutput>()
                .await
                .unwrap()
                .is_none(),
            "the end of the stream is None"
        );
    }

    fn descriptor_item(label: &str) -> Vec<u8> {
        chirograph::wire::encode(&CarrierOutput::Response(CarrierResponse {
            id: 1,
            body: Ok(chirograph::CarrierResponseBody::Descriptor(
                chirograph::EndpointDescriptor {
                    label: label.into(),
                    projections: Vec::new(),
                },
            )),
        }))
        .unwrap()
    }

    #[tokio::test]
    async fn invalid_utf8_inside_an_item_is_refused() {
        let (mut writer, reader) = tokio::io::duplex(128);
        let mut items = ItemReader::new(reader);
        let mut item = descriptor_item("A€");
        let at = item.windows(3).position(|w| w == "€".as_bytes()).unwrap();
        item[at] = 0xff;
        writer.write_all(&item).await.unwrap();
        assert!(
            items
                .next::<CarrierOutput>()
                .await
                .unwrap()
                .unwrap()
                .is_err()
        );
    }

    #[tokio::test]
    async fn a_stream_ending_inside_an_item_is_an_error() {
        let (mut writer, reader) = tokio::io::duplex(128);
        let mut items = ItemReader::new(reader);
        let item = descriptor_item("A");
        writer.write_all(&item[..item.len() - 1]).await.unwrap();
        drop(writer);
        let error = items.next::<CarrierOutput>().await.unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    }

    #[tokio::test]
    async fn cancelled_read_keeps_a_partial_item_and_a_split_codepoint() {
        use std::future::{Future, poll_fn};
        use std::task::Poll;

        let (mut writer, reader) = tokio::io::duplex(128);
        let mut items = ItemReader::new(reader);
        let item = descriptor_item("€");
        let at = item.windows(3).position(|w| w == "€".as_bytes()).unwrap();
        writer.write_all(&item[..at + 2]).await.unwrap();
        let mut pending = Box::pin(items.next::<CarrierOutput>());
        poll_fn(|cx| {
            assert!(pending.as_mut().poll(cx).is_pending());
            Poll::Ready(())
        })
        .await;
        drop(pending);
        writer.write_all(&item[at + 2..]).await.unwrap();
        let output = items
            .next::<CarrierOutput>()
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(matches!(output, CarrierOutput::Response(response)
            if matches!(&response.body, Ok(chirograph::CarrierResponseBody::Descriptor(descriptor))
                if descriptor.label == "€")));
    }
}
