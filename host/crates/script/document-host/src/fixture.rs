// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Fixtures for a Wasm mod (mod projects MP3, D6): a TOML file holding a starting
//! document, a list of `(event kind, payload)` turns, and the DOM and turn log
//! expected afterwards. A run passes or shows the diff.
//!
//! ```toml
//! name = "set the first paragraph"
//!
//! [document]
//! tree = """
//! body
//!   p
//!     #text "Hello"
//! """
//!
//! [[turn]]
//! kind = "set"
//! payload = "Changed"
//!
//! [expect]
//! dom = """
//! body
//!   p
//!     #text "Changed"
//! """
//! log = ["1 set: applied, rev 1"]
//! ```
//!
//! A tree is an outline: one node per line, two spaces per level, an element by its
//! tag and a text node as `#text` followed by a JSON string. The turn log has one
//! line per turn, `<n> <kind>: <outcome>`; a trapped turn ends the run.

use std::path::Path;
use std::sync::Arc;

use genet_scripted_dom::{NodeId, ScriptedDom};
use layout_dom_api::{LayoutDom, LayoutDomMut};
use serde::Deserialize;

use crate::mod_loader::InstallRecord;
use crate::{DocumentScript, Grant, NetFetcher, Quota, TurnOutcome, dom_view, qual};

/// The starting document.
#[derive(Clone, Debug, Deserialize)]
pub struct FixtureDocument {
    pub tree: String,
}

/// One event delivered to the script.
#[derive(Clone, Debug, Deserialize)]
pub struct FixtureTurn {
    pub kind: String,
    #[serde(default)]
    pub payload: String,
}

/// What the run must leave behind.
#[derive(Clone, Debug, Deserialize)]
pub struct FixtureExpect {
    pub dom: String,
    #[serde(default)]
    pub log: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Fixture {
    #[serde(default)]
    pub name: String,
    pub document: FixtureDocument,
    #[serde(default)]
    pub turn: Vec<FixtureTurn>,
    pub expect: FixtureExpect,
}

impl Fixture {
    /// # Errors
    ///
    /// The parser's message, or the tree's first malformed line.
    pub fn from_toml(text: &str) -> Result<Self, String> {
        let fixture: Self = toml::from_str(text).map_err(|e| e.to_string())?;
        parse_tree(&fixture.document.tree)?;
        parse_tree(&fixture.expect.dom)?;
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

/// What a run left behind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FixtureRun {
    /// The final document as an outline.
    pub dom: String,
    /// One line per turn run.
    pub log: Vec<String>,
    /// What the guest logged through `log`.
    pub guest_logs: Vec<String>,
}

/// The result of a fixture run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FixtureOutcome {
    Pass(FixtureRun),
    /// The run differed from the expectation: `-` lines were expected, `+` found.
    Mismatch {
        run: FixtureRun,
        dom_diff: Option<String>,
        log_diff: Option<String>,
    },
}

impl FixtureOutcome {
    pub fn passed(&self) -> bool {
        matches!(self, Self::Pass(_))
    }

    /// The diffs, or `pass`.
    pub fn words(&self) -> String {
        match self {
            Self::Pass(_) => "pass".into(),
            Self::Mismatch {
                dom_diff, log_diff, ..
            } => {
                let mut out = String::new();
                if let Some(d) = dom_diff {
                    out.push_str("dom differs:\n");
                    out.push_str(d);
                }
                if let Some(d) = log_diff {
                    out.push_str("turn log differs:\n");
                    out.push_str(d);
                }
                out
            },
        }
    }
}

/// Parse an outline into a document.
fn parse_tree(tree: &str) -> Result<ScriptedDom, String> {
    let mut dom = ScriptedDom::new();
    // The open ancestors, shallowest first; depth 0 hangs from the document.
    let mut stack: Vec<NodeId> = vec![dom.document()];
    for (n, line) in tree.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        let depth = indent / 2;
        if indent % 2 != 0 || depth >= stack.len() {
            return Err(format!("tree line {}: bad indentation", n + 1));
        }
        stack.truncate(depth + 1);
        let body = line.trim();
        let node = if let Some(rest) = body.strip_prefix("#text") {
            let text: String = serde_json::from_str(rest.trim())
                .map_err(|e| format!("tree line {}: #text wants a JSON string ({e})", n + 1))?;
            dom.create_text(&text)
        } else if body.starts_with('#') || body.contains(char::is_whitespace) {
            return Err(format!("tree line {}: unknown node '{body}'", n + 1));
        } else {
            dom.create_element(qual(body))
        };
        let parent = stack[depth];
        dom.append_child(parent, node);
        stack.push(node);
    }
    Ok(dom)
}

