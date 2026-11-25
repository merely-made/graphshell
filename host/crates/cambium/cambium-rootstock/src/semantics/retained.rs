// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The composed semantic tree Rootstock last published, patched from Genet's
//! accessibility session (table plan D59, D62, D64). A batch the session or
//! this host cannot prove local composes the whole tree again.
use super::relative::{origin, place, translation};
use super::{SemanticProjection, compose_from};
use crate::document_projection::{HOST_ATTRIBUTES, PaintedFacts, host_projection};
use crate::owned_layout::A11yJournal;
use crate::{LeafRegistry, NodeId, OwnedLayout, ProducerRegistry};
use accesskit::{NodeId as A11yNodeId, Rect, Role};
use document_session_api::{
    DocumentA11yNodeId, DocumentA11yProjection, DocumentA11yRole, DocumentA11ySupport,
};
use genet_render::{A11ySession, A11yStyleQueries, A11yUpdate};
use layout_dom_api::{DomMutation, LayoutDom};
use std::collections::{HashMap, HashSet};

/// What one publication changed, in semantic source ids.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SemanticChanges {
    pub added: Vec<A11yNodeId>,
    /// Kept nodes whose data or placement differ.
    pub changed: Vec<A11yNodeId>,
    pub removed: Vec<A11yNodeId>,
    /// Kept nodes whose DOM action route appeared or went.
    pub routes: Vec<A11yNodeId>,
}

/// The tree Rootstock composed, with what changed since publication `base`.
/// `changes` is `None` when the whole tree was composed again.
#[derive(Clone, Copy)]
pub struct ComposedSemantics<'a> {
    pub projection: &'a SemanticProjection,
    pub changes: Option<&'a SemanticChanges>,
    pub base: u64,
    pub sequence: u64,
    /// The contributors the tree was composed with, for envelope capture.
    pub leaves: &'a LeafRegistry<u64>,
    pub producers: &'a ProducerRegistry,
}

pub(crate) struct RetainedSemantics {
    session: A11ySession<NodeId>,
    support: DocumentA11ySupport,
    projection: SemanticProjection,
    index: HashMap<A11yNodeId, usize>,
    absolute: HashMap<A11yNodeId, Option<Rect>>,
    changes: Option<SemanticChanges>,
    sequence: u64,
}

struct Inputs<'a, D> {
    dom: &'a D,
    layout: &'a OwnedLayout,
    facts: &'a PaintedFacts<'a>,
    focus: Option<u64>,
    leaves: &'a LeafRegistry<u64>,
    producers: &'a ProducerRegistry,
}

/// Build the session's style queries from one walk's facts.
macro_rules! style_queries {
    ($style:ident, $inputs:expr) => {
        let inputs = $inputs;
        let generated = |node| inputs.facts.generated(inputs.dom, inputs.layout, node);
        let rendered = |node| inputs.facts.rendered(inputs.dom, inputs.layout, node);
        let $style = A11yStyleQueries {
            generated: &generated,
            rendered: &rendered,
        };
    };
}

impl RetainedSemantics {
    pub(crate) fn new<D: LayoutDom<NodeId = NodeId>>(
        dom: &D,
        layout: &OwnedLayout,
        focus: Option<u64>,
        leaves: &LeafRegistry<u64>,
        producers: &ProducerRegistry,
    ) -> Self {
        let facts = PaintedFacts::collect(dom, layout, focus);
        let inputs = Inputs {
            dom,
            layout,
            facts: &facts,
            focus,
            leaves,
            producers,
        };
        style_queries!(style, &inputs);
        let session = A11ySession::new(dom, layout.fragments(), facts.focused, 0, None, &style);
        let (support, projection) = compose(&session, &inputs);
        let mut retained = Self {
            session,
            support,
            index: index(&projection),
            projection,
            absolute: HashMap::new(),
            changes: None,
            sequence: 0,
        };
        retained.place_all(&facts);
        retained
    }

