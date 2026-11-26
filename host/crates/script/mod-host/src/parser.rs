// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The `parser-guest` host (mod projects D36, D40): the fetched bytes and their media type
//! in, an [`EngineDocument`] and diagnostics out.
//!
//! The guest returns the document as a version byte then CBOR. The host caps it, decodes it
//! strictly, and overwrites what the guest must not decide: the address, the provenance and
//! the trust state.

use document_host::mod_loader::InstallRecord;
use inker::{DocumentProvenance, DocumentTrustState, EngineDocument};
use wasmtime::component::{Component, HasSelf};
use wasmtime::Engine;

use crate::{GuestState, Limits, MAX_DIAGNOSTICS, MAX_DOCUMENT_BYTES};

mod world {
    wasmtime::component::bindgen!({
        path: "../wit",
        world: "parser-guest",
        exports: { default: async },
        imports: { default: async },
    });
}
use world::exports::mere::script::parser as wit;
use world::ParserGuest;

impl world::mere::script::log::Host for GuestState {
    async fn log(&mut self, message: String) {
        GuestState::log(self, message);
    }
}

impl world::mere::script::caps::Host for GuestState {
    async fn granted(&mut self) -> Vec<String> {
        GuestState::granted(self)
    }
}

/// The version byte of a parser's document payload.
pub const DOCUMENT_VERSION: u8 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

/// One diagnostic from a parse: how bad, which bytes of the input, and what.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    /// A byte range of the input.
    pub span: Option<(u32, u32)>,
    pub message: String,
}

impl Diagnostic {
    /// `warning 4..9: message`, or without the range.
    pub fn words(&self) -> String {
        match self.span {
            Some((a, b)) => format!("{} {a}..{b}: {}", self.severity.as_str(), self.message),
            None => format!("{}: {}", self.severity.as_str(), self.message),
        }
    }
}

/// What the host fixes on the document, not the guest.
#[derive(Clone, Copy, Debug)]
pub struct ParseContext<'a> {
    /// The page's address.
    pub address: &'a str,
    /// The engine id recorded as the document's source (`mod:<id>`).
    pub engine_id: &'a str,
    pub trust: DocumentTrustState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedDocument {
    pub document: EngineDocument,
    pub diagnostics: Vec<Diagnostic>,
}

/// Why a parse produced no document. After `Trapped` the instance is spent: drop it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseFailure {
    /// The guest declined, with its own words.
    Refused(String),
    /// The guest trapped or ran out of its deadline or memory.
    Trapped(String),
    /// The input or the returned document is over its cap.
    TooLarge(String),
    /// The payload's version byte is not [`DOCUMENT_VERSION`].
    BadVersion(u8),
    /// The payload is not a strictly encoded `EngineDocument`.
    Malformed(String),
}

impl ParseFailure {
    /// Whether the instance must be dropped (D20).
    pub fn drops_instance(&self) -> bool {
        matches!(self, Self::Trapped(_))
    }
}

impl std::fmt::Display for ParseFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Refused(why) => write!(f, "refused: {why}"),
            Self::Trapped(why) => write!(f, "trapped: {why}"),
            Self::TooLarge(why) => write!(f, "too large: {why}"),
            Self::BadVersion(v) => write!(f, "document version {v} is not {DOCUMENT_VERSION}"),
            Self::Malformed(why) => write!(f, "malformed document: {why}"),
        }
    }
}

impl std::error::Error for ParseFailure {}

/// One live `parser-guest` instance; every call arms a fresh deadline.
pub struct ParserHost {
    store: wasmtime::Store<GuestState>,
    bindings: ParserGuest,
    limits: Limits,
}

impl ParserHost {
    /// Instantiate from the exact bytes supplied. With a `record`, the bytes must match the
    /// component it pinned (D4). `granted` is what `caps.granted()` reports.
    ///
    /// # Errors
    ///
    /// The component is over the cap, differs from the record, or does not instantiate.
    pub fn attach_bytes(
        engine: &Engine,
        bytes: &[u8],
        record: Option<&InstallRecord>,
        granted: Vec<String>,
        limits: Limits,
    ) -> wasmtime::Result<Self> {
        crate::admit_bytes(bytes, record)?;
        let component = Component::from_binary(engine, bytes)?;
        let mut linker = crate::base_linker(engine)?;
        world::mere::script::log::add_to_linker::<_, HasSelf<GuestState>>(&mut linker, |s| s)?;
        world::mere::script::caps::add_to_linker::<_, HasSelf<GuestState>>(&mut linker, |s| s)?;
        let mut store = crate::new_store(engine, granted, limits);
        let bindings =
            pollster::block_on(ParserGuest::instantiate_async(&mut store, &component, &linker))?;
        Ok(Self {
            store,
            bindings,
            limits,
        })
    }

    fn arm(&mut self) {
        self.store.set_epoch_deadline(self.limits.epoch_deadline_ticks);
    }

    /// The guest's captured `log` output, oldest first.
    pub fn logs(&self) -> &[String] {
        &self.store.data().logs
    }

