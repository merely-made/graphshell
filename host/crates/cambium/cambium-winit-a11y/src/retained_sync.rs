// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Partial publication from Rootstock's change set (table plan D64): the
//! native id map, envelopes and routes are patched for the changed nodes and
//! AccessKit is handed only those. Anything unproven publishes the whole tree.

use std::collections::{HashMap, HashSet};

use accesskit::{NodeId, TreeId, TreeUpdate};
use cambium_rootstock::{
    A11yOwner, ComposedSemantics, SemanticChanges, SemanticEnvelope, SemanticNodeIdentity,
    WindowDom,
};

use super::synchronization::{Publish, scale_root};
use super::*;

/// The native tree as last submitted, kept whole so every publication can be
/// linted entire (D73) and a partial one patched into it.
pub(super) struct NativeTree {
    pub tree: TreeUpdate,
    index: HashMap<NodeId, usize>,
    /// The Rootstock composition this tree publishes.
    pub sequence: u64,
    scale: f64,
}

impl NativeTree {
    pub fn new(tree: TreeUpdate, sequence: u64, scale: f64) -> Self {
        let index = tree
            .nodes
            .iter()
            .enumerate()
            .map(|(i, (id, _))| (*id, i))
            .collect();
        Self {
            tree,
            index,
            sequence,
            scale,
        }
    }

    fn remove(&mut self, id: NodeId) {
        if let Some(i) = self.index.remove(&id) {
            self.tree.nodes.swap_remove(i);
            if let Some((moved, _)) = self.tree.nodes.get(i) {
                self.index.insert(*moved, i);
            }
        }
    }

    fn put(&mut self, id: NodeId, node: accesskit::Node) {
        match self.index.get(&id) {
            Some(&i) => self.tree.nodes[i].1 = node,
            None => {
                self.index.insert(id, self.tree.nodes.len());
                self.tree.nodes.push((id, node));
            },
        }
    }
}

/// A partial publication, computed without touching the published maps.
pub(super) struct Candidate {
    /// Source nodes leaving, with their native ids.
    removed: Vec<(NodeId, NodeId)>,
    /// Changed or new nodes: source id, native id, identity and remapped node.
    nodes: Vec<(NodeId, NodeId, SemanticNodeIdentity, accesskit::Node)>,
    envelopes: Vec<(NodeId, SemanticEnvelope)>,
    /// Native ids whose route is replaced (`Some`) or withdrawn (`None`).
    routes: Vec<(NodeId, Option<A11yRequest>)>,
    focus: NodeId,
}

impl A11yHost {
    pub(super) fn sync_composed_inner(
        &mut self,
        owner: &A11yOwner,
        dom: &WindowDom<'_>,
        composed: ComposedSemantics<'_>,
        layout_scale: f64,
    ) -> A11ySync {
        let (window, requests) = match self.sync_prelude(owner) {
            Ok(ready) => ready,
            Err(sync) => return sync,
        };
        let projection = composed.projection;
        let patchable = self.installed
            && self.native.as_ref().is_some_and(|native| {
                native.sequence == composed.base && native.scale == layout_scale
            });
        if let Some(changes) = composed.changes.filter(|_| patchable)
            && let Some(candidate) = self.candidate(owner, dom, composed, changes, layout_scale)
        {
            return self.publish_partial(&window, requests, candidate, composed.sequence);
        }
        let publish = Publish {
            window: &window,
            requests,
            contribution_issues: projection.producer_issues.len(),
            layout_scale,
            retain: Some(composed.sequence),
        };
        let bindings =
            projection.publication_envelopes(owner, dom, composed.leaves, composed.producers);
        self.publish_whole(publish, projection.tree.clone(), bindings)
    }