    pub(crate) fn composed<'a>(
        &'a self,
        leaves: &'a LeafRegistry<u64>,
        producers: &'a ProducerRegistry,
    ) -> ComposedSemantics<'a> {
        ComposedSemantics {
            projection: &self.projection,
            changes: self.changes.as_ref(),
            base: self.sequence.saturating_sub(1),
            sequence: self.sequence,
            leaves,
            producers,
        }
    }

    /// Bring the tree up to date. `journal` is what the layout recorded since
    /// the last update; `None` or a full layout composes everything again.
    pub(crate) fn update<D: LayoutDom<NodeId = NodeId>>(
        &mut self,
        dom: &D,
        layout: &OwnedLayout,
        focus: Option<u64>,
        leaves: &LeafRegistry<u64>,
        producers: &ProducerRegistry,
        journal: Option<A11yJournal>,
    ) {
        self.sequence = self.sequence.saturating_add(1);
        let facts = PaintedFacts::collect(dom, layout, focus);
        let inputs = Inputs {
            dom,
            layout,
            facts: &facts,
            focus,
            leaves,
            producers,
        };
        let batch = journal.filter(|journal| !journal.full);
        let (restyled, mutations) = batch
            .as_ref()
            .map_or((None, &[][..]), |j| (j.restyled.as_ref(), &j.mutations[..]));
        style_queries!(style, &inputs);
        let update = self.session.update(
            dom,
            layout.fragments(),
            facts.focused,
            0,
            None,
            &style,
            restyled,
            mutations,
        );
        self.changes = match update {
            A11yUpdate::Patched {
                added,
                changed,
                removed,
                ..
            } if self.admits(&inputs, mutations, &added, &changed, &removed) => {
                self.patch(&inputs, &added, &changed, &removed)
            },
            _ => None,
        };
        if self.changes.is_none() {
            let (support, projection) = compose(&self.session, &inputs);
            self.support = support;
            self.index = index(&projection);
            self.projection = projection;
            self.place_all(&facts);
        }
    }

    /// The host's own refusals: a write to an attribute only this host reads,
    /// and any change near a committed textbox, which owns its subtree.
    fn admits<D: LayoutDom<NodeId = NodeId>>(
        &self,
        inputs: &Inputs<'_, D>,
        mutations: &[DomMutation<NodeId>],
        added: &[DocumentA11yNodeId],
        changed: &[DocumentA11yNodeId],
        removed: &[DocumentA11yNodeId],
    ) -> bool {
        let dom = inputs.dom;
        let inserted: HashSet<_> = mutations
            .iter()
            .filter_map(|mutation| match mutation {
                DomMutation::Inserted { node, .. } => Some(*node),
                _ => None,
            })
            .collect();
        let entering = |node: NodeId| {
            let mut at = Some(node);
            while let Some(node) = at {
                if inserted.contains(&node) {
                    return true;
                }
                at = dom.parent(node);
            }
            false
        };
        let host_read = |name: &str| {
            HOST_ATTRIBUTES.contains(&name)
                || super::publication::AUTHORED_KEY_ATTRIBUTES.contains(&name)
        };
        if mutations.iter().any(|mutation| {
            matches!(mutation, DomMutation::AttributeChanged { node, name, .. }
                if dom.is_live(*node) && host_read(&name.local) && !entering(*node))
        }) {
            return false;
        }
        let markers = &inputs.facts.markers;
        if markers.is_empty() {
            return true;
        }
        let near_marker = |id: DocumentA11yNodeId| {
            let mut at = Some(id);
            while let Some(id) = at {
                if markers.contains_key(&id.get()) {
                    return true;
                }
                at = self.session.node(id).and_then(|node| node.parent);
            }
            false
        };
        let focus = self.projection.tree.focus;
        !added.iter().chain(changed).any(|id| near_marker(*id))
            && removed
                .iter()
                .all(|id| self.index.contains_key(&A11yNodeId(id.get())))
            && inputs
                .facts
                .focused
                .map(|node| A11yNodeId(dom.opaque_id(node)))
                .is_none_or(|id| id == focus)
    }

    fn patch<D: LayoutDom<NodeId = NodeId>>(
        &mut self,
        inputs: &Inputs<'_, D>,
        added: &[DocumentA11yNodeId],
        changed: &[DocumentA11yNodeId],
        removed: &[DocumentA11yNodeId],
    ) -> Option<SemanticChanges> {
        let mut changes = SemanticChanges::default();
        for id in removed.iter().map(|id| A11yNodeId(id.get())) {
            let Some(i) = self.index.remove(&id) else {
                continue;
            };
            let nodes = &mut self.projection.tree.nodes;
            nodes.swap_remove(i);
            if let Some((moved, _)) = nodes.get(i) {
                self.index.insert(*moved, i);
            }
            self.absolute.remove(&id);
            changes.removed.push(id);
        }
        let touched = self.lower(inputs.facts, added.iter().chain(changed));
        self.place_all(inputs.facts);
        let added: HashSet<_> = added.iter().map(|id| A11yNodeId(id.get())).collect();
        // D74: one pass over every node. A kept node changes only when its
        // placement relative to its parent moved.
        let mut pending = vec![(A11yNodeId(self.session.root().get()), (0.0, 0.0))];
        while let Some((id, parent)) = pending.pop() {
            let Some(&i) = self.index.get(&id) else {
                continue;
            };
            let absolute = self.absolute.get(&id).copied().flatten();
            let node = &mut self.projection.tree.nodes[i].1;
            if touched.contains(&id) || placement_differs(node, absolute, parent) {
                place(node, absolute, parent);
                if added.contains(&id) {
                    changes.added.push(id);
                } else {
                    changes.changed.push(id);
                }
            }
            let here = origin(absolute, parent);
            pending.extend(node.children().iter().map(|child| (*child, here)));
        }
        if changes.added.len() != added.len() {
            // A new node the walk from the root never reached.
            return None;
        }
        self.refresh_targets(inputs, &mut changes);
        Some(changes)
    }

    /// Lower the session's new and changed nodes with the host's own data.
    fn lower<'i>(
        &mut self,
        facts: &PaintedFacts<'_>,
        ids: impl Iterator<Item = &'i DocumentA11yNodeId>,
    ) -> HashSet<A11yNodeId> {
        let fresh: Vec<_> = ids
            .filter_map(|id| self.session.node(*id).cloned())
            .map(|mut node| {
                facts.host_node(&mut node);
                node
            })
            .collect();
        let multiline: HashSet<_> = fresh
            .iter()
            .filter(|node| node.role == DocumentA11yRole::TextField && node.state.multiline)
            .map(|node| A11yNodeId(node.id.get()))
            .collect();
        let lowered = genet_render::accesskit_tree_from_projection(DocumentA11yProjection::new(
            0,
            self.support.clone(),
            self.session.root(),
            fresh,
        ));
        let mut touched = HashSet::new();
        for (id, mut node) in lowered.nodes {
            if multiline.contains(&id) {
                node.set_role(Role::MultilineTextInput);
            }
            touched.insert(id);
            let nodes = &mut self.projection.tree.nodes;
            match self.index.get(&id) {
                Some(&i) => nodes[i].1 = node,
                None => {
                    self.index.insert(id, nodes.len());
                    nodes.push((id, node));
                },
            }
        }
        touched
    }

    /// Every node's absolute box this publication: the painted border box
    /// where Genet places the node, else Genet's own bounds.
    fn place_all(&mut self, facts: &PaintedFacts<'_>) {
        self.absolute.clear();
        for (id, _) in &self.projection.tree.nodes {
            let session = self.session.node(DocumentA11yNodeId::new(id.0));
            let absolute = session
                .and_then(|node| node.bounds)
                .map(|bounds| rect(facts.painted.get(&id.0).copied().unwrap_or(bounds)));
            self.absolute.insert(*id, absolute);
        }
    }

    /// Recompute focus and DOM action routes over the patched tree, as a
    /// full composition does, and report routes that appeared or went.
    fn refresh_targets<D: LayoutDom<NodeId = NodeId>>(
        &mut self,
        inputs: &Inputs<'_, D>,
        changes: &mut SemanticChanges,
    ) {
        let projection = &mut self.projection;
        let root = A11yNodeId(self.session.root().get());
        let mut visible = HashSet::new();
        let mut pending = vec![root];
        while let Some(id) = pending.pop() {
            if let Some(&i) = self.index.get(&id) {
                let node = &projection.tree.nodes[i].1;
                if !node.is_hidden() && visible.insert(id) {
                    pending.extend(node.children());
                }
            }
        }
        let mut targets = HashMap::new();
        for walked in &inputs.facts.order {
            let id = A11yNodeId(walked.opaque);
            if visible.contains(&id) {
                targets.insert(id, walked.node);
            }
        }
        let gone = projection
            .dom_targets
            .keys()
            .filter(|id| !targets.contains_key(id));
        let new = targets
            .keys()
            .filter(|id| !projection.dom_targets.contains_key(id));
        changes.routes.extend(gone.chain(new).copied());
        projection.dom_targets = targets;
        let focused = inputs
            .facts
            .focused
            .map(|node| DocumentA11yNodeId::new(inputs.dom.opaque_id(node)))
            .filter(|id| {
                self.session
                    .node(*id)
                    .is_some_and(|node| node.state.focused)
            })
            .map_or(root, |id| A11yNodeId(id.get()));
        projection.tree.focus = if visible.contains(&focused) {
            focused
        } else {
            root
        };
    }
}

