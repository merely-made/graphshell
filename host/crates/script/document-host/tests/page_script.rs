// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Mod projects MP5b: a mod on a copy of a live page, its turns replayed into
//! the page by identity (D15, D22 to D24). The page here is a `ScriptedDom` with a
//! small `PageSurface` that applies the replayed mutations itself; the real
//! engine path (genet's `apply_host_mutations`) is the headed Turnstone receipt.

use std::path::PathBuf;

use document_host::{
    Grant, PageScript, PageSurface, Quota, TurnOutcome, page_mutation_count,
};
use genet_scripted_dom::{NodeId, ScriptedDom};
use inker::session_engine::{
    HostMutation, HostMutationOp, HostMutationReport, HostNewNode, HostTarget,
};
use layout_dom_api::{LayoutDom, LayoutDomMut, LocalName, Namespace, NodeKind, QualName};

fn doc_wasm() -> PathBuf {
    let p = std::env::var("DOC_HOST_GUEST_WASM").unwrap_or_else(|_| {
        "guest/target/wasm32-wasip2/release/document_core_guest.wasm".to_string()
    });
    let path = PathBuf::from(&p);
    assert!(path.exists(), "guest component missing at {p}");
    path
}

fn qual(local: &str) -> QualName {
    QualName::new(None, Namespace::from(""), LocalName::from(local))
}

/// A page and the surface through which a mod's replay reaches it.
struct Page {
    dom: ScriptedDom,
    applied: Vec<HostMutation>,
}

impl Page {
    fn new(html: &str) -> Self {
        Self {
            dom: ScriptedDom::from_serialized_document(html),
            applied: Vec::new(),
        }
    }

    fn find(&self, tag: &str) -> NodeId {
        fn go(dom: &ScriptedDom, n: NodeId, tag: &str) -> Option<NodeId> {
            if dom.kind(n) == NodeKind::Element && dom.element_name(n).is_some_and(|q| &*q.local == tag) {
                return Some(n);
            }
            dom.dom_children(n).find_map(|c| go(dom, c, tag))
        }
        go(&self.dom, self.dom.document(), tag).expect("tag is on the page")
    }

    fn first_text(&self) -> NodeId {
        fn go(dom: &ScriptedDom, n: NodeId) -> Option<NodeId> {
            if dom.kind(n) == NodeKind::Text {
                return Some(n);
            }
            dom.dom_children(n).find_map(|c| go(dom, c))
        }
        go(&self.dom, self.dom.document()).expect("a text node")
    }
}

impl PageSurface for Page {
    fn dom(&self) -> &ScriptedDom {
        &self.dom
    }

    fn apply(&mut self, mutations: &[HostMutation]) -> Result<HostMutationReport, String> {
        let mut report = HostMutationReport::default();
        for m in mutations {
            let HostTarget::Node(raw) = &m.target else {
                return Err("the test surface takes identity targets only".into());
            };
            let node = NodeId::from_raw(*raw);
            if !self.dom.is_live(node) {
                report.missed += 1;
                continue;
            }
            match &m.op {
                HostMutationOp::SetText { text } => self.dom.set_text(node, text),
                HostMutationOp::Remove => self.dom.remove(node),
                HostMutationOp::InsertBefore { node: new } => {
                    let parent = self.dom.parent(node).expect("a parent");
                    let child = match new {
                        HostNewNode::Text(t) => self.dom.create_text(t),
                        HostNewNode::Comment(t) => self.dom.create_comment(t),
                        HostNewNode::Element { tag, .. } => self.dom.create_element(qual(tag)),
                    };
                    self.dom.insert_before(parent, child, Some(node));
                },
                HostMutationOp::AppendNode { node: new } => {
                    let child = match new {
                        HostNewNode::Text(t) => self.dom.create_text(t),
                        HostNewNode::Comment(t) => self.dom.create_comment(t),
                        HostNewNode::Element { tag, .. } => self.dom.create_element(qual(tag)),
                    };
                    self.dom.append_child(node, child);
                },
                other => return Err(format!("unexpected op {other:?}")),
            }
            report.applied += 1;
        }
        self.applied.extend_from_slice(mutations);
        Ok(report)
    }
}

