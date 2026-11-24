// Copyright 2026 Mark Alan Boykin
// SPDX-License-Identifier: MPL-2.0

//! The workspace's history graph: one resource per retained traversal event,
//! named by its minted UUID, attributed to its recorder and naming its browsing
//! session (mere:engine-split D195, D197, D198, D202), and one aggregate
//! resource per traversed Surface pair carrying the edge's metrics, which
//! outlive evicted events (D196). A session is shared or deleted by filtering
//! on `browsingSession`.

use kernel::graph::edge_data::Traversal;
use mere::model::{
    PersistedEdge, PersistedEdgeFamily, PersistedTraversalEdgeData, PersistedTraversalMetrics,
};

use super::*;

type Pair = (Uuid, Uuid);

pub(super) fn write(graph: &Graph, out: &mut Writer) -> Result<()> {
    let mut pairs = BTreeMap::<Pair, (Vec<&Traversal>, PersistedTraversalMetrics)>::new();
    for (_, from, to, payload) in graph.surface_relation_buckets() {
        let Some(data) = &payload.traversal else {
            continue;
        };
        let from = graph.get_node(from).ok_or("missing source surface")?.id;
        let to = graph.get_node(to).ok_or("missing target surface")?.id;
        let (events, metrics) = pairs.entry((from, to)).or_default();
        events.extend(&data.traversals);
        // Parallel buckets between one pair aggregate into the pair's metrics.
        metrics.total_navigations += data.metrics.total_navigations;
        metrics.forward_navigations += data.metrics.forward_navigations;
        metrics.backward_navigations += data.metrics.backward_navigations;
        metrics.last_navigated_at = metrics.last_navigated_at.max(data.metrics.last_navigated_at);
    }
    for ((from, to), (events, metrics)) in pairs {
        for event in events {
            let row = surface(event.id);
            out.history_kind(&row, "Traversal");
            out.history_set(&row, vocab("fromSurface"), surface(from));
            out.history_set(&row, vocab("toSurface"), surface(to));
            out.history_set(&row, vocab("timestampMs"), unsigned(event.timestamp_ms));
            out.history_set(&row, vocab("trigger"), enum_term(event.trigger)?);
            let agent = NamedNode::new(event.recorded_by.as_str())
                .map_err(|err| format!("recorder of {row} is not an IRI: {err}"))?;
            out.history_set(&row, named(ATTRIBUTED_TO), agent);
            let session = surface(event.browsing_session);
            out.history_set(&row, vocab("browsingSession"), session);
        }
        let row = metrics_id(&out.history, from, to);
        out.history_kind(&row, "EdgeMetrics");
        out.history_set(&row, vocab("fromSurface"), surface(from));
        out.history_set(&row, vocab("toSurface"), surface(to));
        out.history_set(
            &row,
            vocab("totalNavigations"),
            unsigned(metrics.total_navigations),
        );
        out.history_set(
            &row,
            vocab("forwardNavigations"),
            unsigned(metrics.forward_navigations),
        );
        out.history_set(
            &row,
            vocab("backwardNavigations"),
            unsigned(metrics.backward_navigations),
        );
        if let Some(at) = metrics.last_navigated_at {
            out.history_set(&row, vocab("lastNavigatedMs"), unsigned(at));
        }
    }
    Ok(())
}

/// Restore every event with its id and recorder, then lay each pair's exported
/// metrics over the ones replay counted, so evicted history still counts.
pub(super) fn read(source: &Reader, mut graph: Graph) -> Result<Graph> {
    let mut events = Vec::new();
    for row in source.history_typed("Traversal") {
        let (from, to) = endpoint_ids(source, row)?;
        let recorded_by = as_named(source.one(row, ATTRIBUTED_TO)?)?.as_str().to_string();
        let session = uuid_of(as_named(source.get(row, "browsingSession")?)?, "session")?;
        events.push((
            (from, to),
            Traversal {
                timestamp_ms: source.uint(row, "timestampMs")?,
                trigger: source.enumeration(row, "trigger")?,
                id: uuid_of(row, "event")?,
                recorded_by,
                browsing_session: session,
            },
        ));
    }
    events.sort_by_key(|(pair, event)| (*pair, event.timestamp_ms, event.id));

    let mut metrics = BTreeMap::<Pair, PersistedTraversalMetrics>::new();
    for row in source.history_typed("EdgeMetrics") {
        let (from, to) = endpoint_ids(source, row)?;
        if *row != metrics_id(&source.history, from, to) {
            return Err(format!("noncanonical metrics identity {row}"));
        }
        let last_navigated_at = match source.optional(row, vocab("lastNavigatedMs").as_str())? {
            Some(_) => Some(source.uint(row, "lastNavigatedMs")?),
            None => None,
        };
        metrics.insert(
            (from, to),
            PersistedTraversalMetrics {
                total_navigations: source.uint(row, "totalNavigations")?,
                forward_navigations: source.uint(row, "forwardNavigations")?,
                backward_navigations: source.uint(row, "backwardNavigations")?,
                last_navigated_at,
            },
        );
    }

    for ((from_id, to_id), traversal) in events {
        if !metrics.contains_key(&(from_id, to_id)) {
            return Err(format!("traversal {} has no edge metrics", traversal.id));
        }
        let id = traversal.id;
        let outcome = apply::apply_graph_delta(
            &mut graph,
            apply::GraphDelta::ReplayAppendTraversalByIds {
                from_id,
                to_id,
                traversal,
            },
        );
        if !matches!(outcome, apply::GraphDeltaResult::TraversalAppended(true)) {
            return Err(format!("traversal endpoints refused on urn:uuid:{id}"));
        }
    }

    if metrics.is_empty() {
        return Ok(graph);
    }
    let mut snapshot = graph.to_snapshot();
    for ((from, to), restored) in metrics {
        let (from, to) = (from.to_string(), to.to_string());
        let held = snapshot.edges.iter_mut().find_map(|edge| {
            (edge.from_node_id == from && edge.to_node_id == to)
                .then_some(edge.traversal.as_mut())
                .flatten()
        });
        match held {
            Some(data) => data.metrics = restored,
            None => snapshot.edges.push(PersistedEdge {
                from_node_id: from,
                to_node_id: to,
                families: vec![PersistedEdgeFamily::Traversal],
                semantic: None,
                traversal: Some(PersistedTraversalEdgeData {
                    traversals: Vec::new(),
                    metrics: restored,
                }),
                containment: None,
                arrangement: None,
                imported: None,
                provenance: None,
                assertions: Some(Vec::new()),
            }),
        }
    }
    Graph::try_from_recorded_snapshot(&snapshot).map_err(|err| err.to_string())
}

fn metrics_id(history: &NamedNode, from: Uuid, to: Uuid) -> NamedNode {
    named(format!("{}:metrics:{from}:{to}", history.as_str()))
}

fn endpoint_ids(source: &Reader, row: &NamedNode) -> Result<Pair> {
    let id = |property| uuid_of(as_named(source.get(row, property)?)?, "surface");
    Ok((id("fromSurface")?, id("toSurface")?))
}
