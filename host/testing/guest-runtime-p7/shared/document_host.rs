// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The document guest's host (guest runtime plan, P7): the `document-core`
//! bindings, a fixture document, the fetch rules of `document-host` (its
//! `net.rs` compiled from the same file) and the three native guards: a fuel
//! limit, an epoch deadline and a `StoreLimits` memory cap. One call to
//! `run_turn` is what `document-host`'s `run_guarded` is: a fresh store, the
//! guest activated, one event delivered.
//!
//! The document is a fixture, not Genet's DOM (P1 Findings 13): the native
//! test page `<body><p>Hello</p></body>` as a plain tree. A guard run needs
//! only `inspect` and the batch's outcome, so the fixture applies the four
//! mutations by the rules of `dom_view::apply` and nothing more.

use wasmtime::component::{Component, HasSelf, Linker};
use wasmtime::{Engine, ResourceLimiter, Store, StoreLimits, StoreLimitsBuilder};

wasmtime::component::bindgen!({
    path: "../../../crates/script/wit",
    world: "document-core",
});

use mere::script::document::{DocumentQuery, DocumentView, Mutation, ViewNode};

// The native fetch rules, from the same file `document-host` uses.
#[path = "../../../crates/script/document-host/src/net.rs"]
#[allow(dead_code)]
mod net;
use net::host_in_origins;

/// The change-detection token of `content_hash` in `document-host`'s
/// `src/lib.rs`: FNV-1a over kind, 0, text.
fn content_hash(kind: &str, text: &str) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0100_0000_01b3;
    let mut hash = OFFSET;
    for byte in kind.bytes().chain([0]).chain(text.bytes()) {
        hash = (hash ^ u64::from(byte)).wrapping_mul(PRIME);
    }
    hash
}

struct Node {
    id: u64,
    parent: Option<u64>,
    kind: String,
    text: String,
}

/// The fixture document: nodes in document order, with a revision.
pub struct Document {
    nodes: Vec<Node>,
    next_id: u64,
    pub revision: u64,
}

impl Document {
    /// `<body><p>Hello</p></body>`, as the native tests' `page()`.
    pub fn page() -> Self {
        let mut doc = Document {
            nodes: Vec::new(),
            next_id: 1,
            revision: 0,
        };
        let root = doc.create(None, "#document", "");
        let body = doc.create(Some(root), "body", "");
        let p = doc.create(Some(body), "p", "");
        doc.create(Some(p), "#text", "Hello");
        doc
    }