const HTML: &str = "<html><head></head><body><p>Hello</p></body></html>";

fn attach(page: &Page) -> PageScript {
    PageScript::attach(
        &doc_wasm(),
        &page.dom,
        &Grant::allow_all(),
        Quota::default(),
        None,
        Vec::new(),
    )
    .expect("attach")
}

#[test]
fn a_turn_replays_into_the_page_by_identity() {
    let mut page = Page::new(HTML);
    let mut script = attach(&page);
    let text = page.first_text();
    let before = page_mutation_count(&page.dom);

    let out = script.deliver_event(&mut page, "set", "Edited by a mod").unwrap();
    assert_eq!(out, TurnOutcome::Applied(1));
    assert_eq!(page.dom.text(text), Some("Edited by a mod"));
    assert_eq!(page.applied.len(), 1);
    assert!(matches!(page.applied[0].target, HostTarget::Node(_)));
    assert!(page_mutation_count(&page.dom) > before);
    // The copy and the page agree after the replay.
    assert_eq!(script.dom().inner_html(script.dom().document()), page.dom.inner_html(page.dom.document()));
}

#[test]
fn the_mods_own_replays_do_not_move_the_page_and_new_nodes_stay_mapped() {
    let mut page = Page::new(HTML);
    let mut script = attach(&page);
    let body = page.find("body");

    assert_eq!(script.deliver_event(&mut page, "append", "").unwrap(), TurnOutcome::Applied(1));
    assert_eq!(page.dom.dom_children(body).count(), 2);
    // A second and third turn: the replay of the first did not count as the page moving,
    // and the set still finds its node after the map was rebuilt over the appended <p>.
    assert_eq!(script.deliver_event(&mut page, "append", "").unwrap(), TurnOutcome::Applied(2));
    assert_eq!(script.deliver_event(&mut page, "set", "after appends").unwrap(), TurnOutcome::Applied(3));
    assert_eq!(page.dom.dom_children(body).count(), 3);
    assert_eq!(page.dom.text(page.first_text()), Some("after appends"));
}

#[test]
fn a_turn_against_a_page_that_changed_underneath_is_refused() {
    let mut page = Page::new(HTML);
    let mut script = attach(&page);
    let text = page.first_text();

    // The page changes by something other than the mod.
    page.dom.set_text(text, "The page's own script");
    assert!(script.page_moved(&page.dom));
    let out = script.deliver_event(&mut page, "set", "mod text").unwrap();
    assert_eq!(out, TurnOutcome::PageMoved);
    assert_eq!(page.dom.text(text), Some("The page's own script"), "the page keeps its own change");
    assert!(page.applied.is_empty(), "nothing reached the page");
    assert_eq!(script.revision(), 0, "nothing reached the copy");
}

#[test]
fn an_attach_whose_copy_differs_from_the_page_is_refused_naming_where() {
    // Two adjacent text nodes serialize as one, so the re-parsed copy has fewer children.
    let mut page = Page::new(HTML);
    let p = page.find("p");
    let extra = page.dom.create_text(" world");
    page.dom.append_child(p, extra);
    let err = PageScript::attach(
        &doc_wasm(),
        &page.dom,
        &Grant::allow_all(),
        Quota::default(),
        None,
        Vec::new(),
    )
    .err()
    .expect("a differing copy is refused");
    assert!(err.contains("the page copy differs"), "{err}");
    assert!(err.contains("<p>"), "it names where: {err}");
}

#[test]
fn detaching_leaves_the_page_as_the_last_turn_made_it() {
    let mut page = Page::new(HTML);
    let mut script = attach(&page);
    script.deliver_event(&mut page, "set", "Last word").unwrap();
    script.detach().expect("detach");
    assert_eq!(page.dom.text(page.first_text()), Some("Last word"));
}

#[test]
fn a_refused_turn_changes_neither_page_nor_copy() {
    let mut page = Page::new(HTML);
    let mut script = attach(&page);
    let out = script.deliver_event(&mut page, "bad-id", "").unwrap();
    assert!(matches!(out, TurnOutcome::UnknownNode(_) | TurnOutcome::Refused(_)), "{out:?}");
    assert!(page.applied.is_empty());
    assert_eq!(script.revision(), 0);
}
