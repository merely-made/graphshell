// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The wire inside every session stream: an RFC 8742 CBOR Sequence (engine
//! split D210).
//!
//! Each [`crate::CarrierRequest`] one way and [`crate::CarrierOutput`] the
//! other is one canonical CBOR::Core item, encoded through cbor-core. A
//! keyframe or diff inside one is a sealed [`crate::envelope`]. This is the
//! codec every end shares, with no I/O beneath it: the endpoint's stdio and
//! network carriers, the client's session driver and pump, the browser's
//! WebRTC session, and the host's Tokio streams.
//!
//! A sequence has no delimiters, so [`ItemAssembler`] finds each item's end
//! from its heads; CBOR::Core has no indefinite lengths, so that is a count.

use std::fmt;

use cbor_core::{DecodeOptions, Value};
use serde::Serialize;
use serde::de::DeserializeOwned;

/// Why bytes did not become a value, or a value bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WireError {
    /// Not one well-formed CBOR item, or not canonical CBOR::Core.
    Malformed(String),
    /// A well-formed item that is not the type asked for, or a value that
    /// does not serialize.
    Value(String),
    /// The stream ended inside an item.
    Truncated(usize),
}

impl fmt::Display for WireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed(e) => write!(f, "malformed CBOR item: {e}"),
            Self::Value(e) => write!(f, "unexpected CBOR value: {e}"),
            Self::Truncated(n) => write!(f, "the stream ended {n} bytes into an item"),
        }
    }
}

impl std::error::Error for WireError {}

/// One value as one canonical item.
///
/// # Errors
///
/// [`WireError::Value`] when the value does not serialize, a keyframe or diff
/// inside it included.
pub fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, WireError> {
    Value::serialized(value)
        .map(|value| value.encode())
        .map_err(|error| WireError::Value(error.to_string()))
}

/// One item read back. Non-canonical bytes and trailing bytes are refused.
///
/// # Errors
///
/// [`WireError::Malformed`] for bytes that are not exactly one canonical
/// item, and [`WireError::Value`] for one that is not a `T`.
pub fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, WireError> {
    DecodeOptions::new()
        .decode(bytes)
        .map_err(|error| WireError::Malformed(error.to_string()))?
        .deserialized()
        .map_err(|error| WireError::Value(error.to_string()))
}

/// Bytes in, complete items out, with a partial item carried across pushes.
///
/// For a carrier that delivers bytes in pieces of its own size: a data
/// channel, a pipe, a socket. The scan resumes where it stopped, so a large
/// keyframe arriving in many pieces is walked once.
#[derive(Debug, Default)]
pub struct ItemAssembler {
    buffer: Vec<u8>,
    /// How far the current item's heads have been walked.
    scanned: usize,
    /// Data items still owed by the current item; 0 before its first head.
    owed: u64,
}

impl ItemAssembler {
    /// Append bytes as they arrived.
    pub fn push(&mut self, bytes: &[u8]) {
        self.buffer.extend_from_slice(bytes);
    }

    /// Whether nothing is buffered.
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// The next complete item's bytes, if one is buffered.
    ///
    /// # Errors
    ///
    /// [`WireError::Malformed`] for a head CBOR::Core does not allow: an
    /// indefinite length, a reserved argument, or a stray break.
    pub fn next_item(&mut self) -> Result<Option<Vec<u8>>, WireError> {
        if self.owed == 0 && self.scanned == 0 {
            if self.buffer.is_empty() {
                return Ok(None);
            }
            self.owed = 1;
        }
        while self.owed > 0 {
            let Some((head, children)) = head(&self.buffer[self.scanned..])? else {
                return Ok(None);
            };
            self.scanned += head;
            self.owed = self.owed - 1 + children;
        }
        let item = self.buffer.drain(..self.scanned).collect();
        self.scanned = 0;
        Ok(Some(item))
    }

    /// For end of stream: an error when part of an item is still buffered.
    ///
    /// # Errors
    ///
    /// [`WireError::Truncated`] naming how many bytes were left.
    pub fn finish(&self) -> Result<(), WireError> {
        match self.buffer.len() {
            0 => Ok(()),
            n => Err(WireError::Truncated(n)),
        }
    }
}

/// One head and any string body after it: its length in bytes and how many
/// data items it opens. `None` when the bytes stop short of it.
fn head(bytes: &[u8]) -> Result<Option<(usize, u64)>, WireError> {
    let Some(&first) = bytes.first() else {
        return Ok(None);
    };
    let (major, info) = (first >> 5, first & 0x1f);
    let width = match info {
        0..=23 => 0,
        24 => 1,
        25 => 2,
        26 => 4,
        27 => 8,
        _ => {
            return Err(WireError::Malformed(format!(
                "head 0x{first:02x} is indefinite or reserved"
            )));
        },
    };
    if bytes.len() < 1 + width {
        return Ok(None);
    }
    let argument = if width == 0 {
        u64::from(info)
    } else {
        bytes[1..=width]
            .iter()
            .fold(0u64, |value, byte| (value << 8) | u64::from(*byte))
    };
    let head = 1 + width;
    Ok(match major {
        0 | 1 | 7 => Some((head, 0)),
        2 | 3 => {
            let end = usize::try_from(argument)
                .ok()
                .and_then(|length| head.checked_add(length))
                .ok_or_else(|| WireError::Malformed("a string longer than memory".into()))?;
            (bytes.len() >= end).then_some((end, 0))
        },
        4 => Some((head, argument)),
        5 => Some((
            head,
            argument
                .checked_mul(2)
                .ok_or_else(|| WireError::Malformed("a map longer than memory".into()))?,
        )),
        _ => Some((head, 1)),
    })
}

/// A wire error as the I/O error a stream reader reports it as.
pub fn invalid(error: WireError) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, error)
}

/// Items from a blocking byte stream, a chunk at a time.
pub struct ItemReader<R> {
    reader: R,
    items: ItemAssembler,
}

impl<R: std::io::Read> ItemReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            items: ItemAssembler::default(),
        }
    }

    /// The next item's bytes; `Ok(None)` at a clean end of stream.
    ///
    /// # Errors
    ///
    /// A failed read, or [`std::io::ErrorKind::InvalidData`] carrying a
    /// [`WireError`] for a malformed head or a stream that ends inside an
    /// item.
    pub fn next_item(&mut self) -> std::io::Result<Option<Vec<u8>>> {
        let mut chunk = vec![0u8; 64 * 1024];
        loop {
            if let Some(item) = self.items.next_item().map_err(invalid)? {
                return Ok(Some(item));
            }
            let read = self.reader.read(&mut chunk)?;
            if read == 0 {
                self.items.finish().map_err(invalid)?;
                return Ok(None);
            }
            self.items.push(&chunk[..read]);
        }
    }

    /// The next item, decoded. A well-formed item that is not a `T` is
    /// `Ok(Some(Err(_)))`, so the caller decides whether it ends the stream.
    ///
    /// # Errors
    ///
    /// As [`Self::next_item`].
    pub fn next<T: DeserializeOwned>(&mut self) -> std::io::Result<Option<Result<T, WireError>>> {
        Ok(self.next_item()?.map(|item| decode(&item)))
    }
}

#[cfg(test)]
#[path = "wire_tests.rs"]
mod tests;
