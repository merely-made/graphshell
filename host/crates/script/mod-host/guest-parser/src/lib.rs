// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The `parser-guest` test fixture. The media type picks the behaviour:
//!
//! - `application/x-fixture-notes`: a `# ` line is the title and a heading, every other
//!   non-blank line a paragraph; a line that starts with a tab draws a warning over its bytes.
//! - `-refuse` declines, `-trap` traps, `-spin` never returns.
//! - `-huge` returns a document over the host's cap, `-badversion` a payload of version 9.
//! - `-liar` returns a document that names another address, a source and full trust, which
//!   the host must overwrite.
//! - `-chatty` returns 300 diagnostics.
//!
//! The document is built from a local mirror of inker's `EngineDocument` (no inker in a
//! guest): the host decodes it strictly into the real type, so a drift fails its tests.

wit_bindgen::generate!({
    path: "../../wit",
    world: "parser-guest",
});

use exports::mere::script::parser::{Claim, Diagnostic, Guest, Parsed, Severity, Span};
use serde::Serialize;

struct Component;

#[derive(Serialize)]
enum Inline {
    Text(String),
}

#[derive(Serialize)]
enum Block {
    Heading { level: u8, spans: Vec<Inline> },
    Paragraph { spans: Vec<Inline> },
}

#[derive(Serialize, Default)]
struct Provenance {
    source_kind: Option<String>,
    canonical_uri: Option<String>,
    fetched_at: Option<String>,
    source_label: Option<String>,
}

#[derive(Serialize)]
struct Document {
    address: String,
    title: Option<String>,
    content_type: String,
    provenance: Provenance,
    trust: &'static str,
    blocks: Vec<Block>,
}

const TYPES: [&str; 8] = ["notes", "refuse", "trap", "spin", "huge", "badversion", "liar", "chatty"];

fn encode(document: &Document, version: u8) -> Vec<u8> {
    let body = cbor_core::Value::serialized(document).expect("a serializable document");
    let mut out = vec![version];
    out.extend(body.encode());
    out
}

fn warn(start: usize, end: usize, message: &str) -> Diagnostic {
    Diagnostic {
        severity: Severity::Warning,
        span: Some(Span { start: start as u32, end: end as u32 }),
        message: message.into(),
    }
}

fn notes(text: &str, media_type: &str) -> (Document, Vec<Diagnostic>) {
    let mut title = None;
    let mut blocks = Vec::new();
    let mut diagnostics = Vec::new();
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let end = offset + line.len();
        let trimmed = line.trim_end_matches(['\n', '\r']);
        if let Some(head) = trimmed.strip_prefix("# ") {
            title.get_or_insert_with(|| head.to_string());
            blocks.push(Block::Heading { level: 1, spans: vec![Inline::Text(head.into())] });
        } else if !trimmed.trim().is_empty() {
            if trimmed.starts_with('\t') {
                diagnostics.push(warn(offset, end, "a tab starts this line"));
            }
            blocks.push(Block::Paragraph { spans: vec![Inline::Text(trimmed.trim().into())] });
        }
        offset = end;
    }
    let document = Document {
        address: "guest://chosen".into(),
        title,
        content_type: media_type.into(),
        provenance: Provenance::default(),
        trust: "Unknown",
        blocks,
    };
    (document, diagnostics)
}

impl Guest for Component {
    fn describe() -> Claim {
        Claim {
            media_types: TYPES.iter().map(|t| format!("application/x-fixture-{t}")).collect(),
        }
    }

    fn parse(bytes: Vec<u8>, media_type: String) -> Result<Parsed, String> {
        let kind = media_type.strip_prefix("application/x-fixture-").unwrap_or("");
        let text = String::from_utf8_lossy(&bytes).into_owned();
        match kind {
            "notes" => {
                let (document, diagnostics) = notes(&text, &media_type);
                Ok(Parsed { document: encode(&document, 1), diagnostics })
            },
            "refuse" => Err("fixture refuses this input".into()),
            "trap" => panic!("fixture traps"),
            "spin" => loop {
                std::hint::black_box(0);
            },
            "huge" => Ok(Parsed { document: vec![1; 17 * 1024 * 1024], diagnostics: vec![] }),
            "badversion" => {
                let (document, _) = notes(&text, &media_type);
                Ok(Parsed { document: encode(&document, 9), diagnostics: vec![] })
            },
            "liar" => {
                let (mut document, _) = notes(&text, &media_type);
                document.address = "https://evil.test/".into();
                document.trust = "Trusted";
                document.provenance = Provenance {
                    source_kind: Some("nematic.html".into()),
                    canonical_uri: Some("https://evil.test/".into()),
                    fetched_at: None,
                    source_label: Some("the bank".into()),
                };
                Ok(Parsed { document: encode(&document, 1), diagnostics: vec![] })
            },
            "chatty" => {
                let (document, _) = notes(&text, &media_type);
                let diagnostics = (0..300).map(|i| warn(0, 1, &format!("note {i}"))).collect();
                Ok(Parsed { document: encode(&document, 1), diagnostics })
            },
            other => Err(format!("fixture does not parse '{other}'")),
        }
    }
}

export!(Component);
