// Copyright 2026 Mark Alan Boykin
// SPDX-License-Identifier: MPL-2.0

//! Trail round trips under mere:engine-split D195 to D199: minted event ids,
//! per-event attribution, browsing sessions (D202), edge metrics and a history
//! graph apart from the trail.

use std::collections::{BTreeMap, BTreeSet};

use kernel::geometry::PortablePoint;
use kernel::graph::{
    ArrangementSubKind, Author, EdgeAssertion, Graph, NavigationTrigger, NodeKey, apply,
};
use uuid::Uuid;

const ALICE: &str = "urn:test:alice";
const ORIGIN: Uuid = Uuid::from_u128(0x0a11ce);

type Event = (Uuid, Uuid, Uuid, u64, String, String, Uuid);
type Metrics = BTreeMap<(Uuid, Uuid), (u64, u64, u64, Option<u64>)>;

fn node(graph: &mut Graph, n: u128, url: &str) -> NodeKey {
    apply::add_node(
        graph,
        Some(Uuid::from_u128(n)),
        url.into(),
        PortablePoint::new(0.0, 0.0),
    )
}

fn traverse(graph: &mut Graph, from: NodeKey, to: NodeKey, at: u64, trigger: NavigationTrigger) {
    graph.write_as(Author::person(ALICE), |graph| {
        let outcome = apply::apply_graph_delta(
            graph,
            apply::GraphDelta::AppendTraversal {
                from,
                to,
                trigger,
                timestamp_ms: Some(at),
            },
        );
        assert!(matches!(
            outcome,
            apply::GraphDeltaResult::TraversalAppended(true)
        ));
    });
}

fn events(graph: &Graph) -> BTreeSet<Event> {
    let mut out = BTreeSet::new();
    for (_, from, to, payload) in graph.surface_relation_buckets() {
        let (from, to) = (graph.get_node(from).unwrap().id, graph.get_node(to).unwrap().id);
        for event in payload.traversals() {
            out.insert((
                from,
                to,
                event.id,
                event.timestamp_ms,
                format!("{:?}", event.trigger),
                event.recorded_by.clone(),
                event.browsing_session,
            ));
        }
    }
    out
}

fn metrics(graph: &Graph) -> Metrics {
    let mut out = Metrics::new();
    for (_, from, to, payload) in graph.surface_relation_buckets() {
        if let Some(data) = payload.traversal_data() {
            let (from, to) = (graph.get_node(from).unwrap().id, graph.get_node(to).unwrap().id);
            let m = &data.metrics;
            out.insert(
                (from, to),
                (
                    m.total_navigations,
                    m.forward_navigations,
                    m.backward_navigations,
                    m.last_navigated_at,
                ),
            );
        }
    }
    out
}

fn surfaces(graph: &Graph) -> BTreeMap<Uuid, String> {
    graph
        .to_snapshot()
        .nodes
        .iter()
        .map(|node| {
            (
                Uuid::parse_str(&node.node_id).unwrap(),
                node.address.as_url_str().to_string(),
            )
        })
        .collect()
}

fn memberships(graph: &Graph) -> BTreeSet<(Uuid, Uuid)> {
    graph
        .arrangement_edges()
        .filter(|relation| relation.sub_kind == ArrangementSubKind::FrameMember)
        .map(|relation| {
            (
                graph.get_node(relation.from).unwrap().id,
                graph.get_node(relation.to).unwrap().id,
            )
        })
        .collect()
}

fn reopen(graph: &Graph) -> Graph {
    let text = graphshell_rdf_trail::export(graph, ORIGIN).expect("export");
    let reopened = graphshell_rdf_trail::reopen(&text).expect("reopen");
    assert_eq!(reopened.origin, ORIGIN);
    reopened.graph
}

/// P7's acceptance: one address visited twice through two distinct Surfaces.
fn two_surfaces_one_address() -> Graph {
    let mut graph = Graph::new();
    let frame = node(&mut graph, 1, "https://start.test/");
    let first = node(&mut graph, 2, "https://same.test/page");
    let second = node(&mut graph, 3, "https://same.test/page");
    apply::assert_relation(
        &mut graph,
        frame,
        first,
        EdgeAssertion::Arrangement {
            sub_kind: ArrangementSubKind::FrameMember,
        },
    )
    .expect("frame membership");
    traverse(&mut graph, frame, first, 1_000, NavigationTrigger::LinkClick);
    traverse(&mut graph, frame, second, 2_000, NavigationTrigger::LinkClick);
    graph
}

#[test]
fn one_address_through_two_surfaces_returns_both_events() {
    let graph = two_surfaces_one_address();
    let original = events(&graph);
    assert_eq!(original.len(), 2);

    let reopened = reopen(&graph);
    assert_eq!(surfaces(&reopened), surfaces(&graph));
    assert_eq!(events(&reopened), original);
    assert_eq!(metrics(&reopened), metrics(&graph));
    assert_eq!(memberships(&reopened), memberships(&graph));
    for event in &original {
        assert_eq!(event.5, Author::person(ALICE).asserter_iri());
    }
}

#[test]
fn identical_clicks_on_one_edge_stay_two_events() {
    let mut graph = Graph::new();
    let a = node(&mut graph, 1, "https://a.test/");
    let b = node(&mut graph, 2, "https://b.test/");
    traverse(&mut graph, a, b, 5, NavigationTrigger::Back);
    traverse(&mut graph, a, b, 5, NavigationTrigger::Back);
    let reopened = reopen(&graph);
    assert_eq!(events(&reopened).len(), 2);
    assert_eq!(events(&reopened), events(&graph));
}

