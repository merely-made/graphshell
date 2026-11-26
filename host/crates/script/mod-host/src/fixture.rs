// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Fixtures for a parser and a viewer mod (mod projects MP7, D35): a TOML file holding an
//! input and what the run must show. A run passes or shows the diff. The document world's
//! fixtures are `document_host::Fixture`.
//!
//! A parser fixture holds bytes and a media type, and expects the document's outline, its
//! title and its diagnostics, or a failure:
//!
//! ```toml
//! name = "two lines"
//!
//! [input]
//! media_type = "application/x-notes"
//! text = "# Title\nbody\n"        # or hex = "ff00"
//!
//! [expect]
//! title = "Title"
//! outline = """
//! heading 1: Title
//! paragraph: body
//! """
//! diagnostics = ["warning 8..12: odd line"]
//! # fails = "refused: no tabs"    # instead of all of the above
//! ```
//!
//! A viewer fixture holds a graph and expects the scene's structure, not its bytes or
//! pixels: the named regions in order, and a size range.
//!
//! ```toml
//! name = "two nodes"
//!
//! [graph]
//! json = '{"snapshot":{"nodes":[{"label":"A"},{"label":"B"}]},"facets":{}}'
//!
//! [ask]
//! width = 320.0
//!
//! [expect]
//! regions = ["Node A", "Node B"]
//! width = [100.0, 400.0]
//! height = [20.0, 200.0]
//! ```
//!
//! `fails = "text"` expects the run to fail with a message containing the text; a trap with
//! an install record is kept as its `last_trap`.

use std::path::Path;

use document_host::mod_loader::InstallRecord;
use inker::{Block, DocumentTrustState, EngineDocument, InlineSpan};
use serde::Deserialize;

use crate::parser::{ParseContext, ParseFailure, ParsedDocument, ParserHost};
use crate::viewer::{ViewFailure, ViewedScene, ViewerHost};
use crate::{Limits, guarded_engine, watchdog};

/// The result of a fixture run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FixtureOutcome {
    Pass { found: Vec<String> },
    /// The run differed: `-` lines were expected, `+` found.
    Mismatch { found: Vec<String>, diff: String },
}

impl FixtureOutcome {
    pub fn passed(&self) -> bool {
        matches!(self, Self::Pass { .. })
    }

    /// What the run showed, or `pass`.
    pub fn words(&self) -> String {
        match self {
            Self::Pass { .. } => "pass".into(),
            Self::Mismatch { diff, .. } => format!("run differs:\n{diff}"),
        }
    }
}

fn compare(expected: &[String], found: Vec<String>) -> FixtureOutcome {
    match document_host::diff_lines(expected, &found) {
        None => FixtureOutcome::Pass { found },
        Some(diff) => FixtureOutcome::Mismatch { found, diff },
    }
}

fn lines(text: &str) -> Vec<String> {
    text.lines().map(str::to_string).collect()
}

fn read_component(component: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(component).map_err(|e| format!("{}: {e}", component.display()))
}

// ---- parser ----