    fn create(&mut self, parent: Option<u64>, kind: &str, text: &str) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.nodes.push(Node {
            id,
            parent,
            kind: kind.into(),
            text: text.into(),
        });
        id
    }

    /// The text of every text node, joined by commas.
    pub fn texts(&self) -> String {
        let texts: Vec<&str> = self
            .nodes
            .iter()
            .filter(|n| n.kind == "#text")
            .map(|n| n.text.as_str())
            .collect();
        texts.join(",")
    }

    fn view(&self, query: &DocumentQuery) -> DocumentView {
        let in_scope = |n: &Node| match query {
            DocumentQuery::WholeDocument => true,
            DocumentQuery::Subtree(root) => self.is_within(n.id, *root),
        };
        let nodes = self
            .nodes
            .iter()
            .filter(|n| in_scope(n))
            .map(|n| ViewNode {
                id: n.id,
                parent: n.parent,
                kind: n.kind.clone(),
                text: n.text.clone(),
                content_hash: content_hash(&n.kind, &n.text),
            })
            .collect();
        DocumentView {
            revision: self.revision,
            nodes,
        }
    }

    fn is_within(&self, mut id: u64, root: u64) -> bool {
        loop {
            if id == root {
                return true;
            }
            match self
                .nodes
                .iter()
                .find(|n| n.id == id)
                .and_then(|n| n.parent)
            {
                Some(parent) => id = parent,
                None => return false,
            }
        }
    }

    /// `dom_view::apply` for the cases a guard run reaches: a stale revision is
    /// a conflict, an unknown node rejects the batch, an empty batch leaves the
    /// revision alone, otherwise every change applies and the revision advances.
    fn apply(&mut self, expected: u64, changes: &[Mutation]) -> Applied {
        if expected != self.revision {
            return Applied::Conflict(self.revision);
        }
        for change in changes {
            let named = match change {
                Mutation::SetText(a) => a.node,
                Mutation::Remove(id) => *id,
                Mutation::InsertBefore(a) => a.reference,
                Mutation::AppendChild(a) => a.parent,
            };
            if !self.nodes.iter().any(|n| n.id == named) {
                return Applied::UnknownNode(named);
            }
        }
        if changes.is_empty() {
            return Applied::Applied(self.revision);
        }
        for change in changes {
            match change {
                Mutation::SetText(a) => {
                    if let Some(n) = self.nodes.iter_mut().find(|n| n.id == a.node) {
                        if n.kind == "#text" {
                            n.text = a.text.clone();
                        }
                    }
                },
                Mutation::Remove(id) => {
                    let doomed: Vec<u64> = self
                        .nodes
                        .iter()
                        .filter(|n| self.is_within(n.id, *id))
                        .map(|n| n.id)
                        .collect();
                    self.nodes.retain(|n| !doomed.contains(&n.id));
                },
                Mutation::InsertBefore(a) => {
                    let at = self.nodes.iter().position(|n| n.id == a.reference);
                    if let Some(at) = at {
                        let parent = self.nodes[at].parent;
                        let id = self.next_id;
                        self.next_id += 1;
                        let node = Node {
                            id,
                            parent,
                            kind: a.new.kind.clone(),
                            text: a.new.text.clone(),
                        };
                        self.nodes.insert(at, node);
                    }
                },
                Mutation::AppendChild(a) => {
                    self.create(Some(a.parent), &a.new.kind, &a.new.text);
                },
            }
        }
        self.revision += 1;
        Applied::Applied(self.revision)
    }
}

/// How the fixture document took a batch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Applied {
    Applied(u64),
    Conflict(u64),
    UnknownNode(u64),
}

/// What guards a turn runs under. `None` leaves a guard off.
#[derive(Debug, Clone)]
pub struct Limits {
    /// Wasmtime fuel for the whole turn (activate and the event).
    pub fuel: Option<u64>,
    /// Epochs the turn may see before its deadline. Needs something to tick.
    pub epoch_deadline: Option<u64>,
    /// Tick the engine's epoch on every Nth host import call (0 = never). One
    /// thread has no watchdog, so the host's own imports are the only clock.
    pub tick_every_imports: u64,
    /// A `StoreLimits` cap on linear memory, in bytes.
    pub memory_cap: Option<usize>,
    pub origins: Vec<String>,
    pub max_fetches_per_turn: u32,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            fuel: None,
            epoch_deadline: None,
            tick_every_imports: 0,
            memory_cap: None,
            origins: vec!["example.com".into()],
            max_fetches_per_turn: 32,
        }
    }
}

/// `StoreLimits` with a record of what it did: the largest memory it allowed
/// and how many growths it denied. The cap itself is `StoreLimits`'.
struct Watched {
    inner: StoreLimits,
    peak_bytes: usize,
    denied: u32,
}

impl ResourceLimiter for Watched {
    fn memory_growing(
        &mut self,
        current: usize,
        desired: usize,
        maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        let allowed = self.inner.memory_growing(current, desired, maximum)?;
        if allowed {
            self.peak_bytes = self.peak_bytes.max(desired);
        } else {
            self.denied += 1;
        }
        Ok(allowed)
    }

    fn table_growing(
        &mut self,
        current: usize,
        desired: usize,
        maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        self.inner.table_growing(current, desired, maximum)
    }
}

/// The store data of one script.
pub struct ScriptHost {
    pub doc: Document,
    pub logs: Vec<String>,
    limits: Watched,
    engine: Engine,
    origins: Vec<String>,
    max_fetches: u32,
    fetches_this_turn: u32,
    tick_every: u64,
    import_calls: u64,
}

