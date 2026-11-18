// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! One host observation of a retained document, scoped to its engine revision.

use std::collections::HashMap;

use inker::{
    DocumentA11yAction, DocumentA11yActionData, DocumentA11yActionRequest, DocumentA11yNodeId,
    DocumentA11yProjection,
};
use taproot::{SemanticLookup, SemanticNode, SemanticTarget};
use uxtree::envelope::{Address, Envelope, Stability};

use crate::{PeltController, PeltSessionIdentity};

/// The typed address of a node in one retained session's observation.
pub type ScopedDocumentEnvelope = Envelope<PeltSessionIdentity, PeltSessionIdentity, ()>;

/// A retained semantic read whose node addresses and actions use the same scope.
pub struct DocumentSemanticLookup {
    projection: DocumentA11yProjection,
    entries: Vec<Entry>,
    by_id: HashMap<DocumentA11yNodeId, usize>,
}

struct Entry {
    node_index: usize,
    hidden: bool,
    opaque_address: String,
    envelope: ScopedDocumentEnvelope,
}

impl DocumentSemanticLookup {
    fn new(session: PeltSessionIdentity, projection: DocumentA11yProjection) -> Option<Self> {
        let indices: HashMap<_, _> = projection
            .nodes()
            .iter()
            .enumerate()
            .map(|(index, node)| (node.id, index))
            .collect();
        if indices.len() != projection.nodes().len() {
            return None;
        }
        let mut entries = Vec::with_capacity(indices.len());
        let mut by_id = HashMap::with_capacity(indices.len());
        let mut pending = vec![(projection.root(), false)];
        let revision = projection.revision();
        while let Some((id, inherited_hidden)) = pending.pop() {
            let node_index = *indices.get(&id)?;
            if by_id.insert(id, entries.len()).is_some() {
                return None;
            }
            let node = &projection.nodes()[node_index];
            let hidden = inherited_hidden || node.state.hidden;
            pending.extend(node.children.iter().rev().map(|child| (*child, hidden)));
            let address = Address::new(session, accesskit::NodeId(id.get()));
            entries.push(Entry {
                node_index,
                hidden,
                opaque_address: format!(
                    "pelt-document:{}:{}:{revision}:{}",
                    session.instance_id,
                    session.generation,
                    id.get()
                ),
                envelope: Envelope::scoped(address, revision, session),
            });
        }
        Some(Self {
            projection,
            entries,
            by_id,
        })
    }

    pub fn envelope(&self, target: DocumentA11yNodeId) -> Option<&ScopedDocumentEnvelope> {
        self.by_id
            .get(&target)
            .map(|&index| &self.entries[index].envelope)
    }
}

impl SemanticLookup for DocumentSemanticLookup {
    fn nodes(&self) -> Vec<SemanticNode<'_>> {
        self.entries
            .iter()
            .map(|entry| {
                let node = &self.projection.nodes()[entry.node_index];
                SemanticNode {
                    target: SemanticTarget::Contributed(node.id.get()),
                    role: taproot::document_role_token(node.role),
                    name: node.name.as_deref(),
                    hidden: entry.hidden,
                    authored_key: entry.envelope.authored_key.as_deref(),
                    opaque_address: Some(&entry.opaque_address),
                }
            })
            .collect()
    }
}

impl<F: 'static> PeltController<F> {
    /// Observe the engine projection and qualify it with the current session.
    pub fn document_semantic_lookup(&self) -> Option<DocumentSemanticLookup> {
        DocumentSemanticLookup::new(self.session_identity(), self.accessibility_projection()?)
    }

    /// Deliver a retained semantic action. The controller checks the session
    /// identity and forwards the captured revision for the engine to admit.
    pub fn dispatch_document_semantic_action(
        &mut self,
        observed: &DocumentSemanticLookup,
        target: DocumentA11yNodeId,
        action: DocumentA11yAction,
        data: Option<DocumentA11yActionData>,
    ) -> bool {
        let Some(&index) = observed.by_id.get(&target) else {
            return false;
        };
        let entry = &observed.entries[index];
        let node = &observed.projection.nodes()[entry.node_index];
        if entry.hidden
            || node.state.disabled
            || !node.actions.contains(&action)
            || entry.envelope.provenance != self.session_identity()
        {
            return false;
        }
        let Stability::Scoped { revision } = entry.envelope.stability else {
            return false;
        };
        self.dispatch_accessibility_action(&DocumentA11yActionRequest {
            revision,
            target,
            action,
            data,
        })
    }
}