/// A document as an outline (the form [`parse_tree`] reads).
fn outline(dom: &ScriptedDom) -> String {
    let view = dom_view::snapshot(dom, 0);
    let mut depth: std::collections::HashMap<u64, usize> = std::collections::HashMap::new();
    let mut out = String::new();
    for node in view.nodes.iter().skip(1) {
        let d = node.parent.and_then(|p| depth.get(&p).copied()).map_or(0, |p| p + 1);
        depth.insert(node.id, d);
        out.push_str(&"  ".repeat(d));
        if node.kind == "#text" {
            out.push_str("#text ");
            out.push_str(&serde_json::to_string(&node.text).unwrap_or_default());
        } else {
            out.push_str(&node.kind);
        }
        out.push('\n');
    }
    out
}

/// A line diff of `expected` against `found`: `-` expected only, `+` found only.
/// `None` when they match.
fn diff(expected: &[String], found: &[String]) -> Option<String> {
    if expected == found {
        return None;
    }
    let (n, m) = (expected.len(), found.len());
    // Longest common subsequence table, suffix-first.
    let mut lcs = vec![vec![0usize; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            lcs[i][j] = if expected[i] == found[j] {
                lcs[i + 1][j + 1] + 1
            } else {
                lcs[i + 1][j].max(lcs[i][j + 1])
            };
        }
    }
    let (mut i, mut j) = (0, 0);
    let mut out = String::new();
    while i < n || j < m {
        if i < n && j < m && expected[i] == found[j] {
            out.push_str(&format!("  {}\n", expected[i]));
            i += 1;
            j += 1;
        } else if i < n && (j == m || lcs[i + 1][j] >= lcs[i][j + 1]) {
            out.push_str(&format!("- {}\n", expected[i]));
            i += 1;
        } else {
            out.push_str(&format!("+ {}\n", found[j]));
            j += 1;
        }
    }
    Some(out)
}

fn lines(text: &str) -> Vec<String> {
    text.lines().map(str::to_string).collect()
}

/// Run `fixture` against the component at `component`: attach it to the starting
/// document, deliver each turn, and compare the DOM and turn log with what the
/// fixture expects. A turn that traps ends the run; with a `record` the trap is
/// kept as its `last_trap`, naming the turn (an attach that fails, naming the
/// import, likewise).
///
/// # Errors
///
/// The fixture's document is malformed, or the component would not attach.
pub fn run_fixture(
    fixture: &Fixture,
    component: &Path,
    grant: &Grant,
    quota: Quota,
    fetcher: Option<Arc<dyn NetFetcher>>,
    net_origins: Vec<String>,
    mut record: Option<&mut InstallRecord>,
) -> Result<FixtureOutcome, String> {
    let dom = parse_tree(&fixture.document.tree)?;
    let attached = match record.as_deref() {
        Some(r) => DocumentScript::attach_pinned(component, r, dom, grant, quota, fetcher, net_origins),
        None => DocumentScript::attach(component, dom, grant, quota, fetcher, net_origins),
    };
    let mut script = attached.map_err(|e| {
        let why = format!("attach: {e:#}");
        if let Some(r) = record.as_deref_mut() {
            r.note_trap(why.clone());
        }
        why
    })?;
    let mut log = Vec::new();
    for (i, turn) in fixture.turn.iter().enumerate() {
        let n = i + 1;
        let kind = &turn.kind;
        match script.deliver_event(kind, &turn.payload) {
            Ok(outcome) => log.push(format!("{n} {kind}: {}", outcome_words(&outcome))),
            Err(e) => {
                log.push(format!("{n} {kind}: trapped"));
                if let Some(r) = record.as_deref_mut() {
                    r.note_trap(format!("turn {n} '{kind}': {e:#}"));
                }
                break;
            },
        }
    }
    let run = FixtureRun {
        dom: outline(script.dom()),
        log,
        guest_logs: script.guest_logs().to_vec(),
    };
    let dom_diff = diff(&lines(&fixture.expect.dom), &lines(&run.dom));
    let log_diff = diff(&fixture.expect.log, &run.log);
    Ok(if dom_diff.is_none() && log_diff.is_none() {
        FixtureOutcome::Pass(run)
    } else {
        FixtureOutcome::Mismatch {
            run,
            dom_diff,
            log_diff,
        }
    })
}

fn outcome_words(outcome: &TurnOutcome) -> String {
    match outcome {
        TurnOutcome::Applied(rev) => format!("applied, rev {rev}"),
        TurnOutcome::Conflict(rev) => format!("conflict, current rev {rev}"),
        TurnOutcome::UnknownNode(id) => format!("unknown node {id}"),
        TurnOutcome::Refused(why) => format!("refused ({why})"),
    }
}
