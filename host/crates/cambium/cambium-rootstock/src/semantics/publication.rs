// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Freeze one contributed semantic read into envelopes and typed routes.
use super::SemanticProjection;
use crate::a11y_provenance::{A11yProvenance, NodeTarget, SemanticNodeIdentity};
use crate::{LeafRegistry, ProducerRegistry};
use accesskit::NodeId as A11yNodeId;
use layout_dom_api::{LayoutDom, LocalName, Namespace};
use sceno::SourceRef;
use std::collections::HashMap;
use uxtree::envelope::{Address, Envelope};

/// Metadata for one native publication. Addresses are semantic source IDs until
/// the native publisher remaps them; only its finalized envelopes are published.
pub type SemanticEnvelope = Envelope<crate::A11yOwner, SemanticNodeIdentity, SourceRef>;

/// Complete source envelopes plus operation routes, frozen in one read.
/// The requests are typed input-delivery records, not another metadata model.
pub struct SemanticPublication {
    pub envelopes: HashMap<A11yNodeId, SemanticEnvelope>,
    pub requests: HashMap<u64, crate::A11yRequest>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PublicationIssue {
    OwnerEnded,
    UnknownNode(A11yNodeId),
    DuplicateNode(A11yNodeId),
    RouteMismatch(A11yNodeId),
    ContributorEnded(A11yNodeId),
}
impl SemanticProjection {
    /// Freeze all node identities and action routes from one authored read.
    /// Unknown/duplicate nodes or inconsistent route metadata refuse publication.
    pub fn publication_envelopes(
        &self,
        owner: &crate::A11yOwner,
        dom: &crate::WindowDom<'_>,
        leaves: &LeafRegistry<u64>,
        producers: &ProducerRegistry,
    ) -> Result<SemanticPublication, PublicationIssue> {
        if !owner.is_live() {
            return Err(PublicationIssue::OwnerEnded);
        }
        let mut dom_nodes = HashMap::new();
        let mut members = HashMap::new();
        for member in dom.members() {
            dom_nodes.insert(A11yNodeId(dom.opaque_id(member.node())), member);
            members.insert(member.node(), member);
        }
        let mut envelopes = HashMap::new();
        for (id, _) in &self.tree.nodes {
            let target = self
                .produced_nodes
                .get(id)
                .map(|metadata| metadata.target.clone())
                .or_else(|| {
                    dom_nodes
                        .get(id)
                        .map(|member| NodeTarget::Node(member.node()))
                })
                .ok_or(PublicationIssue::UnknownNode(*id))?;
            let member = match &target {
                NodeTarget::Node(node) => members.get(node),
                NodeTarget::Produced { slot, .. } => self
                    .producer_slots
                    .get(slot)
                    .and_then(|node| members.get(node)),
            }
            .copied()
            .ok_or(PublicationIssue::UnknownNode(*id))?;
            let authored_key = if let NodeTarget::Node(node) = &target {
                dom.attribute(*node, &Namespace::default(), &LocalName::from("data-key"))
                    .or_else(|| dom.attribute(*node, &Namespace::default(), &LocalName::from("id")))
                    .map(str::to_owned)
            } else {
                None
            };
            let identity = SemanticNodeIdentity(
                A11yProvenance::capture_published_member(owner, target, member, leaves, producers)
                    .ok_or(PublicationIssue::RouteMismatch(*id))?,
            );
            if !identity.0.contributors_live() {
                return Err(PublicationIssue::ContributorEnded(*id));
            }
            let mut envelope = Envelope::life(Address::new(owner.clone(), *id), identity);
            envelope.domain_ref = self
                .produced_nodes
                .get(id)
                .and_then(|metadata| metadata.domain_ref.clone());
            if let Some(action) = self.produced_targets.get(id) {
                envelope.action_ids.push(action.id.clone());
            }
            envelope.authored_key = authored_key;
            if envelopes.insert(*id, envelope).is_some() {
                return Err(PublicationIssue::DuplicateNode(*id));
            }
        }
        let mut requests = HashMap::new();
        for (id, mut request) in self
            .dom_targets
            .iter()
            .map(|(id, node)| {
                (
                    *id,
                    crate::A11yRequest::node(crate::A11yAction::Focus, *node),
                )
            })
            .chain(self.produced_targets.iter().map(|(id, target)| {
                (
                    *id,
                    crate::A11yRequest::produced(crate::A11yAction::Focus, target.clone()),
                )
            }))
        {
            let envelope = envelopes
                .get(&id)
                .ok_or(PublicationIssue::UnknownNode(id))?;
            if envelope.provenance.0.target_matches(&request.target) {
                request.provenance = Some(envelope.provenance.0.clone());
            } else {
                return Err(PublicationIssue::RouteMismatch(id));
            }
            requests.insert(id.0, request);
        }
        Ok(SemanticPublication {
            envelopes,
            requests,
        })
    }
}
