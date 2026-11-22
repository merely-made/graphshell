// Copyright 2026 Mark Alan Boykin
// SPDX-License-Identifier: MPL-2.0

//! Graphshell's browsing-trail RDF exchange. The named graph is the original
//! workspace's scope. Surface UUIDs and visit positions identify occurrences,
//! while an address only identifies the page an occurrence visited.
//! Event identifiers are scoped to the exported trail. Reopening retains the
//! scope and the event order, including events with identical payloads.

use std::collections::{BTreeMap, HashMap};

use kernel::geometry::PortablePoint;
use kernel::graph::{ArrangementSubKind, EdgeAssertion, Graph, SharedNavigationMemory, apply};
use mere::dataset::View;
use oxrdf::vocab::rdf;
use oxrdf::{Literal, NamedNode, Quad, Term};
use uuid::Uuid;

mod navigation;

const NS: &str = "urn:graphshell:vocabulary:browsing-trail:1:";
const DERIVED_FROM: &str = "http://www.w3.org/ns/prov#wasDerivedFrom";
const SCOPE: &str = "urn:graphshell:browsing-trail:";

/// Imported origin scope is kept independently of the new local session ID.
/// The scope records the source named by the file, without authenticating it.
pub struct ReopenedTrail {
    pub origin: Uuid,
    pub graph: Graph,
}

type Result<T> = std::result::Result<T, String>;

/// Export the live trail and its frame membership as a TriG dataset.
pub fn export(graph: &Graph, origin: Uuid) -> Result<String> {
    let scope = named(format!("{SCOPE}{origin}"));
    let mut out = Writer {
        scope: scope.clone(),
        quads: Vec::new(),
    };
    out.write(&scope, rdf::TYPE.into_owned(), vocab("BrowsingTrail"));
    let snapshot = graph.to_snapshot_at(0);
    for node in &snapshot.nodes {
        let id = Uuid::parse_str(&node.node_id).map_err(|err| err.to_string())?;
        let subject = surface(id);
        out.kind(&subject, "Surface");
        out.set(
            &subject,
            "address",
            Literal::from(node.address.as_url_str()),
        );
    }
    navigation::write(snapshot.navigation.snapshot(), &mut out)?;
    let mut traversals = BTreeMap::<_, Vec<&mere::model::edge_data::Traversal>>::new();
    for (_, from, to, payload) in graph.surface_relation_buckets() {
        if let Some(data) = &payload.traversal {
            let from = graph.get_node(from).ok_or("missing source surface")?.id;
            let to = graph.get_node(to).ok_or("missing target surface")?.id;
            traversals
                .entry((from, to))
                .or_default()
                .extend(&data.traversals);
        }
    }
    let events = traversals
        .into_iter()
        .flat_map(|((from, to), events)| events.into_iter().map(move |event| (from, to, event)));
    for (index, (from, to, event)) in events.enumerate() {
        let row = out.row("traversal", index, "Traversal");
        out.set(&row, "fromSurface", surface(from));
        out.set(&row, "toSurface", surface(to));
        out.set(&row, "timestampMs", unsigned(event.timestamp_ms));
        out.set(&row, "trigger", enum_term(event.trigger)?);
    }
    let mut frames = std::collections::BTreeSet::new();
    for relation in graph.arrangement_edges() {
        if relation.sub_kind == ArrangementSubKind::FrameMember {
            let from = graph
                .get_node(relation.from)
                .ok_or("missing frame surface")?
                .id;
            let to = graph
                .get_node(relation.to)
                .ok_or("missing member surface")?
                .id;
            frames.insert((from, to));
        }
    }
    for (index, (from, to)) in frames.into_iter().enumerate() {
        let row = out.row("membership", index, "FrameMembership");
        out.set(&row, "fromSurface", surface(from));
        out.set(&row, "toSurface", surface(to));
    }
    out.quads.sort_by_cached_key(ToString::to_string);
    let mut serializer = oxttl::TriGSerializer::new().for_writer(Vec::new());
    for quad in &out.quads {
        serializer
            .serialize_quad(quad)
            .map_err(|err| err.to_string())?;
    }
    String::from_utf8(serializer.finish().map_err(|err| err.to_string())?)
        .map_err(|err| err.to_string())
}

