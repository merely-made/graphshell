// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! A mod on a live page (mod projects MP5b; D15, D22 to D24).
//!
//! The mod runs on a copy of the page: `inner_html` of the page's document,
//! re-parsed into a fresh `ScriptedDom`. A parallel walk maps each copy node to
//! its page node (the arenas differ, so no id is shared), and an attach is
//! refused, naming why, when the two shapes differ. Each applied turn is
//! translated against that map into identity-targeted host mutations and
//! applied to the page through a [`PageSurface`]; the map is rebuilt by the same
//! walk afterwards, which also proves the replay landed. A turn is refused when
//! the page's DOM mutation count moved by anything but this mod's own replays.

use std::collections::HashMap;
use std::path::Path;

use genet_scripted_dom::{NodeId, ScriptedDom};
use inker::session_engine::{HostMutation, HostMutationOp, HostMutationReport, HostNewNode};
use layout_dom_api::{LayoutDom, NodeKind};

use crate::mere::script::document::{AppendArgs, InsertArgs, Mutation, SetTextArgs};
use crate::{
    ApplyBatch, DocumentScript, Grant, NetFetcher, Quota, ReloadError, TurnOutcome, mod_loader,
};

/// The page a mod is attached to, as the embedder lends it: a read of its DOM
/// and the engine's own mutation path.
pub trait PageSurface {
    /// The page's live DOM.
    fn dom(&self) -> &ScriptedDom;
    /// Apply identity-targeted mutations through the engine's restyle path.
    ///
    /// # Errors
    ///
    /// Why the engine refused the batch.
    fn apply(&mut self, mutations: &[HostMutation]) -> Result<HostMutationReport, String>;
}

/// Every DOM change the page has recorded, drained or not (base plus pending).
#[must_use]
pub fn page_mutation_count(dom: &ScriptedDom) -> u64 {
    let (base, pending) = dom.pending_mutations();
    base + pending.len() as u64
}

fn label(dom: &ScriptedDom, node: NodeId) -> String {
    match dom.kind(node) {
        NodeKind::Element => dom
            .element_name(node)
            .map_or_else(|| "element".into(), |q| format!("<{}>", q.local)),
        other => format!("{other:?}").to_lowercase(),
    }
}

/// The copy-to-page node map and the page's mutation count it was made at.
pub struct PageLink {
    /// Copy node identity to page node identity.
    map: HashMap<u64, u64>,
    /// The page's mutation count after the copy or the last replay.
    page_count: u64,
}

impl PageLink {
    /// Copy `page` and map the copy to it.
    ///
    /// # Errors
    ///
    /// Why the copy does not match the page: the first node where kind, tag,
    /// text or child count differs.
    pub fn copy(page: &ScriptedDom) -> Result<(ScriptedDom, Self), String> {
        let html = page.inner_html(page.document());
        let copy = ScriptedDom::from_serialized_document(&html);
        let mut link = Self {
            map: HashMap::new(),
            page_count: page_mutation_count(page),
        };
        link.resync(&copy, page)?;
        Ok((copy, link))
    }

    /// Rebuild the map by a parallel walk of `copy` and `page`.
    fn resync(&mut self, copy: &ScriptedDom, page: &ScriptedDom) -> Result<(), String> {
        let mut map = HashMap::new();
        walk(copy, copy.document(), page, page.document(), "document", &mut map)?;
        self.map = map;
        Ok(())
    }

    /// The page node a copy node stands for.
    fn page_node(&self, copy_node: u64) -> Option<u64> {
        self.map.get(&copy_node).copied()
    }

    /// Whether the page's DOM changed other than by this mod's replays.
    #[must_use]
    pub fn moved(&self, page: &ScriptedDom) -> bool {
        page_mutation_count(page) != self.page_count
    }
}