/// The whole tree from the session's projection, as `project_semantics`.
fn compose<D: LayoutDom<NodeId = NodeId>>(
    session: &A11ySession<NodeId>,
    inputs: &Inputs<'_, D>,
) -> (DocumentA11ySupport, SemanticProjection) {
    let projection = session.projection();
    let document = host_projection(&projection, inputs.facts, inputs.focus);
    let composed = compose_from(
        document,
        inputs.facts,
        inputs.leaves,
        inputs.producers,
        None,
    );
    (projection.support().clone(), composed)
}

fn rect(bounds: document_session_api::DocumentA11yBounds) -> Rect {
    Rect::new(
        f64::from(bounds.x),
        f64::from(bounds.y),
        f64::from(bounds.x + bounds.width),
        f64::from(bounds.y + bounds.height),
    )
}

fn placement_differs(node: &accesskit::Node, absolute: Option<Rect>, parent: (f64, f64)) -> bool {
    let Some(want) = absolute else {
        return node.bounds().is_some() || node.transform().is_some();
    };
    let Some(bounds) = node.bounds() else {
        return true;
    };
    translation(node) != (want.x0 - parent.0, want.y0 - parent.1)
        || bounds != Rect::new(0.0, 0.0, want.width(), want.height())
}

fn index(projection: &SemanticProjection) -> HashMap<A11yNodeId, usize> {
    projection
        .tree
        .nodes
        .iter()
        .enumerate()
        .map(|(i, (id, _))| (*id, i))
        .collect()
}

#[cfg(test)]
#[path = "retained_tests.rs"]
mod tests;
