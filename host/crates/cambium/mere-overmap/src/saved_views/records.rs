// Copyright 2026 Mark Alan Boykin
// SPDX-License-Identifier: MPL-2.0

use kernel::graph::{CapturedDelta, Graph};
use mere_ontology::{Profile, RecordIndex, SavedView};
use portolan::FacetId;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const SOURCE: &str = "mere.named-statements/v1";
const VIEW: &str = "mere.saved-view/v1";

/// An owner's explicit named resource statements and their original source revision.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceMaterial {
    pub id: String,
    pub revision: String,
    pub statements: Vec<[String; 3]>,
}

fn address(kind: &str, key: &str) -> String {
    format!(
        "urn:mere:linked-data:{kind}:{}",
        blake3::hash(key.as_bytes()).to_hex()
    )
}

fn value<'a>(graph: &'a Graph, kind: &str, key: &str) -> Option<&'a Value> {
    let node = graph.get_node_by_url(&address(kind, key))?.1;
    graph.facets().get(&node.id, &FacetId::new(kind))
}

fn store(
    graph: &Graph,
    kind: &str,
    key: &str,
    title: &str,
    payload: Value,
) -> Result<Vec<CapturedDelta>, String> {
    if value(graph, kind, key) == Some(&payload) {
        return Ok(vec![]);
    }
    let url = address(kind, key);
    let id = Graph::node_namespace_id(&url).to_string();
    let mut edits = Vec::new();
    match graph.get_node_by_url(&url) {
        Some((_, node)) if node.id.to_string() != id => {
            return Err("The named record address belongs to a different graph node".into());
        },
        None => {
            edits.push(CapturedDelta::ReplayAddNodeWithIdIfMissing {
                id: id.clone(),
                url,
                position: [0., 0.],
            });
            edits.push(CapturedDelta::ReplaySetNodeTitleById {
                node_id: id.clone(),
                title: title.into(),
            });
        },
        _ => {},
    }
    edits.push(CapturedDelta::ReplaySetNodeFacetById {
        node_id: id,
        facet: kind.into(),
        value_json: payload.to_string(),
    });
    Ok(edits)
}

pub(super) fn save(graph: &Graph, view: &SavedView) -> Result<Vec<CapturedDelta>, String> {
    let payload = serde_json::from_str(&view.to_json()?).map_err(super::err)?;
    store(graph, VIEW, &view.name, &view.name, payload)
}

fn decode_view(value: &Value, scope: &str) -> Result<SavedView, String> {
    let view = SavedView::from_json(&value.to_string())?;
    if view.scope != scope {
        return Err("Saved definition belongs to a different owner or session".into());
    }
    Ok(view)
}
pub(super) fn open(graph: &Graph, scope: &str, name: &str) -> Result<SavedView, String> {
    let value = value(graph, VIEW, name).ok_or("This session has no saved view with that name")?;
    let view = decode_view(value, scope)?;
    if view.name != name {
        return Err("Stored definition does not match the requested name".into());
    }
    Ok(view)
}
pub(super) fn list(graph: &Graph, scope: &str) -> Result<Vec<String>, String> {
    let facet = FacetId::new(VIEW);
    let mut names = Vec::new();
    for (_, node) in graph.nodes() {
        if let Some(value) = graph.facets().get(&node.id, &facet) {
            let view = decode_view(value, scope)?;
            if node.id != Graph::node_namespace_id(&address(VIEW, &view.name)) {
                return Err("Stored definition belongs to a different node".into());
            }
            names.push(view.name);
        }
    }
    names.sort();
    Ok(names)
}

impl SourceMaterial {
    fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty() || self.revision.trim().is_empty() {
            return Err("A disclosed record source requires its stable ID and revision".into());
        }
        for term in self.statements.iter().flatten() {
            oxiri::Iri::parse(term.as_str()).map_err(|e| format!("Invalid named resource: {e}"))?;
        }
        Ok(())
    }
}
// Only the named statements the user chose to disclose under this exact lens.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Projection {
    selection: Value,
    statements: Vec<[String; 3]>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceRecord {
    id: String,
    revision: String,
    projections: Vec<Projection>,
}
fn source(graph: &Graph, id: &str) -> Result<Option<SourceRecord>, String> {
    let Some(value) = value(graph, SOURCE, id) else {
        return Ok(None);
    };
    let record: SourceRecord = serde_json::from_value(value.clone()).map_err(super::err)?;
    if record.id != id || record.revision.trim().is_empty() {
        return Err("Stored record source identity or revision is invalid".into());
    }
    Ok(Some(record))
}
pub(super) fn disclose(
    graph: &Graph,
    input: &SourceMaterial,
    profile: &Profile,
) -> Result<Vec<CapturedDelta>, String> {
    input.validate()?;
    let mut statements = input.statements.clone();
    statements.sort();
    statements.dedup();
    let selection = serde_json::to_value(&profile.selection).map_err(super::err)?;
    let mut record = source(graph, &input.id)?
        .filter(|r| r.revision == input.revision)
        .unwrap_or_else(|| SourceRecord {
            id: input.id.clone(),
            revision: input.revision.clone(),
            projections: vec![],
        });
    if let Some(old) = record.projections.iter().find(|p| p.selection == selection) {
        if old.statements != statements {
            return Err(
                "These source statements differ under the same lens and original revision".into(),
            );
        }
        return Ok(vec![]);
    }
    record.projections.push(Projection {
        selection,
        statements,
    });
    let payload = serde_json::to_value(&record).map_err(super::err)?;
    store(graph, SOURCE, &record.id, &record.id, payload)
}
pub(super) fn withdraw(graph: &Graph, id: &str) -> Result<Vec<CapturedDelta>, String> {
    let Some((_, node)) = graph.get_node_by_url(&address(SOURCE, id)) else {
        return Ok(Vec::new());
    };
    if graph
        .facets()
        .get(&node.id, &FacetId::new(SOURCE))
        .is_none()
    {
        return Ok(Vec::new());
    }
    Ok(vec![CapturedDelta::ReplayRemoveNodeFacetById {
        node_id: node.id.to_string(),
        facet: SOURCE.into(),
    }])
}
pub(super) fn index(
    graph: &Graph,
    id: &str,
    profile: &Profile,
) -> Result<Option<(String, RecordIndex)>, String> {
    let Some(source) = source(graph, id)? else {
        return Ok(None);
    };
    let selection = serde_json::to_value(&profile.selection).map_err(super::err)?;
    let mut matching = source
        .projections
        .into_iter()
        .filter(|p| p.selection == selection);
    let Some(projection) = matching.next() else {
        return Ok(None);
    };
    if matching.next().is_some() {
        return Err("Duplicate source projections for this vocabulary selection".into());
    }
    let material = SourceMaterial {
        id: source.id,
        revision: source.revision,
        statements: projection.statements,
    };
    material.validate()?;
    let index = RecordIndex::new(
        profile,
        material
            .statements
            .iter()
            .map(|[s, p, o]| (s.as_str(), p.as_str(), o.as_str())),
    );
    Ok(Some((material.revision, index)))
}