    /// The partial publication for `changes`, or `None` when it cannot be
    /// proven equal to a whole one: a kept envelope that no longer stands
    /// (D77), a node whose identity changed, or a reference outside the map.
    pub(super) fn candidate(
        &self,
        owner: &A11yOwner,
        dom: &WindowDom<'_>,
        composed: ComposedSemantics<'_>,
        changes: &SemanticChanges,
        layout_scale: f64,
    ) -> Option<Candidate> {
        let projection = composed.projection;
        let removed: HashSet<NodeId> = changes.removed.iter().copied().collect();
        let fresh: HashSet<NodeId> = changes
            .added
            .iter()
            .chain(&changes.changed)
            .chain(&changes.routes)
            .filter(|id| !removed.contains(id))
            .copied()
            .collect();
        // D77: every kept envelope re-checks its owner and mount.
        if !self.namespace.sources().all(|(source, native)| {
            fresh.contains(&source)
                || removed.contains(&source)
                || self
                    .envelopes
                    .get(&native)
                    .is_some_and(|envelope| envelope.provenance.is_live_in(owner, dom))
        }) {
            return None;
        }
        let bindings = projection
            .publication_envelopes_for(owner, dom, composed.leaves, composed.producers, &fresh)
            .ok()?;
        let mut natives = HashMap::new();
        for id in &fresh {
            let identity = bindings.envelopes.get(id)?.provenance.clone();
            let (native, retained) = self.namespace.lookup(*id, &identity);
            if !retained && !changes.added.contains(id) {
                return None;
            }
            natives.insert(*id, native);
        }
        let map = |source: NodeId| match natives.get(&source) {
            Some(native) => Ok(*native),
            None if removed.contains(&source) => Err(()),
            None => self.namespace.get(source).ok_or(()),
        };
        let index: HashMap<_, _> = changes
            .added
            .iter()
            .chain(&changes.changed)
            .map(|id| (*id, ()))
            .collect();
        let root = projection.tree.tree.as_ref()?.root;
        let mut nodes = Vec::new();
        for (id, node) in projection
            .tree
            .nodes
            .iter()
            .filter(|(id, _)| index.contains_key(id))
        {
            let mut node = node.clone();
            if let Some(key) = bindings.envelopes[id].authored_key.as_deref() {
                node.set_author_id(key);
            }
            crate::remap::try_map_node_ids(&mut node, map).ok()?;
            if *id == root {
                scale_root(&mut node, layout_scale);
            }
            let identity = bindings.envelopes[id].provenance.clone();
            nodes.push((*id, natives[id], identity, node));
        }
        let mut envelopes = Vec::new();
        for (source, mut envelope) in bindings.envelopes {
            let native = natives[&source];
            envelope.address.local = native;
            envelope.stability = uxtree::envelope::Stability::Life;
            envelopes.push((native, envelope));
        }
        let mut routes: Vec<_> = fresh
            .iter()
            .map(|id| (natives[id], bindings.requests.get(&id.0).cloned()))
            .collect();
        let mut gone = Vec::new();
        for id in &changes.removed {
            let native = self.namespace.get(*id)?;
            routes.push((native, None));
            gone.push((*id, native));
        }
        Some(Candidate {
            removed: gone,
            nodes,
            envelopes,
            routes,
            focus: map(projection.tree.focus).ok()?,
        })
    }

    fn publish_partial(
        &mut self,
        window: &std::sync::Arc<dyn Window>,
        requests: Vec<A11yRequest>,
        candidate: Candidate,
        sequence: u64,
    ) -> A11ySync {
        let refused = |host: &mut Self, reason, issue: String| {
            host.clear_publication();
            eprintln!("[cambium-winit] accessibility publication refused: {issue}");
            window.set_visible(true);
            A11ySync {
                requests: Vec::new(),
                publication: A11yPublication::refused(host.adapter_state(), reason),
            }
        };
        let partial = self.apply(candidate, sequence);
        let native = self
            .native
            .as_ref()
            .expect("a partial publication patches a kept tree");
        // D73: lint the whole retained tree every publication.
        let finding = uxtree::lint::structure(TreeId::ROOT, std::slice::from_ref(&native.tree))
            .into_iter()
            .next();
        if let Some(finding) = finding {
            let mut sync = refused(
                self,
                A11yPublicationRefusal::InvalidTree,
                format!("{finding:?}"),
            );
            sync.requests = requests;
            return sync;
        }
        let nodes = partial.nodes.len();
        let submitted =
            crate::forest_publisher::submit_partial(&mut self.bridge, &native.tree, partial);
        if let Err(error) = submitted {
            let mut sync = refused(self, A11yPublicationRefusal::InvalidTree, error);
            sync.requests = requests;
            return sync;
        }
        A11ySync {
            requests,
            publication: A11yPublication {
                adapter: self.adapter_state(),
                outcome: A11yPublicationOutcome::Submitted {
                    nodes,
                    contribution_issues: 0,
                },
            },
        }
    }

    /// Patch the published maps and the retained tree; returns the partial
    /// update holding only the changed nodes.
    pub(super) fn apply(&mut self, candidate: Candidate, sequence: u64) -> TreeUpdate {
        let native = self
            .native
            .as_mut()
            .expect("a partial publication patches a kept tree");
        for (source, id) in candidate.removed {
            self.namespace.remove(source);
            self.envelopes.remove(&id);
            native.remove(id);
        }
        let mut partial = Vec::with_capacity(candidate.nodes.len());
        for (source, id, identity, node) in candidate.nodes {
            self.namespace.insert(source, id, identity);
            native.put(id, node.clone());
            partial.push((id, node));
        }
        for (id, envelope) in candidate.envelopes {
            self.envelopes.insert(id, envelope);
        }
        for (id, route) in candidate.routes {
            match route {
                Some(request) => self.requests.insert(id, request),
                None => self.requests.remove(&id),
            };
        }
        native.tree.focus = candidate.focus;
        native.sequence = sequence;
        TreeUpdate {
            nodes: partial,
            tree: None,
            tree_id: TreeId::ROOT,
            focus: candidate.focus,
        }
    }
}

#[cfg(test)]
#[path = "retained_sync_tests.rs"]
mod tests;