fn walk(
    copy: &ScriptedDom,
    c: NodeId,
    page: &ScriptedDom,
    p: NodeId,
    at: &str,
    map: &mut HashMap<u64, u64>,
) -> Result<(), String> {
    let ck = copy.kind(c);
    if ck != page.kind(p) || label(copy, c) != label(page, p) {
        return Err(format!(
            "the page copy differs at {at}: the page has {} where the copy has {}",
            label(page, p),
            label(copy, c)
        ));
    }
    if matches!(ck, NodeKind::Text | NodeKind::Comment) && copy.text(c) != page.text(p) {
        return Err(format!("the page copy differs at {at}: text differs"));
    }
    let cc: Vec<NodeId> = copy.dom_children(c).collect();
    let pc: Vec<NodeId> = page.dom_children(p).collect();
    if cc.len() != pc.len() {
        return Err(format!(
            "the page copy differs under {at}: the page has {} children where the copy has {}",
            pc.len(),
            cc.len()
        ));
    }
    map.insert(copy.opaque_id(c), page.opaque_id(p));
    for (i, (c, p)) in cc.into_iter().zip(pc).enumerate() {
        let name = format!("{at}/{}[{i}]", label(page, p));
        walk(copy, c, page, p, &name, map)?;
    }
    Ok(())
}

/// Translate a batch, against the copy as it stands before the batch, into
/// identity-targeted mutations on the page.
fn to_host_mutations(
    link: &PageLink,
    copy: &ScriptedDom,
    batch: &ApplyBatch,
) -> Result<Vec<HostMutation>, String> {
    let target = |raw: u64| {
        link.page_node(raw)
            .ok_or_else(|| format!("node {raw} has no counterpart on the page"))
    };
    let mut out = Vec::with_capacity(batch.changes.len());
    for change in &batch.changes {
        out.push(match change {
            Mutation::SetText(SetTextArgs { node, text }) => {
                let page_node = target(*node)?; // unmapped ids refuse before the copy is asked
                if !matches!(
                    copy.kind(NodeId::from_raw(*node)),
                    NodeKind::Text | NodeKind::Comment
                ) {
                    return Err(format!("set-text targets node {node}, which is not text"));
                }
                HostMutation::node(page_node, HostMutationOp::SetText { text: text.clone() })
            },
            Mutation::Remove(node) => HostMutation::node(target(*node)?, HostMutationOp::Remove),
            Mutation::InsertBefore(InsertArgs { reference, new }) => HostMutation::node(
                target(*reference)?,
                HostMutationOp::InsertBefore {
                    node: new_node(&new.kind, &new.text),
                },
            ),
            Mutation::AppendChild(AppendArgs { parent, new }) => HostMutation::node(
                target(*parent)?,
                HostMutationOp::AppendNode {
                    node: new_node(&new.kind, &new.text),
                },
            ),
        });
    }
    Ok(out)
}

fn new_node(kind: &str, text: &str) -> HostNewNode {
    if kind == "#text" {
        HostNewNode::Text(text.to_owned())
    } else {
        HostNewNode::Element {
            tag: kind.to_owned(),
            class: None,
            text: None,
        }
    }
}

/// A [`DocumentScript`] attached to a copy of a live page, replaying its turns.
pub struct PageScript {
    script: DocumentScript,
    link: PageLink,
}

impl PageScript {
    /// Copy `page` and attach `component_path` to the copy.
    ///
    /// # Errors
    ///
    /// Why the page could not be copied, or as [`DocumentScript::attach`].
    pub fn attach(
        component_path: &Path,
        page: &ScriptedDom,
        grant: &Grant,
        quota: Quota,
        fetcher: Option<std::sync::Arc<dyn NetFetcher>>,
        net_origins: Vec<String>,
    ) -> Result<Self, String> {
        let (copy, link) = PageLink::copy(page)?;
        let script = DocumentScript::attach(component_path, copy, grant, quota, fetcher, net_origins)
            .map_err(|e| format!("{e:#}"))?;
        Ok(Self { script, link })
    }