#[derive(Clone, Debug, Deserialize)]
pub struct ParserInput {
    pub media_type: String,
    /// The input as text; or `hex`, for bytes that are not UTF-8.
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub hex: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct ParserExpect {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub outline: String,
    #[serde(default)]
    pub diagnostics: Vec<String>,
    /// The run must fail with a message containing this text.
    #[serde(default)]
    pub fails: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ParserFixture {
    #[serde(default)]
    pub name: String,
    pub input: ParserInput,
    #[serde(default)]
    pub expect: ParserExpect,
}

impl ParserFixture {
    /// # Errors
    ///
    /// The parser's message, a media type that is not `type/subtype`, or an input with both
    /// or neither of `text` and `hex`.
    pub fn from_toml(text: &str) -> Result<Self, String> {
        let fixture: Self = toml::from_str(text).map_err(|e| e.to_string())?;
        fixture.input_bytes()?;
        Ok(fixture)
    }

    /// # Errors
    ///
    /// The file could not be read, or as for [`from_toml`](Self::from_toml).
    pub fn from_file(path: &Path) -> Result<Self, String> {
        let text =
            std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Self::from_toml(&text).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// The input's bytes.
    ///
    /// # Errors
    ///
    /// Both or neither of `text` and `hex`, bad hex, or a media type that is not one.
    pub fn input_bytes(&self) -> Result<Vec<u8>, String> {
        crate::normalize_media_type(&self.input.media_type)
            .ok_or_else(|| format!("'{}' is not a media type", self.input.media_type))?;
        match (&self.input.text, &self.input.hex) {
            (Some(t), None) => Ok(t.as_bytes().to_vec()),
            (None, Some(h)) => unhex(h),
            _ => Err("an input holds either text or hex, not both and not neither".into()),
        }
    }
}

fn unhex(h: &str) -> Result<Vec<u8>, String> {
    let digits: Vec<u8> = h.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if digits.len() % 2 != 0 {
        return Err("hex input has an odd number of digits".into());
    }
    digits
        .chunks(2)
        .map(|pair| {
            std::str::from_utf8(pair)
                .ok()
                .and_then(|p| u8::from_str_radix(p, 16).ok())
                .ok_or_else(|| "hex input has a non-hex digit".to_string())
        })
        .collect()
}

/// A document as an outline, one line per block, two spaces per level of nesting. A block
/// kind the outline does not spell out shows as its `kind_name`.
pub fn outline(document: &EngineDocument) -> Vec<String> {
    let mut out = Vec::new();
    for block in &document.blocks {
        outline_block(block, 0, &mut out);
    }
    out
}

fn outline_block(block: &Block, depth: usize, out: &mut Vec<String>) {
    let pad = "  ".repeat(depth);
    match block {
        Block::Presented { block, .. } => outline_block(block, depth, out),
        Block::Heading { level, spans } => out.push(format!("{pad}heading {level}: {}", plain(spans))),
        Block::Paragraph { spans } => out.push(format!("{pad}paragraph: {}", plain(spans))),
        Block::CodeBlock { text, .. } => out.push(format!("{pad}code: {}", text.trim_end())),
        Block::Quote { blocks } => {
            out.push(format!("{pad}quote"));
            for b in blocks {
                outline_block(b, depth + 1, out);
            }
        },
        Block::List { ordered, items } => {
            out.push(format!("{pad}list {}", if *ordered { "ordered" } else { "bulleted" }));
            for item in items {
                out.push(format!("{pad}  item"));
                for b in item {
                    outline_block(b, depth + 2, out);
                }
            }
        },
        Block::Rule => out.push(format!("{pad}rule")),
        other => out.push(format!("{pad}{}", other.kind_name())),
    }
}

fn plain(spans: &[InlineSpan]) -> String {
    spans
        .iter()
        .map(|span| match span {
            InlineSpan::Text(t) | InlineSpan::Code(t) => t.clone(),
            InlineSpan::Emphasis(s) | InlineSpan::Strong(s) => plain(s),
            InlineSpan::Link { spans, .. } => plain(spans),
            _ => String::new(),
        })
        .collect()
}

fn parser_found(parsed: &ParsedDocument, want_title: bool) -> Vec<String> {
    let mut found = Vec::new();
    if let (true, Some(title)) = (want_title, &parsed.document.title) {
        found.push(format!("title: {title}"));
    }
    found.extend(outline(&parsed.document));
    found.extend(parsed.diagnostics.iter().map(|d| format!("diagnostic {}", d.words())));
    found
}

/// Run `fixture` against the parser component at `component`: attach it (pinned to `record`
/// when one is given), hand it the input, and compare what it returns with the fixture. A
/// trap ends the run and, with a record, is kept as its `last_trap`.
///
/// # Errors
///
/// The component could not be read or would not attach, or the fixture's input is bad.
pub fn run_parser_fixture(
    fixture: &ParserFixture,
    component: &Path,
    limits: Limits,
    mut record: Option<&mut InstallRecord>,
) -> Result<FixtureOutcome, String> {
    let input = fixture.input_bytes()?;
    let bytes = read_component(component)?;
    let engine = guarded_engine().map_err(|e| e.to_string())?;
    let _dog = watchdog(&engine);
    let mut host = ParserHost::attach_bytes(
        &engine,
        &bytes,
        record.as_deref(),
        vec!["mere:script/log".into()],
        limits,
    )
    .map_err(|e| {
        let why = format!("attach: {e:#}");
        if let Some(r) = record.as_deref_mut() {
            r.note_trap(why.clone());
        }
        why
    })?;
    let ctx = ParseContext {
        address: "fixture://input",
        engine_id: "mod:fixture",
        trust: DocumentTrustState::Unknown,
    };
    let run = host.parse(&input, &fixture.input.media_type, &ctx);
    let expect = &fixture.expect;
    Ok(match (run, &expect.fails) {
        (Ok(parsed), None) => {
            let mut want = Vec::new();
            if let Some(title) = &expect.title {
                want.push(format!("title: {title}"));
            }
            want.extend(lines(&expect.outline));
            want.extend(expect.diagnostics.iter().map(|d| format!("diagnostic {d}")));
            compare(&want, parser_found(&parsed, expect.title.is_some()))
        },
        (Ok(parsed), Some(fails)) => {
            let found = parser_found(&parsed, true);
            let want = vec![format!("failure containing: {fails}")];
            FixtureOutcome::Mismatch {
                diff: document_host::diff_lines(&want, &found).unwrap_or_default(),
                found,
            }
        },
        (Err(failure), fails) => failure_outcome(&failure, fails.as_deref(), record, |r, why| {
            if matches!(failure, ParseFailure::Trapped(_)) {
                r.note_trap(why);
            }
        }),
    })
}

fn failure_outcome<F: std::fmt::Display>(
    failure: &F,
    fails: Option<&str>,
    record: Option<&mut InstallRecord>,
    note: impl FnOnce(&mut InstallRecord, String),
) -> FixtureOutcome {
    let words = failure.to_string();
    if let Some(r) = record {
        note(r, words.clone());
    }
    let found = vec![format!("failed: {words}")];
    match fails {
        Some(text) if words.contains(text) => FixtureOutcome::Pass { found },
        Some(text) => FixtureOutcome::Mismatch {
            diff: document_host::diff_lines(&[format!("failure containing: {text}")], &found)
                .unwrap_or_default(),
            found,
        },
        None => FixtureOutcome::Mismatch {
            diff: format!("+ failed: {words}\n"),
            found,
        },
    }
}

// ---- viewer ----

#[derive(Clone, Debug, Deserialize)]
pub struct ViewerGraph {
    /// The graph as `mere.graph-snapshot-and-facets/json`.
    pub json: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ViewerAsk {
    #[serde(default = "default_width")]
    pub width: f32,
}

fn default_width() -> f32 {
    320.0
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct ViewerExpect {
    /// The scene's region labels, in order; the count is their number.
    #[serde(default)]
    pub regions: Vec<String>,
    /// `[min, max]` of the result's width, when it matters.
    #[serde(default)]
    pub width: Option<[f32; 2]>,
    /// `[min, max]` of the result's height, when it matters.
    #[serde(default)]
    pub height: Option<[f32; 2]>,
    /// The run must fail with a message containing this text.
    #[serde(default)]
    pub fails: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ViewerFixture {
    #[serde(default)]
    pub name: String,
    pub graph: ViewerGraph,
    #[serde(default = "default_ask")]
    pub ask: ViewerAsk,
    #[serde(default)]
    pub expect: ViewerExpect,
}

fn default_ask() -> ViewerAsk {
    ViewerAsk {
        width: default_width(),
    }
}

impl ViewerFixture {
    /// # Errors
    ///
    /// The parser's message, or a graph that is not JSON.
    pub fn from_toml(text: &str) -> Result<Self, String> {
        let fixture: Self = toml::from_str(text).map_err(|e| e.to_string())?;
        // The graph is a JSON object; the host reads no more of it than that.
        let graph = fixture.graph.json.trim_start();
        if !graph.starts_with('{') {
            return Err("a viewer fixture's graph is a JSON object".into());
        }
        Ok(fixture)
    }

    /// # Errors
    ///
    /// The file could not be read, or as for [`from_toml`](Self::from_toml).
    pub fn from_file(path: &Path) -> Result<Self, String> {
        let text =
            std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Self::from_toml(&text).map_err(|e| format!("{}: {e}", path.display()))
    }
}

fn viewer_found(view: &ViewedScene, expect: &ViewerExpect) -> (Vec<String>, Vec<String>) {
    let labels = view.region_labels();
    let mut found = vec![format!("regions: {}", labels.len())];
    found.extend(labels.iter().map(|l| format!("region: {l}")));
    let mut want = vec![format!("regions: {}", expect.regions.len())];
    want.extend(expect.regions.iter().map(|l| format!("region: {l}")));
    for (name, range, value) in [
        ("width", expect.width, view.width),
        ("height", expect.height, view.height),
    ] {
        if let Some([min, max]) = range {
            let inside = value >= min && value <= max;
            found.push(format!("{name}: {}", if inside { "in range".into() } else { format!("{value} outside {min}..{max}") }));
            want.push(format!("{name}: in range"));
        }
    }
    (want, found)
}

/// Run `fixture` against the viewer component at `component`, as [`run_parser_fixture`].
///
/// # Errors
///
/// The component could not be read or would not attach.
pub fn run_viewer_fixture(
    fixture: &ViewerFixture,
    component: &Path,
    limits: Limits,
    mut record: Option<&mut InstallRecord>,
) -> Result<FixtureOutcome, String> {
    let bytes = read_component(component)?;
    let engine = guarded_engine().map_err(|e| e.to_string())?;
    let _dog = watchdog(&engine);
    let mut host = ViewerHost::attach_bytes(
        &engine,
        &bytes,
        record.as_deref(),
        vec!["mere:script/log".into()],
        limits,
    )
    .map_err(|e| {
        let why = format!("attach: {e:#}");
        if let Some(r) = record.as_deref_mut() {
            r.note_trap(why.clone());
        }
        why
    })?;
    let run = host.view(fixture.graph.json.as_bytes(), fixture.ask.width);
    let expect = &fixture.expect;
    Ok(match (run, &expect.fails) {
        (Ok(view), None) => {
            let (want, found) = viewer_found(&view, expect);
            compare(&want, found)
        },
        (Ok(view), Some(fails)) => {
            let (_, found) = viewer_found(&view, expect);
            FixtureOutcome::Mismatch {
                diff: document_host::diff_lines(&[format!("failure containing: {fails}")], &found)
                    .unwrap_or_default(),
                found,
            }
        },
        (Err(failure), fails) => failure_outcome(&failure, fails.as_deref(), record, |r, why| {
            if matches!(failure, ViewFailure::Trapped(_)) {
                r.note_trap(why);
            }
        }),
    })
}
