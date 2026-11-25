// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The typed semantic ROOT host's adapter to native address assignment.
use accesskit::{NodeId, TreeId, TreeUpdate};
use cambium_rootstock::{A11yRequest, SemanticEnvelope, SemanticNodeIdentity, SemanticPublication};
use std::collections::HashMap;
use uxtree::lint::{FindingKind, StructureIssue};

use crate::native_addresses::NativeAddresses;

#[derive(Default)]
pub(super) struct NativeIds {
    entries: NativeAddresses<SemanticNodeIdentity>,
}

pub(super) struct Prepared {
    pub tree: TreeUpdate,
    pub namespace: NativeIds,
    pub requests: HashMap<NodeId, A11yRequest>,
    pub envelopes: HashMap<NodeId, SemanticEnvelope>,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Refusal {
    ForeignTree,
    IncompleteTree,
    DuplicateNode(NodeId),
    MissingIdentity(NodeId),
    DanglingReference(NodeId),
    NotATree(NodeId),
    UnreachableNode,
}

impl NativeIds {
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn get(&self, source: NodeId) -> Option<NodeId> {
        self.entries.get((TreeId::ROOT, source))
    }

    /// Every bound source id with its native id.
    pub fn sources(&self) -> impl Iterator<Item = (NodeId, NodeId)> + '_ {
        self.entries
            .iter()
            .map(|((_, source), native)| (source, native))
    }

    pub fn lookup(&self, source: NodeId, identity: &SemanticNodeIdentity) -> (NodeId, bool) {
        self.entries.lookup((TreeId::ROOT, source), identity)
    }

    pub fn insert(&mut self, source: NodeId, native: NodeId, identity: SemanticNodeIdentity) {
        self.entries
            .insert((TreeId::ROOT, source), native, identity);
    }

    pub fn remove(&mut self, source: NodeId) {
        self.entries.remove((TreeId::ROOT, source));
    }

    /// Source id, native id and identity of every entry, for comparison.
    #[cfg(test)]
    pub fn snapshot(&self) -> HashMap<NodeId, (NodeId, SemanticNodeIdentity)> {
        self.entries
            .iter()
            .map(|((_, source), native)| {
                let identity = self.entries.identity((TreeId::ROOT, source)).unwrap();
                (source, (native, identity.clone()))
            })
            .collect()
    }

    /// A candidate only. The installed namespace advances after install/update.
    /// On failure neither current entries nor routing is modified by preparation.
    pub fn prepare(
        &self,
        mut tree: TreeUpdate,
        bindings: SemanticPublication,
    ) -> Result<Prepared, Refusal> {
        if tree.tree_id != TreeId::ROOT || tree.nodes.iter().any(|(_, n)| n.tree_id().is_some()) {
            return Err(Refusal::ForeignTree);
        }
        let root = tree.tree.as_ref().ok_or(Refusal::IncompleteTree)?.root;
        if let Some(finding) = uxtree::lint::structure(tree.tree_id, std::slice::from_ref(&tree))
            .into_iter()
            .next()
        {
            let node = finding.node;
            return Err(match finding.kind {
                FindingKind::Structure(StructureIssue::DuplicateNode) => {
                    Refusal::DuplicateNode(node.unwrap())
                },
                FindingKind::Structure(StructureIssue::DanglingNode) => {
                    Refusal::DanglingReference(node.unwrap())
                },
                FindingKind::Structure(StructureIssue::Cycle | StructureIssue::RepeatedChild) => {
                    Refusal::NotATree(node.unwrap())
                },
                FindingKind::Structure(StructureIssue::UnreachableNode) => Refusal::UnreachableNode,
                _ => Refusal::IncompleteTree,
            });
        }
        let mut entries = NativeAddresses::default();
        for (source, _) in &tree.nodes {
            let identity = bindings
                .envelopes
                .get(source)
                .map(|envelope| envelope.provenance.clone())
                .ok_or(Refusal::MissingIdentity(*source))?;
            entries.bind(&self.entries, (TreeId::ROOT, *source), identity, false);
        }
        let map = |source: NodeId| {
            entries
                .get((TreeId::ROOT, source))
                .ok_or(Refusal::DanglingReference(source))
        };
        for (id, node) in &mut tree.nodes {
            if let Some(key) = bindings.envelopes[id].authored_key.as_deref() {
                node.set_author_id(key);
            }
            *id = map(*id)?;
            crate::remap::try_map_node_ids(node, &map)?;
        }
        tree.tree.as_mut().unwrap().root = map(root)?;
        tree.focus = map(tree.focus)?;
        let requests = bindings
            .requests
            .into_iter()
            .map(|(id, request)| Ok((map(NodeId(id))?, request)))
            .collect::<Result<HashMap<_, _>, Refusal>>()?;
        let mut envelopes = HashMap::new();
        for (source, mut envelope) in bindings.envelopes {
            // Extra identities without routes were ignored before this change.
            // The request remap above still refuses every dangling route.
            if let Some(native) = entries.get((TreeId::ROOT, source)) {
                envelope.address.local = native;
                envelope.stability = uxtree::envelope::Stability::Life;
                envelopes.insert(native, envelope);
            }
        }
        Ok(Prepared {
            tree,
            namespace: NativeIds { entries },
            requests,
            envelopes,
        })
    }
}
