// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The personal graph's fold into incipit-graph (engine split D259). The
//! protocol, its admission and its bookkeeping are pandect's and re-exported;
//! [`IncipitFold`] is a copy of pandect's `KernelFold` arms (Mere 36a13570b,
//! `personal_sync.rs`) and the two statement writers it calls (from
//! `personal_sync/assertions.rs`), over this crate's graph. A replica's
//! projection into this graph is `replica.projection_with(&IncipitFold)`;
//! [`materialize`] is pandect's `materialize` into this graph.

pub use pandect::personal_sync::*;

use std::sync::Arc;

use kernel::geometry::PortablePoint;
use kernel::graph::apply::{GraphDelta, add_node, apply_graph_delta};
use kernel::graph::Graph;
use mere::model::{AssertionBasis, GraphScope};
use mere::model::{EdgeAssertion, EdgeFamily, RelationKind, RelationSelector, SemanticStatement};
use muniment::Backend;
use pandect::SAVED_SCENE_FACET_V3;
use pandect::access::{ACCESS_HISTORY_FACET, AccessHistory};
use stickleback::{DataKeyring, MunimentStore};
use uuid::Uuid;

/// Project one retained operation set into incipit-graph, as pandect's
/// `materialize` did into the kernel's.
pub async fn materialize<B: Backend + Clone + Send + Sync + 'static>(
    store: &MunimentStore<B, PersonalGraphExt>,
    graph_id: [u8; 32],
    roster: &SyncRoster,
    keyring: Option<&Arc<DataKeyring>>,
    selection: &SyncSelection,
) -> Result<SyncProjection<Graph>, PersonalGraphError> {
    materialize_with(store, graph_id, roster, keyring, selection, &IncipitFold).await
}

/// incipit-graph's fold.
#[derive(Clone, Copy, Debug, Default)]
pub struct IncipitFold;

impl PersonalFold<Graph> for IncipitFold {
    fn apply(
        &self,
        graph: &mut Graph,
        asserter_iri: &str,
        fallback_id: &str,
        event: &PersonalGraphEvent,
    ) -> Result<(), PersonalGraphError> {
        match event {
            PersonalGraphEvent::AddNode { id, address, title } => {
                if graph.get_node_key_by_id(*id).is_none() {
                    let key = add_node(
                        graph,
                        Some(*id),
                        address.clone(),
                        PortablePoint::new(0.0, 0.0),
                    );
                    if !title.is_empty() {
                        apply_graph_delta(
                            graph,
                            GraphDelta::SetNodeTitle {
                                key,
                                title: title.clone(),
                            },
                        );
                    }
                }
            },
            PersonalGraphEvent::RemoveNode { id } => {
                apply_graph_delta(graph, GraphDelta::ReplayRemoveNodeById { node_id: *id });
            },
            PersonalGraphEvent::SetTitle { node, title } => {
                apply_graph_delta(
                    graph,
                    GraphDelta::ReplaySetNodeTitleById {
                        node_id: *node,
                        title: title.clone(),
                    },
                );
            },
            PersonalGraphEvent::AddTag { node, tag } => {
                apply_graph_delta(
                    graph,
                    GraphDelta::ReplayInsertNodeTagById {
                        node_id: *node,
                        tag: tag.clone(),
                    },
                );
            },
            PersonalGraphEvent::RemoveTag { node, tag } => {
                apply_graph_delta(
                    graph,
                    GraphDelta::ReplayRemoveNodeTagById {
                        node_id: *node,
                        tag: tag.clone(),
                    },
                );
            },
            PersonalGraphEvent::AssertRelation {
                from,
                to,
                assertion,
                assertion_version,
                statement_id,
                asserted_at_ms,
                basis,
            } => {
                if matches!(assertion_version, Some(1))
                    || matches!(assertion, EdgeAssertion::Semantic { .. })
                {
                    assert_statement(
                        graph,
                        *from,
                        *to,
                        assertion,
                        asserter_iri,
                        statement_id.as_deref().unwrap_or(fallback_id),
                        *asserted_at_ms,
                        basis.clone(),
                    )?;
                } else {
                    apply_graph_delta(
                        graph,
                        GraphDelta::ReplayAssertRelationByIds {
                            from_id: *from,
                            to_id: *to,
                            assertion: assertion.clone(),
                            asserter_iri: asserter_iri.to_owned(),
                        },
                    );
                }
            },
            PersonalGraphEvent::RetractRelation { from, to, selector } => {
                if retract_legacy(graph, *from, *to, *selector, asserter_iri) {
                    return Ok(());
                }
                apply_graph_delta(
                    graph,
                    GraphDelta::ReplayRetractRelationsByIds {
                        from_id: *from,
                        to_id: *to,
                        selector: *selector,
                    },
                );
            },
            PersonalGraphEvent::RetractAssertion {
                from: _,
                to: _,
                statement_id,
                withdrawn_at_ms,
            } => {
                graph.apply_imported_semantic_withdrawal(
                    statement_id,
                    *withdrawn_at_ms,
                    Some(asserter_iri.to_owned()),
                );
            },
            PersonalGraphEvent::SetFacet { node, facet, value } => {
                apply_graph_delta(
                    graph,
                    GraphDelta::ReplaySetNodeFacetById {
                        node_id: *node,
                        facet: facet.clone(),
                        value: value.clone(),
                    },
                );
            },
            PersonalGraphEvent::RemoveFacet { node, facet } => {
                apply_graph_delta(
                    graph,
                    GraphDelta::ReplayRemoveNodeFacetById {
                        node_id: *node,
                        facet: facet.clone(),
                    },
                );
            },
            PersonalGraphEvent::SaveScene { node, scene } => {
                apply_graph_delta(
                    graph,
                    GraphDelta::ReplaySetNodeFacetById {
                        node_id: *node,
                        facet: SAVED_SCENE_FACET_V3.to_string(),
                        value: serde_json::to_value(scene).expect("saved scene always serializes"),
                    },
                );
            },
            _ => {},
        }
        Ok(())
    }