/// Decode a trail into an independent graph, retaining the original surface IDs.
pub fn reopen(text: &str) -> Result<ReopenedTrail> {
    let dataset = mere::formats::parse::parse_trig(text, None).map_err(|err| err.to_string())?;
    let source = Reader::new(dataset.quads(View::Live))?;
    let origin = Uuid::parse_str(
        source
            .scope
            .as_str()
            .strip_prefix(SCOPE)
            .ok_or("invalid trail scope")?,
    )
    .map_err(|err| err.to_string())?;
    if source.one(&source.scope, rdf::TYPE.as_str())? != &Term::from(vocab("BrowsingTrail")) {
        return Err("unsupported browsing trail profile".into());
    }

    let mut graph = Graph::new();
    for row in source.typed("Surface") {
        let id = surface_id(row)?;
        let address = source.text(row, "address")?;
        apply::add_node(&mut graph, Some(id), address, PortablePoint::new(0.0, 0.0));
    }
    let mut snapshot = graph.to_snapshot_at(0);
    snapshot.navigation = SharedNavigationMemory::from_snapshot(navigation::read(&source)?);
    let mut graph = Graph::try_from_snapshot(&snapshot).map_err(|err| err.to_string())?;

    for row in source.rows("membership", "FrameMembership")? {
        let (from, to) = source.endpoints(&graph, row)?;
        apply::assert_relation(
            &mut graph,
            from,
            to,
            EdgeAssertion::Arrangement {
                sub_kind: ArrangementSubKind::FrameMember,
            },
        )
        .ok_or("frame membership could not be restored")?;
    }
    for row in source.rows("traversal", "Traversal")? {
        let (from, to) = source.endpoints(&graph, row)?;
        let trigger = source.enumeration(row, "trigger")?;
        let timestamp_ms = Some(source.uint(row, "timestampMs")?);
        let outcome = apply::apply_graph_delta(
            &mut graph,
            apply::GraphDelta::AppendTraversal {
                from,
                to,
                trigger,
                timestamp_ms,
            },
        );
        if !matches!(outcome, apply::GraphDeltaResult::TraversalAppended(true)) {
            return Err(format!("traversal endpoints refused on {row}"));
        }
    }

    Ok(ReopenedTrail { origin, graph })
}

fn named(value: impl Into<String>) -> NamedNode {
    NamedNode::new_unchecked(value)
}

fn vocab(local: &str) -> NamedNode {
    named(format!("{NS}{local}"))
}

fn surface(id: Uuid) -> NamedNode {
    named(format!("urn:uuid:{id}"))
}

fn surface_id(node: &NamedNode) -> Result<Uuid> {
    let id = Uuid::parse_str(
        node.as_str()
            .strip_prefix("urn:uuid:")
            .ok_or("invalid surface identity")?,
    )
    .map_err(|err| err.to_string())?;
    if surface(id) != *node {
        return Err(format!("noncanonical surface identity {node}"));
    }
    Ok(id)
}

fn unsigned(value: u64) -> Literal {
    Literal::new_typed_literal(value.to_string(), oxrdf::vocab::xsd::UNSIGNED_LONG)
}

fn enum_term(value: impl serde::Serialize) -> Result<NamedNode> {
    let value = serde_json::to_value(value).map_err(|err| err.to_string())?;
    Ok(vocab(value.as_str().ok_or("expected a unit enum")?))
}

struct Writer {
    scope: NamedNode,
    quads: Vec<Quad>,
}

impl Writer {
    fn write(&mut self, subject: &NamedNode, predicate: NamedNode, object: impl Into<Term>) {
        self.quads.push(Quad::new(
            subject.clone(),
            predicate,
            object,
            self.scope.clone(),
        ));
    }

    fn set(&mut self, subject: &NamedNode, predicate: &str, object: impl Into<Term>) {
        self.write(subject, vocab(predicate), object);
    }

    fn kind(&mut self, subject: &NamedNode, class: &str) {
        self.write(subject, rdf::TYPE.into_owned(), vocab(class));
        self.write(subject, named(DERIVED_FROM), self.scope.clone());
    }

    fn id(&self, table: &str, index: usize) -> NamedNode {
        named(format!("{}:{table}:{index}", self.scope.as_str()))
    }

    fn row(&mut self, table: &str, index: usize, class: &str) -> NamedNode {
        let node = self.id(table, index);
        self.kind(&node, class);
        node
    }
}

struct Reader {
    scope: NamedNode,
    records: BTreeMap<NamedNode, HashMap<NamedNode, Vec<Term>>>,
}