impl ScriptHost {
    /// Counts an import call and ticks the epoch every Nth.
    fn imported(&mut self) {
        self.import_calls += 1;
        if self.tick_every != 0 && self.import_calls % self.tick_every == 0 {
            self.engine.increment_epoch();
        }
    }
}

impl mere::script::log::Host for ScriptHost {
    fn log(&mut self, message: String) {
        self.imported();
        self.logs.push(message);
    }
}

impl mere::script::caps::Host for ScriptHost {
    fn granted(&mut self) -> Vec<String> {
        self.imported();
        vec![
            "mere:script/log".into(),
            "mere:script/document-host".into(),
            "mere:script/net".into(),
        ]
    }
}

impl mere::script::net::Host for ScriptHost {
    // The per-turn count first, then the origin, then the echo fetcher, in
    // `document-host`'s order and words.
    fn fetch(
        &mut self,
        req: mere::script::net::Request,
    ) -> Result<mere::script::net::Response, String> {
        self.imported();
        if self.fetches_this_turn >= self.max_fetches {
            return Err(format!(
                "net.fetch: per-turn request limit reached ({})",
                self.max_fetches
            ));
        }
        self.fetches_this_turn += 1;
        if !host_in_origins(&req.url, &self.origins) {
            return Err(format!(
                "net.fetch: {} is outside this script's net origins",
                req.url
            ));
        }
        Ok(mere::script::net::Response {
            status: 200,
            content_type: Some("text/plain".into()),
            body: format!("fetched:{}", req.url),
        })
    }
}

impl mere::script::document_host::Host for ScriptHost {
    fn inspect(&mut self, query: DocumentQuery) -> DocumentView {
        self.imported();
        self.doc.view(&query)
    }
}

/// How a turn ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The guest returned a batch and the document took it.
    Applied(u64),
    /// The guest returned a batch the document refused.
    Rejected(Applied),
    /// The guest returned a turn error (its own refusal, such as a fetch).
    Refused(String),
    /// The turn trapped: fuel, an epoch deadline, a memory cap or the guest.
    Trapped(String),
}

pub struct Turn {
    pub outcome: Outcome,
    /// Fuel left when the turn ended, when fuel was on.
    pub fuel_left: Option<u64>,
    /// The largest linear memory the limiter allowed, in bytes.
    pub memory_peak_bytes: usize,
    /// Growths the `StoreLimits` cap denied.
    pub growths_denied: u32,
    pub text: String,
    pub revision: u64,
}

/// The linker for the document guests, with the WASI stubs, and the stubs'
/// output (the guest's standard error).
pub fn linker(engine: &Engine) -> wasmtime::Result<(Linker<ScriptHost>, crate::stubs::StubOutput)> {
    let mut linker = Linker::new(engine);
    let output = crate::stubs::add_wasi_stubs(&mut linker)?;
    mere::script::log::add_to_linker::<ScriptHost, HasSelf<ScriptHost>>(&mut linker, |s| s)?;
    mere::script::caps::add_to_linker::<ScriptHost, HasSelf<ScriptHost>>(&mut linker, |s| s)?;
    mere::script::net::add_to_linker::<ScriptHost, HasSelf<ScriptHost>>(&mut linker, |s| s)?;
    mere::script::document_host::add_to_linker::<ScriptHost, HasSelf<ScriptHost>>(
        &mut linker,
        |s| s,
    )?;
    Ok((linker, output))
}

/// A script that is attached: its store and guest persist across turns, as a
/// `DocumentScript` does. A trap ends it, as natively: the instance is not
/// entered again.
pub struct Script {
    store: Store<ScriptHost>,
    guest: DocumentCore,
    limits: Limits,
    ended: bool,
}

/// What "no fuel limit" and "no epoch deadline" mean: budgets no turn reaches.
const UNLIMITED_FUEL: u64 = i64::MAX as u64;
const UNLIMITED_EPOCHS: u64 = u64::MAX / 2;

/// A turn that ended in a trap.
fn trapped(error: &wasmtime::Error) -> Outcome {
    Outcome::Trapped(match error.downcast_ref::<wasmtime::Trap>() {
        Some(trap) => trap.to_string(),
        None => format!("{error:#}"),
    })
}