    fn set_access_history(&self, graph: &mut Graph, node: Uuid, history: AccessHistory) {
        apply_graph_delta(
            graph,
            GraphDelta::ReplaySetNodeFacetById {
                node_id: node,
                facet: ACCESS_HISTORY_FACET.to_string(),
                value: serde_json::to_value(history).expect("access history always serializes"),
            },
        );
    }
}

fn relation_kind(assertion: &EdgeAssertion) -> RelationKind {
    match assertion {
        EdgeAssertion::Semantic { sub_kind, .. } => RelationKind::Semantic(*sub_kind),
        EdgeAssertion::Containment { sub_kind } => RelationKind::Containment(*sub_kind),
        EdgeAssertion::Arrangement { sub_kind } => RelationKind::Arrangement(*sub_kind),
        EdgeAssertion::Imported { sub_kind } => RelationKind::Imported(*sub_kind),
        EdgeAssertion::Provenance { sub_kind } => RelationKind::Provenance(*sub_kind),
    }
}

fn assertion_label(assertion: &EdgeAssertion) -> Option<String> {
    match assertion {
        EdgeAssertion::Semantic { label, .. } => label.clone(),
        _ => None,
    }
}

fn assert_statement(
    graph: &mut Graph,
    from: Uuid,
    to: Uuid,
    assertion: &EdgeAssertion,
    asserter: &str,
    id: &str,
    at_ms: Option<u64>,
    basis: Option<AssertionBasis>,
) -> Result<(), PersonalGraphError> {
    let Some((from, to)) = graph
        .get_node_key_by_id(from)
        .zip(graph.get_node_key_by_id(to))
    else {
        return Err(PersonalGraphError::Excluded(
            "exact relation assertion references a missing endpoint".into(),
        ));
    };
    let label = assertion_label(assertion);
    let basis = basis.unwrap_or_else(|| AssertionBasis {
        note: label.clone(),
        ..Default::default()
    });
    let kind = relation_kind(assertion);
    let predicate = kind
        .predicate_iri()
        .expect("assertion relation kinds have canonical predicates")
        .to_owned();
    let recognized_sub_kind = match kind {
        RelationKind::Semantic(sub_kind) => Some(sub_kind),
        _ => None,
    };
    let label = basis.note.clone().or(label);
    let statement = SemanticStatement {
        statement_id: id.to_owned(),
        predicate,
        recognized_sub_kind,
        label,
        basis,
        graph_scope: GraphScope::Default,
        provenance_iri: Some(asserter.to_owned()),
        asserted_at_ms: at_ms,
        first_asserted_ms: at_ms,
        withdrawn: None,
    };
    // Personal event validation and IdentityIndex establish the complete
    // exact-ID contract before this deterministic fold reaches the writer.
    graph
        .try_assert_persisted_relation_statement(from, to, statement)
        .map_err(|error| PersonalGraphError::Excluded(error.to_string()))?;
    Ok(())
}

fn retract_legacy(
    graph: &mut Graph,
    from: Uuid,
    to: Uuid,
    selector: RelationSelector,
    asserter: &str,
) -> bool {
    if selector == RelationSelector::Family(EdgeFamily::Traversal) {
        return false;
    }
    if let Some((from, to)) = graph
        .get_node_key_by_id(from)
        .zip(graph.get_node_key_by_id(to))
    {
        let ids = graph
            .projected_relations_between(from, to)
            .flat_map(|(_, edge)| {
                edge.held_assertions()
                    .filter(|s| {
                        s.provenance_iri.as_deref() == Some(asserter)
                            && mere::model::selector_matches_kind(selector, s.relation_kind())
                    })
                    .map(|s| s.statement_id.clone())
            })
            .collect::<Vec<_>>();
        for id in ids {
            graph.apply_imported_semantic_withdrawal_for_pair(
                from,
                to,
                &id,
                None,
                Some(asserter.to_owned()),
            );
        }
    }
    true
}