impl Reader {
    fn new(quads: impl IntoIterator<Item = Quad>) -> Result<Self> {
        let mut records: BTreeMap<NamedNode, HashMap<NamedNode, Vec<Term>>> = BTreeMap::new();
        let mut scope = None;
        for quad in quads {
            let oxrdf::GraphName::NamedNode(graph) = quad.graph_name else {
                return Err("the trail must be in its source named graph".into());
            };
            if scope.as_ref().is_some_and(|existing| existing != &graph) {
                return Err("a browsing trail file must contain one source graph".into());
            }
            scope = Some(graph);
            let oxrdf::NamedOrBlankNode::NamedNode(subject) = quad.subject else {
                return Err("trail records require named identities".into());
            };
            records
                .entry(subject)
                .or_default()
                .entry(quad.predicate)
                .or_default()
                .push(quad.object);
        }
        let scope = scope.ok_or("empty browsing trail")?;
        let source = Self { scope, records };
        for node in source.records.keys().filter(|node| *node != &source.scope) {
            if source.one(node, DERIVED_FROM)? != &Term::from(source.scope.clone()) {
                return Err(format!("{node} has inconsistent source provenance"));
            }
        }
        Ok(source)
    }

    fn values(&self, node: &NamedNode, property: &str) -> &[Term] {
        self.records
            .get(node)
            .and_then(|p| p.get(&named(property)))
            .map_or(&[], Vec::as_slice)
    }

    fn optional(&self, node: &NamedNode, property: &str) -> Result<Option<&Term>> {
        match self.values(node, property) {
            [] => Ok(None),
            [value] => Ok(Some(value)),
            _ => Err(format!("multiple {property} values on {node}")),
        }
    }

    fn one(&self, node: &NamedNode, property: &str) -> Result<&Term> {
        self.optional(node, property)?
            .ok_or_else(|| format!("missing {property} on {node}"))
    }

    fn get(&self, node: &NamedNode, property: &str) -> Result<&Term> {
        self.one(node, vocab(property).as_str())
    }

    fn text(&self, node: &NamedNode, property: &str) -> Result<String> {
        match self.get(node, property)? {
            Term::Literal(value) if value.datatype() == oxrdf::vocab::xsd::STRING => {
                Ok(value.value().into())
            },
            _ => Err(format!("expected a string {property} on {node}")),
        }
    }

    fn uint(&self, node: &NamedNode, property: &str) -> Result<u64> {
        match self.get(node, property)? {
            Term::Literal(value) if value.datatype() == oxrdf::vocab::xsd::UNSIGNED_LONG => value
                .value()
                .parse()
                .map_err(|_| format!("invalid unsigned {property} on {node}")),
            _ => Err(format!("expected an unsigned {property} on {node}")),
        }
    }

    fn enumeration<T: serde::de::DeserializeOwned>(
        &self,
        node: &NamedNode,
        property: &str,
    ) -> Result<T> {
        let term = self.get(node, property)?;
        let raw = as_named(term)?
            .as_str()
            .strip_prefix(NS)
            .ok_or("invalid vocabulary term")?;
        serde_json::from_value(serde_json::Value::String(raw.into())).map_err(|err| err.to_string())
    }

    fn typed<'a>(&'a self, class: &str) -> impl Iterator<Item = &'a NamedNode> {
        let class = Term::from(vocab(class));
        self.records
            .keys()
            .filter(move |subject| self.values(subject, rdf::TYPE.as_str()).contains(&class))
    }

    fn index(&self, term: &Term, table: &str) -> Result<usize> {
        let prefix = format!("{}:{table}:", self.scope.as_str());
        let node = as_named(term)?;
        let idx = node
            .as_str()
            .strip_prefix(&prefix)
            .ok_or("record reference is out of scope")?;
        idx.parse().map_err(|_| format!("invalid index on {node}"))
    }

    fn rows(&self, table: &str, class: &str) -> Result<Vec<&NamedNode>> {
        let mut rows = self
            .typed(class)
            .map(|node| Ok((self.index(&Term::from(node.clone()), table)?, node)))
            .collect::<Result<Vec<_>>>()?;
        rows.sort_by_key(|(index, _)| *index);
        rows.into_iter()
            .enumerate()
            .map(|(expected, (actual, node))| {
                if actual == expected {
                    Ok(node)
                } else {
                    Err(format!("invalid {table} sequence at {node}"))
                }
            })
            .collect()
    }

    fn endpoints(
        &self,
        graph: &Graph,
        row: &NamedNode,
    ) -> Result<(kernel::graph::NodeKey, kernel::graph::NodeKey)> {
        let get = |prop| {
            let id = surface_id(as_named(self.get(row, prop)?)?)?;
            graph
                .get_node_key_by_id(id)
                .ok_or_else(|| format!("unknown surface {id}"))
        };
        Ok((get("fromSurface")?, get("toSurface")?))
    }
}

fn as_named(term: &Term) -> Result<&NamedNode> {
    match term {
        Term::NamedNode(node) => Ok(node),
        _ => Err("expected an IRI reference".into()),
    }
}