    /// Deliver one event. The guest's batch goes to the page first, through
    /// `page`, and to the copy once the page took all of it. `Err` means the
    /// guest trapped (detach it, D20); the page and the copy are unchanged.
    ///
    /// # Errors
    ///
    /// A guest trap.
    pub fn deliver_event(
        &mut self,
        page: &mut dyn PageSurface,
        kind: &str,
        payload: &str,
    ) -> wasmtime::Result<TurnOutcome> {
        if self.link.moved(page.dom()) {
            return Ok(TurnOutcome::PageMoved);
        }
        let batch = match self.script.call_event(kind, payload)? {
            Ok(batch) => batch,
            Err(e) => return Ok(DocumentScript::turn_error(e)),
        };
        if batch.expected_revision != self.script.revision() {
            return Ok(TurnOutcome::Conflict(self.script.revision()));
        }
        let mutations = match to_host_mutations(&self.link, self.script.dom(), &batch) {
            Ok(m) => m,
            Err(why) => return Ok(TurnOutcome::Refused(why)),
        };
        if mutations.is_empty() {
            return Ok(self.script.commit_batch(batch));
        }
        // Every target is in the map, so each is live on the copy: the page goes
        // first and the copy follows only once the page took the whole batch.
        match page.apply(&mutations) {
            Ok(report) if report.missed == 0 && report.applied == mutations.len() => {},
            Ok(report) => {
                return Ok(TurnOutcome::Refused(format!(
                    "the page took {} of {} changes; detach the mod",
                    report.applied,
                    mutations.len()
                )));
            },
            Err(why) => {
                return Ok(TurnOutcome::Refused(format!("the page refused the turn: {why}")));
            },
        }
        self.link.page_count = page_mutation_count(page.dom());
        let landed = match self.script.commit_batch(batch) {
            TurnOutcome::Applied(rev) => rev,
            other => return Ok(other),
        };
        if let Err(why) = self.link.resync(self.script.dom(), page.dom()) {
            return Ok(TurnOutcome::Refused(format!(
                "the copy and the page diverged: {why}"
            )));
        }
        Ok(TurnOutcome::Applied(landed))
    }

    /// The copy the mod sees.
    #[must_use]
    pub fn dom(&self) -> &ScriptedDom {
        self.script.dom()
    }

    /// The copy's revision.
    #[must_use]
    pub fn revision(&self) -> u64 {
        self.script.revision()
    }

    /// What the guest has logged.
    #[must_use]
    pub fn guest_logs(&self) -> &[String] {
        self.script.guest_logs()
    }

    /// Whether the page changed other than by this mod since its last replay.
    #[must_use]
    pub fn page_moved(&self, page: &ScriptedDom) -> bool {
        self.link.moved(page)
    }

    /// Swap in an edited component over the same copy; see [`DocumentScript::reload`].
    ///
    /// # Errors
    ///
    /// As [`DocumentScript::reload`].
    pub fn reload(
        &mut self,
        component_path: &Path,
        grant: &Grant,
        quota: Quota,
        fetcher: Option<std::sync::Arc<dyn NetFetcher>>,
        net_origins: Vec<String>,
        record: Option<&mut mod_loader::InstallRecord>,
    ) -> Result<(), ReloadError> {
        self.script
            .reload(component_path, grant, quota, fetcher, net_origins, record)
    }

    /// Run `deactivate` and drop the copy. The page keeps what the last applied
    /// turn made it (D14).
    ///
    /// # Errors
    ///
    /// A trap in `deactivate`; the page is unaffected either way.
    pub fn detach(self) -> wasmtime::Result<()> {
        self.script.detach().map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mere::script::document::Block;

    #[test]
    fn append_child_maps_every_node_kind_to_append_node() {
        let page = ScriptedDom::from_serialized_document("<html><head></head><body><p>x</p></body></html>");
        let (copy, link) = PageLink::copy(&page).unwrap();
        let body = copy
            .dom_children(copy.dom_children(copy.document()).next().unwrap())
            .nth(1)
            .unwrap();
        let parent = copy.opaque_id(body);
        let block = |kind: &str, text: &str| Block {
            kind: kind.into(),
            text: text.into(),
        };
        let batch = ApplyBatch {
            expected_revision: 0,
            changes: vec![
                Mutation::AppendChild(AppendArgs { parent, new: block("p", "") }),
                Mutation::AppendChild(AppendArgs { parent, new: block("#text", "tail") }),
            ],
        };
        let out = to_host_mutations(&link, &copy, &batch).expect("both kinds translate");
        let ops: Vec<_> = out.iter().map(|m| m.op.clone()).collect();
        assert_eq!(
            ops,
            [
                HostMutationOp::AppendNode {
                    node: HostNewNode::Element { tag: "p".into(), class: None, text: None }
                },
                HostMutationOp::AppendNode { node: HostNewNode::Text("tail".into()) },
            ]
        );
        assert!(out.iter().all(|m| matches!(m.target, inker::session_engine::HostTarget::Node(_))));
    }
}