#[test]
fn metrics_survive_evicted_traversals() {
    let mut graph = Graph::new();
    let a = node(&mut graph, 1, "https://a.test/");
    let b = node(&mut graph, 2, "https://b.test/");
    let c = node(&mut graph, 3, "https://c.test/");
    for at in [10, 20, 30] {
        traverse(&mut graph, a, b, at, NavigationTrigger::LinkClick);
    }
    traverse(&mut graph, a, c, 40, NavigationTrigger::Forward);
    traverse(&mut graph, a, c, 50, NavigationTrigger::Back);

    // Evict: a→b keeps its latest event, a→c keeps none; the metrics stay.
    let mut snapshot = graph.to_snapshot();
    for edge in &mut snapshot.edges {
        if let Some(data) = edge.traversal.as_mut() {
            let keep = usize::from(edge.to_node_id == Uuid::from_u128(2).to_string());
            let drop = data.traversals.len() - keep;
            data.traversals.drain(..drop);
        }
    }
    let evicted = Graph::try_from_recorded_snapshot(&snapshot).unwrap();
    assert_eq!(events(&evicted).len(), 1);
    let kept = metrics(&evicted);
    assert_eq!(kept[&(Uuid::from_u128(1), Uuid::from_u128(2))].0, 3);
    assert_eq!(kept[&(Uuid::from_u128(1), Uuid::from_u128(3))], (2, 1, 1, Some(50)));

    let reopened = reopen(&evicted);
    assert_eq!(events(&reopened), events(&evicted));
    assert_eq!(metrics(&reopened), kept);
    // Control: replaying the one retained event alone would count one, not three.
    assert_ne!(
        metrics(&reopened)[&(Uuid::from_u128(1), Uuid::from_u128(2))].0,
        events(&reopened).len() as u64
    );
}

#[test]
fn a_dropped_event_is_caught() {
    let graph = two_surfaces_one_address();
    let text = graphshell_rdf_trail::export(&graph, ORIGIN).unwrap();
    let victim = format!("urn:uuid:{}", events(&graph).first().unwrap().2);
    let quads: Vec<oxrdf::Quad> = oxttl::TriGParser::new()
        .for_slice(text.as_bytes())
        .collect::<Result<_, _>>()
        .unwrap();
    let mut serializer = oxttl::TriGSerializer::new().for_writer(Vec::new());
    let mut dropped = 0;
    for quad in &quads {
        if matches!(&quad.subject, oxrdf::NamedOrBlankNode::NamedNode(s) if s.as_str() == victim) {
            dropped += 1;
            continue;
        }
        serializer.serialize_quad(quad).unwrap();
    }
    assert!(dropped > 0, "the victim's quads were found");
    let pruned = String::from_utf8(serializer.finish().unwrap()).unwrap();
    let reopened = graphshell_rdf_trail::reopen(&pruned).unwrap().graph;
    assert_eq!(events(&reopened).len(), 1);
    assert_ne!(events(&reopened), events(&graph));
}

#[test]
fn events_sit_in_the_history_graph_with_their_recorder() {
    let graph = two_surfaces_one_address();
    let text = graphshell_rdf_trail::export(&graph, ORIGIN).unwrap();
    let quads: Vec<oxrdf::Quad> = oxttl::TriGParser::new()
        .for_slice(text.as_bytes())
        .collect::<Result<_, _>>()
        .unwrap();
    let trail = format!("urn:graphshell:browsing-trail:{ORIGIN}");
    let history = format!("urn:graphshell:browsing-history:{ORIGIN}");
    let graph_of = |subject: &str| -> BTreeSet<String> {
        quads
            .iter()
            .filter(|q| matches!(&q.subject, oxrdf::NamedOrBlankNode::NamedNode(s) if s.as_str() == subject))
            .map(|q| q.graph_name.to_string())
            .collect()
    };
    for event in events(&graph) {
        let subject = format!("urn:uuid:{}", event.2);
        assert_eq!(graph_of(&subject), BTreeSet::from([format!("<{history}>")]));
        assert!(quads.iter().any(|q| {
            q.subject.to_string() == format!("<{subject}>")
                && q.predicate.as_str() == "http://www.w3.org/ns/prov#wasAttributedTo"
                && q.object.to_string() == format!("<{ALICE}>")
        }));
    }
    let surface = format!("urn:uuid:{}", Uuid::from_u128(2));
    assert_eq!(graph_of(&surface), BTreeSet::from([format!("<{trail}>")]));
}

#[test]
fn browsing_sessions_survive_the_round_trip() {
    let mut graph = Graph::new();
    graph.set_browsing_idle_gap_ms(1_000);
    let a = node(&mut graph, 1, "https://a.test/");
    let b = node(&mut graph, 2, "https://b.test/");
    let c = node(&mut graph, 3, "https://c.test/");
    traverse(&mut graph, a, b, 0, NavigationTrigger::LinkClick);
    traverse(&mut graph, b, c, 500, NavigationTrigger::LinkClick);
    traverse(&mut graph, a, c, 5_000, NavigationTrigger::AddressBarEntry);
    graph.split_browsing_session();
    traverse(&mut graph, c, a, 5_100, NavigationTrigger::Back);
    let original = events(&graph);
    let by_time: BTreeMap<u64, Uuid> = original.iter().map(|e| (e.3, e.6)).collect();
    assert_eq!(by_time[&0], by_time[&500], "inside the gap");
    assert_ne!(by_time[&500], by_time[&5_000], "past the gap");
    assert_ne!(by_time[&5_000], by_time[&5_100], "split by hand");

    let reopened = reopen(&graph);
    assert_eq!(events(&reopened), original);
    let sessions: BTreeSet<Uuid> = events(&reopened).iter().map(|e| e.6).collect();
    assert_eq!(sessions.len(), 3);
}