impl Script {
    /// A fresh store under `limits`, the guest instantiated and activated.
    /// Fuel and epochs are always compiled in (`config::guarded`): "off" is a
    /// budget no turn reaches.
    pub fn attach(
        engine: &Engine,
        linker: &Linker<ScriptHost>,
        component: &Component,
        limits: &Limits,
    ) -> wasmtime::Result<Script> {
        let mut builder = StoreLimitsBuilder::new();
        if let Some(cap) = limits.memory_cap {
            builder = builder.memory_size(cap);
        }
        let watched = Watched {
            inner: builder.build(),
            peak_bytes: 0,
            denied: 0,
        };
        let host = ScriptHost {
            doc: Document::page(),
            logs: Vec::new(),
            limits: watched,
            engine: engine.clone(),
            origins: limits.origins.clone(),
            max_fetches: limits.max_fetches_per_turn,
            fetches_this_turn: 0,
            tick_every: limits.tick_every_imports,
            import_calls: 0,
        };
        let mut store = Store::new(engine, host);
        store.limiter(|h| &mut h.limits);
        store.set_fuel(limits.fuel.unwrap_or(UNLIMITED_FUEL))?;
        store.set_epoch_deadline(UNLIMITED_EPOCHS);
        let guest = DocumentCore::instantiate(&mut store, component, linker)?;
        guest.call_activate(&mut store)?;
        Ok(Script {
            store,
            guest,
            limits: limits.clone(),
            ended: false,
        })
    }

    /// The fuel each turn is given, when fuel is limited.
    pub fn fuel_limit(&self) -> Option<u64> {
        self.limits.fuel
    }

    /// Delivers one event under a fresh per-turn budget (fuel, epoch deadline
    /// and fetch count), and applies the batch the guest returns.
    pub fn deliver(&mut self, kind: &str, payload: &str) -> Turn {
        let fuel = self.limits.fuel.unwrap_or(UNLIMITED_FUEL);
        let outcome = if self.ended {
            Outcome::Trapped("the script was ended by an earlier trap".into())
        } else {
            self.store.data_mut().fetches_this_turn = 0;
            self.store.set_fuel(fuel).expect("fuel is on");
            self.store
                .set_epoch_deadline(self.limits.epoch_deadline.unwrap_or(UNLIMITED_EPOCHS));
            let event = mere::script::document::Event {
                kind: kind.into(),
                payload: payload.into(),
            };
            match self.guest.call_handle_event(&mut self.store, &event) {
                Err(error) => {
                    self.ended = true;
                    trapped(&error)
                },
                Ok(Err(turn_error)) => Outcome::Refused(match turn_error {
                    mere::script::document::TurnError::Refused(why) => why,
                    other => format!("{other:?}"),
                }),
                Ok(Ok(batch)) => {
                    let doc = &mut self.store.data_mut().doc;
                    match doc.apply(batch.expected_revision, &batch.changes) {
                        Applied::Applied(revision) => Outcome::Applied(revision),
                        other => Outcome::Rejected(other),
                    }
                },
            }
        };
        let host = self.store.data();
        Turn {
            outcome,
            fuel_left: self
                .store
                .get_fuel()
                .ok()
                .filter(|_| self.limits.fuel.is_some()),
            memory_peak_bytes: host.limits.peak_bytes,
            growths_denied: host.limits.denied,
            text: host.doc.texts(),
            revision: host.doc.revision,
        }
    }
}

/// One turn, as `run_guarded` runs it: attach under `limits`, then deliver
/// `kind` with `payload`. A trap while attaching is reported as a trapped turn.
pub fn run_turn(
    engine: &Engine,
    linker: &Linker<ScriptHost>,
    component: &Component,
    kind: &str,
    payload: &str,
    limits: &Limits,
) -> Turn {
    match Script::attach(engine, linker, component, limits) {
        Ok(mut script) => script.deliver(kind, payload),
        Err(error) => Turn {
            outcome: trapped(&error),
            fuel_left: None,
            memory_peak_bytes: 0,
            growths_denied: 0,
            text: Document::page().texts(),
            revision: 0,
        },
    }
}