    /// The media types the guest says it parses, normalized.
    ///
    /// # Errors
    ///
    /// The guest trapped.
    pub fn describe(&mut self) -> wasmtime::Result<Vec<String>> {
        self.arm();
        let claim =
            pollster::block_on(self.bindings.mere_script_parser().call_describe(&mut self.store))?;
        Ok(claim
            .media_types
            .iter()
            .filter_map(|t| crate::normalize_media_type(t))
            .collect())
    }

    /// Parse `bytes` of `media_type`. The document comes back with the host's address,
    /// provenance and trust.
    ///
    /// # Errors
    ///
    /// See [`ParseFailure`].
    pub fn parse(
        &mut self,
        bytes: &[u8],
        media_type: &str,
        ctx: &ParseContext<'_>,
    ) -> Result<ParsedDocument, ParseFailure> {
        if bytes.len() > MAX_DOCUMENT_BYTES {
            return Err(ParseFailure::TooLarge(format!(
                "input is {} bytes; the cap is {MAX_DOCUMENT_BYTES}",
                bytes.len()
            )));
        }
        self.arm();
        let media_type = crate::normalize_media_type(media_type).unwrap_or_default();
        let result = pollster::block_on(self.bindings.mere_script_parser().call_parse(
            &mut self.store,
            bytes,
            &media_type,
        ))
        .map_err(|trap| ParseFailure::Trapped(crate::first_line(&trap)))?;
        let parsed = result.map_err(ParseFailure::Refused)?;
        let document = decode_document(&parsed.document, ctx)?;
        Ok(ParsedDocument {
            document,
            diagnostics: diagnostics_of(parsed.diagnostics),
        })
    }
}

/// Decode a parser's payload: cap, version byte, strict CBOR, then the host's own fields.
///
/// # Errors
///
/// The payload is over the cap, carries another version, or is not an `EngineDocument`.
pub fn decode_document(
    payload: &[u8],
    ctx: &ParseContext<'_>,
) -> Result<EngineDocument, ParseFailure> {
    if payload.len() > MAX_DOCUMENT_BYTES {
        return Err(ParseFailure::TooLarge(format!(
            "the document is {} bytes; the cap is {MAX_DOCUMENT_BYTES}",
            payload.len()
        )));
    }
    let (version, body) = payload
        .split_first()
        .ok_or_else(|| ParseFailure::Malformed("empty payload".into()))?;
    if *version != DOCUMENT_VERSION {
        return Err(ParseFailure::BadVersion(*version));
    }
    let mut document: EngineDocument = cbor_core::DecodeOptions::new()
        .decode(body)
        .map_err(|e| ParseFailure::Malformed(e.to_string()))?
        .deserialized()
        .map_err(|e| ParseFailure::Malformed(e.to_string()))?;
    document.address = ctx.address.to_string();
    document.provenance = DocumentProvenance::for_engine(ctx.engine_id, ctx.address);
    document.trust = ctx.trust;
    Ok(document)
}

fn diagnostics_of(list: Vec<wit::Diagnostic>) -> Vec<Diagnostic> {
    let total = list.len();
    let mut out: Vec<Diagnostic> = list
        .into_iter()
        .take(MAX_DIAGNOSTICS)
        .map(|d| Diagnostic {
            severity: match d.severity {
                wit::Severity::Info => Severity::Info,
                wit::Severity::Warning => Severity::Warning,
                wit::Severity::Error => Severity::Error,
            },
            span: d.span.map(|s| (s.start, s.end)),
            message: d.message,
        })
        .collect();
    if total > MAX_DIAGNOSTICS {
        out.push(Diagnostic {
            severity: Severity::Warning,
            span: None,
            message: format!(
                "the mod emitted {total} diagnostics; the first {MAX_DIAGNOSTICS} are kept"
            ),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> ParseContext<'static> {
        ParseContext { address: "a:1", engine_id: "mod:t", trust: DocumentTrustState::Unknown }
    }

    #[test]
    fn a_payload_is_checked_for_version_shape_and_trailing_bytes() {
        assert!(matches!(decode_document(&[], &ctx()), Err(ParseFailure::Malformed(_))));
        assert_eq!(decode_document(&[2, 0xa0], &ctx()), Err(ParseFailure::BadVersion(2)));
        // An empty CBOR map is not a document: its required fields are missing.
        assert!(matches!(decode_document(&[1, 0xa0], &ctx()), Err(ParseFailure::Malformed(_))));
        let doc = EngineDocument {
            address: "x".into(),
            title: None,
            content_type: "t/u".into(),
            lang: None,
            provenance: DocumentProvenance::default(),
            trust: DocumentTrustState::Trusted,
            diagnostics: vec![],
            navigation: inker::DocumentNavigation::default(),
            blocks: vec![],
        };
        let mut payload = vec![DOCUMENT_VERSION];
        payload.extend(cbor_core::Value::serialized(&doc).unwrap().encode());
        let ok = decode_document(&payload, &ctx()).unwrap();
        assert_eq!((ok.address.as_str(), ok.trust), ("a:1", DocumentTrustState::Unknown));
        payload.push(0);
        assert!(
            matches!(decode_document(&payload, &ctx()), Err(ParseFailure::Malformed(_))),
            "trailing bytes are refused"
        );
    }
}
