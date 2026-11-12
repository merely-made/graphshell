//! P0/P1 direct-Rust guest component for the DocumentScript substrate probe.
//!
//! Implements the `document-core` world's per-turn `handle-event` (§10.2): the
//! host hands it the current snapshot + an event; it returns an atomic,
//! revision-checked batch of id-targeted mutations (§10.3). A direct-Rust
//! component, not an interpreter — it proves the contract end to end (plan §6 P0,
//! extended to the §10.3 transaction contract). Uses only its one granted `log`
//! capability.
//!
//! Node identity (§10.3): nodes are addressed by the host's opaque `node-id`
//! (stable across edits and moves, capability-scoped). `content-hash` in the view
//! is a change-detection token the script *reads*, never an address. Positional
//! ops use id-relative anchors (parent / sibling ids), not index paths.

wit_bindgen::generate!({
    path: "../wit",
    world: "document-core",
});

// `event`, `apply-batch`, `turn-error` land at the crate root via the world's
// `use document.{...}`; the rest (incl. `document-view`, now reached through the
// `inspect` import rather than a param) stay qualified under the interface.
use crate::mere::script::document::{
    AppendArgs, Block, DocumentQuery, DocumentView, InsertArgs, Mutation, SetTextArgs, ViewNode,
};
use crate::mere::script::document_host::inspect;
use crate::mere::script::log::log;

struct Component;

fn first_paragraph(view: &DocumentView) -> Option<&ViewNode> {
    view.nodes.iter().find(|n| n.kind == "paragraph")
}

fn root(view: &DocumentView) -> Option<&ViewNode> {
    view.nodes.iter().find(|n| n.parent.is_none())
}

fn para(text: String) -> Block {
    Block { kind: "paragraph".to_string(), text }
}

impl Guest for Component {
    fn activate() {
        log("guest: activated");
    }

    fn deactivate() {
        log("guest: deactivated");
    }

    fn handle_event(ev: Event) -> Result<ApplyBatch, TurnError> {
        let Event { kind, payload } = ev;
        // Pull the snapshot on demand via the host import (a re-entrant call back
        // into the host), rather than receiving it as a param. The script chooses
        // what and when to copy.
        let view = inspect(DocumentQuery::WholeDocument);
        log(&format!(
            "guest: turn '{kind}' at rev {} ({} nodes pulled)",
            view.revision,
            view.nodes.len()
        ));

        let changes = match kind.as_str() {
            // Target a node by its opaque id; read (do not address by) its
            // content-hash to show the change-detection token crossing the seam.
            "set" => {
                let p = first_paragraph(&view)
                    .ok_or_else(|| TurnError::Refused("no paragraph to edit".to_string()))?;
                log(&format!(
                    "  -> set-text on node {} (content-hash {:#018x})",
                    p.id, p.content_hash
                ));
                vec![Mutation::SetText(SetTextArgs { node: p.id, text: payload })]
            }
            // Id-relative anchor: append under the root's id (not an index path).
            "append" => {
                let r =
                    root(&view).ok_or_else(|| TurnError::Refused("no root".to_string()))?;
                vec![Mutation::AppendChild(AppendArgs { parent: r.id, new: para(payload) })]
            }
            // Id-relative anchor: insert before a sibling id.
            "insert" => {
                let p = first_paragraph(&view)
                    .ok_or_else(|| TurnError::Refused("no paragraph".to_string()))?;
                vec![Mutation::InsertBefore(InsertArgs { reference: p.id, new: para(payload) })]
            }
            // Remove the last child by id (the host removes its subtree).
            "remove" => {
                let last = view
                    .nodes
                    .iter()
                    .rev()
                    .find(|n| n.parent.is_some())
                    .ok_or_else(|| TurnError::Refused("nothing to remove".to_string()))?;
                vec![Mutation::Remove(last.id)]
            }
            // Cite a stale revision: demonstrates the host rejecting with the
            // current revision so the script could rebase (§10.3).
            "stale" => {
                let p = first_paragraph(&view)
                    .ok_or_else(|| TurnError::Refused("no paragraph".to_string()))?;
                return Ok(ApplyBatch {
                    expected_revision: view.revision.saturating_sub(1),
                    changes: vec![Mutation::SetText(SetTextArgs {
                        node: p.id,
                        text: "stale edit".to_string(),
                    })],
                });
            }
            // Reference a node the host does not know: demonstrates atomic
            // validation (unknown-node, nothing applied).
            "bad-id" => vec![Mutation::SetText(SetTextArgs {
                node: u64::MAX,
                text: "ghost".to_string(),
            })],
            // Scoped pull: inspect only the root's subtree, a smaller copy than
            // whole-document. A no-op turn (empty batch).
            "subtree" => {
                if let Some(r) = root(&view) {
                    let sub = inspect(DocumentQuery::Subtree(r.id));
                    log(&format!(
                        "  scoped inspect subtree({}) -> {} of {} nodes",
                        r.id,
                        sub.nodes.len(),
                        view.nodes.len()
                    ));
                }
                vec![]
            }
            other => return Err(TurnError::Refused(format!("unknown event kind: {other}"))),
        };

        Ok(ApplyBatch { expected_revision: view.revision, changes })
    }
}

export!(Component);
