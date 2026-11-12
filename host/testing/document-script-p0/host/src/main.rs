//! P0/P1 host runner for the DocumentScript substrate probe.
//!
//! Loads the direct-Rust `document-core` component, grants one capability (`log`)
//! + the WASI floor, and drives host-authoritative turns of the §10.3 transaction
//! contract: it owns the document and its revision, hands the script a copied
//! snapshot (each node carrying an opaque stable `node-id` + a content-hash),
//! receives an atomic batch of id-targeted mutations, and applies it against the
//! cited revision — minting ids for new nodes, rejecting a stale revision with the
//! current one, and rejecting an unknown node-id before any change lands.
//!
//! Synchronous (§10.8 sync probe; Wasmtime 45.0.2). Node identity is opaque +
//! host-assigned (§10.3): the script can only name nodes the host put in the
//! snapshot, so it cannot reach outside its grant.

use std::path::PathBuf;

use wasmtime::component::{Component, HasSelf, Linker, ResourceTable};
use wasmtime::{Engine, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

wasmtime::component::bindgen!({
    path: "../wit",
    world: "document-core",
});

use crate::mere::script::document::{
    AppendArgs, DocumentQuery, DocumentView, InsertArgs, Mutation, SetTextArgs, ViewNode,
};

/// A node in the host's authoritative document. `id` is the opaque identity the
/// script addresses; it is assigned once and never reused.
struct DocNode {
    id: u64,
    parent: Option<u64>,
    kind: String,
    text: String,
}

/// The host-owned document. The script never holds this; it sees only copied
/// snapshots and references nodes by `id`.
struct Doc {
    nodes: Vec<DocNode>, // document order
    revision: u64,
    next_id: u64,
}

impl Doc {
    /// A small starting document: a root with two paragraphs.
    fn new() -> Self {
        Self {
            nodes: vec![
                DocNode { id: 0, parent: None, kind: "root".into(), text: String::new() },
                DocNode { id: 1, parent: Some(0), kind: "paragraph".into(), text: "Intro paragraph.".into() },
                DocNode { id: 2, parent: Some(0), kind: "paragraph".into(), text: "Second paragraph.".into() },
            ],
            revision: 0,
            next_id: 3,
        }
    }

    fn snapshot(&self) -> DocumentView {
        DocumentView {
            revision: self.revision,
            nodes: self
                .nodes
                .iter()
                .map(|n| ViewNode {
                    id: n.id,
                    parent: n.parent,
                    kind: n.kind.clone(),
                    text: n.text.clone(),
                    content_hash: content_hash(&n.kind, &n.text),
                })
                .collect(),
        }
    }

    /// A scoped snapshot: only `root` and its descendants (the `subtree` query).
    /// Carries the current revision so a batch computed from it cites correctly.
    fn snapshot_subtree(&self, root: u64) -> DocumentView {
        let ids = self.subtree(root);
        DocumentView {
            revision: self.revision,
            nodes: self
                .nodes
                .iter()
                .filter(|n| ids.contains(&n.id))
                .map(|n| ViewNode {
                    id: n.id,
                    parent: n.parent,
                    kind: n.kind.clone(),
                    text: n.text.clone(),
                    content_hash: content_hash(&n.kind, &n.text),
                })
                .collect(),
        }
    }

    fn index_of(&self, id: u64) -> Option<usize> {
        self.nodes.iter().position(|n| n.id == id)
    }

    /// `id` plus every node reachable downward from it (for subtree removal).
    fn subtree(&self, id: u64) -> Vec<u64> {
        let mut out = vec![id];
        let mut i = 0;
        while i < out.len() {
            let p = out[i];
            for n in &self.nodes {
                if n.parent == Some(p) && !out.contains(&n.id) {
                    out.push(n.id);
                }
            }
            i += 1;
        }
        out
    }

    /// Apply a batch atomically against its cited revision. Validates every
    /// referenced id before mutating, so a bad reference leaves the document
    /// untouched. Returns the new revision on success.
    fn apply(&mut self, batch: ApplyBatch) -> Result<u64, TurnError> {
        if batch.expected_revision != self.revision {
            return Err(TurnError::RevisionConflict(self.revision));
        }
        // Atomic precheck: an existing-node reference that the host does not know
        // fails the whole batch before anything applies.
        for m in &batch.changes {
            let referenced = match m {
                Mutation::SetText(a) => Some(a.node),
                Mutation::Remove(id) => Some(*id),
                Mutation::InsertBefore(a) => Some(a.reference),
                Mutation::AppendChild(a) => Some(a.parent),
            };
            if let Some(id) = referenced {
                if self.index_of(id).is_none() {
                    return Err(TurnError::UnknownNode(id));
                }
            }
        }
        if batch.changes.is_empty() {
            return Ok(self.revision); // a no-op turn does not advance the revision
        }
        for m in batch.changes {
            self.apply_one(m);
        }
        self.revision += 1;
        Ok(self.revision)
    }

    fn apply_one(&mut self, m: Mutation) {
        match m {
            Mutation::SetText(SetTextArgs { node, text }) => {
                if let Some(i) = self.index_of(node) {
                    self.nodes[i].text = text;
                }
            }
            Mutation::Remove(id) => {
                let gone = self.subtree(id);
                self.nodes.retain(|n| !gone.contains(&n.id));
            }
            Mutation::InsertBefore(InsertArgs { reference, new }) => {
                if let Some(i) = self.index_of(reference) {
                    let parent = self.nodes[i].parent;
                    let id = self.mint();
                    self.nodes.insert(
                        i,
                        DocNode { id, parent, kind: new.kind, text: new.text },
                    );
                }
            }
            Mutation::AppendChild(AppendArgs { parent, new }) => {
                let id = self.mint();
                self.nodes.push(DocNode {
                    id,
                    parent: Some(parent),
                    kind: new.kind,
                    text: new.text,
                });
            }
        }
    }

    fn mint(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn dump(&self) {
        for n in &self.nodes {
            let indent = if n.parent.is_some() { "  " } else { "" };
            println!("    {indent}[{}] {} {:?}", n.id, n.kind, n.text);
        }
    }
}

/// FNV-1a over kind + text. A deterministic stand-in for the change-detection
/// token; a real impl uses a subtree Merkle hash.
fn content_hash(kind: &str, text: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in kind.bytes().chain(std::iter::once(0)).chain(text.bytes()) {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Host state: the document plus the WASI floor the std guest needs.
struct HostState {
    doc: Doc,
    wasi: WasiCtx,
    table: ResourceTable,
}

impl crate::mere::script::log::Host for HostState {
    fn log(&mut self, message: String) {
        println!("    [cap:log] {message}");
    }
}

/// The `inspect` capability: the guest calls this back during a turn to pull a
/// (scoped) copied snapshot. Only nodes the host puts in the view are nameable.
impl crate::mere::script::document_host::Host for HostState {
    fn inspect(&mut self, query: DocumentQuery) -> DocumentView {
        match query {
            DocumentQuery::WholeDocument => self.doc.snapshot(),
            DocumentQuery::Subtree(id) => self.doc.snapshot_subtree(id),
        }
    }
}

impl WasiView for HostState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView { ctx: &mut self.wasi, table: &mut self.table }
    }
}

fn main() -> wasmtime::Result<()> {
    let wasm_path = std::env::args().nth(1).map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from("../guest/target/wasm32-wasip2/release/document_script_p0_guest.wasm")
    });
    println!("loading component: {}", wasm_path.display());

    let engine = Engine::default();
    let component = Component::from_file(&engine, &wasm_path)?;

    let mut linker = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;
    crate::mere::script::log::add_to_linker::<HostState, HasSelf<HostState>>(&mut linker, |s| s)?;
    crate::mere::script::document_host::add_to_linker::<HostState, HasSelf<HostState>>(
        &mut linker,
        |s| s,
    )?;

    let mut store = Store::new(
        &engine,
        HostState {
            doc: Doc::new(),
            wasi: WasiCtxBuilder::new().build(),
            table: ResourceTable::new(),
        },
    );
    let bindings = DocumentCore::instantiate(&mut store, &component, &linker)?;

    println!("\nactivating script (lifecycle):");
    bindings.call_activate(&mut store)?;

    println!("\ninitial document (rev 0):");
    store.data().doc.dump();

    // (event kind, payload). Covers all four id-targeted mutations, the stale
    // revision (conflict), and an unknown node-id, plus a guest-side refusal.
    let turns = [
        ("set", "Edited intro via node-id."),
        ("append", "Appended under root's id."),
        ("insert", "Inserted before a sibling id."),
        ("subtree", ""),
        ("remove", ""),
        ("stale", ""),
        ("bad-id", ""),
        ("frobnicate", ""),
    ];

    for (kind, payload) in turns {
        let rev = store.data().doc.revision;
        let ev = Event { kind: kind.to_string(), payload: payload.to_string() };
        println!("\n-> turn at rev {rev}: event '{kind}'");

        match bindings.call_handle_event(&mut store, &ev)? {
            Ok(batch) => {
                let cited = batch.expected_revision;
                match store.data_mut().doc.apply(batch) {
                    Ok(new_rev) => println!("    applied (cited rev {cited}); revision now {new_rev}"),
                    Err(TurnError::RevisionConflict(cur)) => {
                        println!("    host rejected: revision-conflict (cited {cited}, current {cur})")
                    }
                    Err(TurnError::UnknownNode(id)) => {
                        println!("    host rejected: unknown-node {id} (nothing applied)")
                    }
                    Err(TurnError::Refused(why)) => println!("    host rejected: {why}"),
                }
            }
            Err(TurnError::Refused(why)) => println!("    guest declined: refused ({why})"),
            Err(TurnError::RevisionConflict(cur)) => {
                println!("    guest returned revision-conflict (current {cur})")
            }
            Err(TurnError::UnknownNode(id)) => {
                println!("    guest returned unknown-node {id}")
            }
        }
    }

    println!("\ndeactivating script (lifecycle):");
    bindings.call_deactivate(&mut store)?;

    println!("\nfinal document (rev {}):", store.data().doc.revision);
    store.data().doc.dump();
    Ok(())
}
