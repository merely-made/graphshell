// Copyright 2026 Mark Alan Boykin
// SPDX-License-Identifier: MPL-2.0

//! Graphshell's browsing-trail RDF exchange (mere:engine-split D182, D195 to
//! D199). The trail graph, named for the original workspace's scope, holds the
//! Surfaces, navigation memory and frame membership. Traversal events and each
//! edge's aggregate metrics live apart from it in the workspace's history graph.
//! Surface UUIDs identify occurrences, while an address only identifies the
//! page an occurrence visited. An event is named by the UUID minted when it was
//! recorded and names the agent that recorded it, so reopening keeps events
//! with identical payloads apart.

use std::collections::{BTreeMap, HashMap};

use kernel::geometry::PortablePoint;
use kernel::graph::{ArrangementSubKind, EdgeAssertion, Graph, SharedNavigationMemory, apply};
use mere::dataset::View;
use oxrdf::vocab::rdf;
use oxrdf::{Literal, NamedNode, Quad, Term};
use uuid::Uuid;

mod history;
mod navigation;

const NS: &str = "urn:graphshell:vocabulary:browsing-trail:1:";
const DERIVED_FROM: &str = "http://www.w3.org/ns/prov#wasDerivedFrom";
const ATTRIBUTED_TO: &str = "http://www.w3.org/ns/prov#wasAttributedTo";
const SCOPE: &str = "urn:graphshell:browsing-trail:";
const HISTORY: &str = "urn:graphshell:browsing-history:";

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
        history: named(format!("{HISTORY}{origin}")),
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
    history::write(graph, &mut out)?;
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
    let graph = history::read(&source, graph)?;

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
    uuid_of(node, "surface")
}

/// The UUID a canonical `urn:uuid:` IRI names, for a Surface or an event.
fn uuid_of(node: &NamedNode, what: &str) -> Result<Uuid> {
    let id = Uuid::parse_str(
        node.as_str()
            .strip_prefix("urn:uuid:")
            .ok_or_else(|| format!("invalid {what} identity"))?,
    )
    .map_err(|err| err.to_string())?;
    if surface(id) != *node {
        return Err(format!("noncanonical {what} identity {node}"));
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
    history: NamedNode,
    quads: Vec<Quad>,
}

impl Writer {
    fn write(&mut self, subject: &NamedNode, predicate: NamedNode, object: impl Into<Term>) {
        let graph = self.scope.clone();
        self.write_in(graph, subject, predicate, object);
    }

    fn write_in(
        &mut self,
        graph: NamedNode,
        subject: &NamedNode,
        predicate: NamedNode,
        object: impl Into<Term>,
    ) {
        self.quads
            .push(Quad::new(subject.clone(), predicate, object, graph));
    }

    /// A statement in the workspace's history graph.
    fn history_set(&mut self, subject: &NamedNode, predicate: NamedNode, object: impl Into<Term>) {
        let graph = self.history.clone();
        self.write_in(graph, subject, predicate, object);
    }

    fn history_kind(&mut self, subject: &NamedNode, class: &str) {
        self.history_set(subject, rdf::TYPE.into_owned(), vocab(class));
        let history = self.history.clone();
        self.history_set(subject, named(DERIVED_FROM), history);
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
    history: NamedNode,
    records: BTreeMap<NamedNode, HashMap<NamedNode, Vec<Term>>>,
    /// The named graph each subject was read from.
    graphs: HashMap<NamedNode, NamedNode>,
}

impl Reader {
    fn new(quads: impl IntoIterator<Item = Quad>) -> Result<Self> {
        let mut records: BTreeMap<NamedNode, HashMap<NamedNode, Vec<Term>>> = BTreeMap::new();
        let mut graphs: HashMap<NamedNode, NamedNode> = HashMap::new();
        let (mut scope, mut history) = (None, None);
        for quad in quads {
            let oxrdf::GraphName::NamedNode(graph) = quad.graph_name else {
                return Err("the trail must be in its source named graphs".into());
            };
            let slot = if graph.as_str().starts_with(SCOPE) {
                &mut scope
            } else if graph.as_str().starts_with(HISTORY) {
                &mut history
            } else {
                return Err(format!("unexpected named graph {graph}"));
            };
            if slot.as_ref().is_some_and(|existing| existing != &graph) {
                return Err("a browsing trail file must contain one source graph".into());
            }
            *slot = Some(graph.clone());
            let oxrdf::NamedOrBlankNode::NamedNode(subject) = quad.subject else {
                return Err("trail records require named identities".into());
            };
            if graphs
                .insert(subject.clone(), graph.clone())
                .is_some_and(|seen| seen != graph)
            {
                return Err(format!("{subject} is described in two graphs"));
            }
            records
                .entry(subject)
                .or_default()
                .entry(quad.predicate)
                .or_default()
                .push(quad.object);
        }
        let scope = scope.ok_or("empty browsing trail")?;
        let origin = scope.as_str().strip_prefix(SCOPE).unwrap_or_default();
        let expected = named(format!("{HISTORY}{origin}"));
        if history.as_ref().is_some_and(|history| history != &expected) {
            return Err(format!("the history graph is not {expected}"));
        }
        let source = Self {
            scope,
            history: expected,
            records,
            graphs,
        };
        for node in source.records.keys().filter(|node| *node != &source.scope) {
            let graph = &source.graphs[node];
            if source.one(node, DERIVED_FROM)? != &Term::from(graph.clone()) {
                return Err(format!("{node} has inconsistent source provenance"));
            }
        }
        Ok(source)
    }

    /// Subjects of `class` described in the history graph.
    fn history_typed<'a>(&'a self, class: &str) -> impl Iterator<Item = &'a NamedNode> {
        self.typed(class)
            .filter(|node| self.graphs.get(*node) == Some(&self.history))
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
