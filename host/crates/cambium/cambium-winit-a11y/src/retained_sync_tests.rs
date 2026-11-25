// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! A patched native publication must equal a whole one of the same tree: id
//! map, envelopes, routes and nodes (table plan D64).
use super::*;
use crate::publication::Prepared;
use cambium_rootstock::{OwnedLayout, SemanticProjection};
use genet_scripted_dom::ScriptedDom;
use layout_dom_api::{LayoutDomMut, LocalName, Namespace, QualName};
use std::cell::Cell;
use std::rc::Rc;

const SHEET: &str = ".strip { position: absolute; left: 0; top: 0; width: 400px; height: 4000px; } \
    .row { position: absolute; left: 0; width: 400px; height: 20px; display: flex; } \
    .row span { display: block; width: 60px; height: 20px; }";

fn name(local: &str) -> QualName {
    QualName::new(None, Namespace::from(""), LocalName::from(local))
}

struct Fixture {
    dom: ScriptedDom,
    strip: genet_scripted_dom::NodeId,
    rows: Vec<genet_scripted_dom::NodeId>,
    next: usize,
    leaves: LeafRegistry<u64>,
    producers: ProducerRegistry,
    marker: Rc<Cell<bool>>,
}

impl Fixture {
    fn new() -> Self {
        let mut dom = ScriptedDom::new();
        let strip = dom.create_element(name("div"));
        dom.set_attribute(strip, name("class"), "strip");
        dom.set_attribute(strip, name("role"), "rowgroup");
        dom.append_child(dom.document(), strip);
        let mut fixture = Self {
            dom,
            strip,
            rows: Vec::new(),
            next: 0,
            leaves: LeafRegistry::new(),
            producers: ProducerRegistry::new(),
            marker: Rc::new(Cell::new(true)),
        };
        for _ in 0..6 {
            fixture.push_row();
        }
        fixture
    }

    fn push_row(&mut self) {
        let row = self.dom.create_element(name("div"));
        self.dom.set_attribute(row, name("class"), "row");
        self.dom.set_attribute(row, name("role"), "row");
        self.dom.set_attribute(row, name("tabindex"), "0");
        self.dom
            .set_attribute(row, name("data-key"), &format!("row-{}", self.next));
        let top = format!("top: {}px", self.next * 20);
        self.dom.set_attribute(row, name("style"), &top);
        for column in 0..2 {
            let cell = self.dom.create_element(name("span"));
            self.dom.set_attribute(cell, name("role"), "cell");
            let text = self.dom.create_text(&format!("r{}c{column}", self.next));
            self.dom.append_child(cell, text);
            self.dom.append_child(row, cell);
        }
        self.dom.append_child(self.strip, row);
        self.rows.push(row);
        self.next += 1;
    }

    fn scroll(&mut self) {
        let leaving = self.rows.remove(0);
        self.dom.remove(leaving);
        self.push_row();
        let top = format!("top: -{}px", self.next * 3);
        self.dom.set_attribute(self.strip, name("style"), &top);
    }

    fn owner(&self) -> cambium_rootstock::A11yOwner {
        cambium_rootstock::A11yOwner::new(&self.marker)
    }

    fn projection(&self) -> SemanticProjection {
        let view = cambium_rootstock::WindowDom::document(&self.dom);
        let layout = OwnedLayout::new(&view, &[SHEET], 400., 300., &[], &HashMap::new());
        cambium_rootstock::project_semantics(&view, &layout, &self.leaves, &self.producers, None)
    }

    /// A whole publication against `ids`, as `publish_whole` prepares it.
    fn prepare(&self, projection: &SemanticProjection, ids: &publication::NativeIds) -> Prepared {
        let view = cambium_rootstock::WindowDom::document(&self.dom);
        let bindings = projection
            .publication_envelopes(&self.owner(), &view, &self.leaves, &self.producers)
            .unwrap();
        let mut tree = projection.tree.clone();
        let root = tree.tree.as_ref().unwrap().root;
        let (_, node) = tree.nodes.iter_mut().find(|(id, _)| *id == root).unwrap();
        synchronization::scale_root(node, 1.0);
        ids.prepare(tree, bindings).unwrap()
    }
}

/// The exact change set between two whole compositions.
fn diff(before: &SemanticProjection, after: &SemanticProjection) -> SemanticChanges {
    let old: HashMap<_, _> = before.tree.nodes.iter().cloned().collect();
    let new: HashMap<_, _> = after.tree.nodes.iter().cloned().collect();
    let mut changes = SemanticChanges::default();
    for (id, node) in &new {
        match old.get(id) {
            None => changes.added.push(*id),
            Some(previous) if previous != node => changes.changed.push(*id),
            _ => {},
        }
    }
    changes.removed = old
        .keys()
        .filter(|id| !new.contains_key(id))
        .copied()
        .collect();
    let routes = |a: &SemanticProjection, b: &SemanticProjection| {
        a.dom_targets
            .keys()
            .filter(|id| !b.dom_targets.contains_key(id))
            .copied()
            .collect::<Vec<_>>()
    };
    changes.routes = routes(before, after);
    changes.routes.extend(routes(after, before));
    changes
}

use cambium_rootstock::SemanticChanges;

/// A publication in source ids, so freshly allocated native ids compare.
#[derive(Debug, PartialEq)]
struct Published {
    nodes: HashMap<A11yNodeId, accesskit::Node>,
    focus: A11yNodeId,
    envelopes: HashMap<A11yNodeId, cambium_rootstock::SemanticEnvelope>,
    requests: HashMap<A11yNodeId, A11yRequest>,
    identities: HashMap<A11yNodeId, cambium_rootstock::SemanticNodeIdentity>,
}

fn published(
    tree: &TreeUpdate,
    ids: &publication::NativeIds,
    envelopes: &HashMap<A11yNodeId, cambium_rootstock::SemanticEnvelope>,
    requests: &HashMap<A11yNodeId, A11yRequest>,
) -> Published {
    let snapshot = ids.snapshot();
    let source: HashMap<_, _> = snapshot.iter().map(|(s, (n, _))| (*n, *s)).collect();
    let back = |native: A11yNodeId| source.get(&native).copied().ok_or(());
    let nodes = tree
        .nodes
        .iter()
        .map(|(id, node)| {
            let mut node = node.clone();
            crate::remap::try_map_node_ids(&mut node, back).unwrap();
            (source[id], node)
        })
        .collect();
    let envelopes = envelopes
        .iter()
        .map(|(native, envelope)| {
            let mut envelope = envelope.clone();
            envelope.address.local = source[native];
            (source[native], envelope)
        })
        .collect();
    Published {
        nodes,
        focus: source[&tree.focus],
        envelopes,
        requests: requests
            .iter()
            .map(|(native, request)| (source[native], request.clone()))
            .collect(),
        identities: snapshot.into_iter().map(|(s, (_, i))| (s, i)).collect(),
    }
}

#[test]
fn a_patched_publication_equals_a_whole_one() {
    let mut f = Fixture::new();
    let mut host = A11yHost::new(|| {});
    let mut before = f.projection();
    let first = f.prepare(&before, &host.namespace);
    host.native = Some(retained_sync::NativeTree::new(first.tree, 0, 1.0));
    (host.namespace, host.requests, host.envelopes) =
        (first.namespace, first.requests, first.envelopes);
    for sequence in 1..4 {
        f.scroll();
        let after = f.projection();
        let changes = diff(&before, &after);
        let whole = f.prepare(&after, &host.namespace);
        let kept: Vec<_> = host.namespace.sources().collect();
        let view = cambium_rootstock::WindowDom::document(&f.dom);
        let composed = cambium_rootstock::ComposedSemantics {
            projection: &after,
            changes: Some(&changes),
            base: sequence - 1,
            sequence,
            leaves: &f.leaves,
            producers: &f.producers,
        };
        let candidate = host
            .candidate(&f.owner(), &view, composed, &changes, 1.0)
            .expect("a scroll step patches");
        let partial = host.apply(candidate, sequence);
        assert!(
            partial.nodes.len() < whole.tree.nodes.len() / 2,
            "{}",
            partial.nodes.len()
        );
        let native = host.native.as_ref().unwrap();
        assert_eq!(
            published(
                &native.tree,
                &host.namespace,
                &host.envelopes,
                &host.requests
            ),
            published(
                &whole.tree,
                &whole.namespace,
                &whole.envelopes,
                &whole.requests
            ),
        );
        // Kept sources keep their native ids, as a whole publication would.
        for (source, native) in kept {
            if let Some(now) = whole.namespace.get(source) {
                assert_eq!(host.namespace.get(source), Some(now));
                assert_eq!(now, native);
            }
        }
        assert!(
            uxtree::lint::structure(accesskit::TreeId::ROOT, std::slice::from_ref(&native.tree))
                .is_empty()
        );
        before = after;
    }
}

#[test]
fn an_ended_owner_refuses_the_patch() {
    let mut f = Fixture::new();
    let mut host = A11yHost::new(|| {});
    let before = f.projection();
    let first = f.prepare(&before, &host.namespace);
    host.native = Some(retained_sync::NativeTree::new(first.tree, 0, 1.0));
    (host.namespace, host.requests, host.envelopes) =
        (first.namespace, first.requests, first.envelopes);
    f.scroll();
    let after = f.projection();
    let changes = diff(&before, &after);
    let view = cambium_rootstock::WindowDom::document(&f.dom);
    let composed = cambium_rootstock::ComposedSemantics {
        projection: &after,
        changes: Some(&changes),
        base: 0,
        sequence: 1,
        leaves: &f.leaves,
        producers: &f.producers,
    };
    let replacement = Rc::new(Cell::new(true));
    let other = cambium_rootstock::A11yOwner::new(&replacement);
    assert!(
        host.candidate(&other, &view, composed, &changes, 1.0)
            .is_none()
    );
}
