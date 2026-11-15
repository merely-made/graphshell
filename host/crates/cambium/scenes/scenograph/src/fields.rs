// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! A recipe's ordered fields list (table projection plan, T1).
//!
//! A field is described in SHACL's property-shape terms with DASH's `viewer`
//! added. List position is the order; SHACL's `order` is written on export.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::ValidationIssue;

/// The viewer a field gets when it names none: its type's plain form.
pub const DEFAULT_VIEWER: &str = "default";
/// Where our own viewer ids become IRIs in an exported shape.
pub const VIEWER_NAMESPACE: &str = "https://mere.computer/ns/viewer#";

const SHACL: &str = "http://www.w3.org/ns/shacl#";
const DASH: &str = "http://datashapes.org/dash#";
const RDFS: &str = "http://www.w3.org/2000/01/rdf-schema#";
/// Viewer ids that DASH already names.
const DASH_VIEWERS: &[(&str, &str)] = &[("thumbnail", "ImageViewer")];

/// One field of a recipe, in reading order.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FieldSpec {
    /// The dataset's key for the field.
    pub path: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// The section of columns this field belongs to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// A registered viewer id; absent means [`DEFAULT_VIEWER`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub viewer: Option<String>,
}

impl FieldSpec {
    pub fn new(path: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            name: name.into(),
            description: String::new(),
            group: None,
            viewer: None,
        }
    }

    pub fn viewer_id(&self) -> &str {
        self.viewer.as_deref().unwrap_or(DEFAULT_VIEWER)
    }
}

/// The viewer ids a host can realize. Declared as data so a host or a guest
/// can add to it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ViewerCatalog(BTreeSet<String>);

impl ViewerCatalog {
    /// The viewers of the first table: default, chip, bar and check.
    pub fn built_in() -> Self {
        Self(
            [DEFAULT_VIEWER, "chip", "bar", "check"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
        )
    }

    pub fn with(mut self, id: impl Into<String>) -> Self {
        self.0.insert(id.into());
        self
    }

    pub fn contains(&self, id: &str) -> bool {
        self.0.contains(id)
    }

    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(String::as_str)
    }
}

impl Default for ViewerCatalog {
    fn default() -> Self {
        Self::built_in()
    }
}

/// Checks that need no dataset: every field has a path and a name, and no
/// path appears twice.
pub fn validate_fields(issues: &mut Vec<ValidationIssue>, fields: &[FieldSpec]) {
    let mut seen = BTreeSet::new();
    for (index, field) in fields.iter().enumerate() {
        if field.path.trim().is_empty() {
            issues.push(ValidationIssue::error(
                &format!("fields[{index}].path"),
                "a field needs a path",
            ));
        } else if !seen.insert(field.path.as_str()) {
            issues.push(ValidationIssue::error(
                &format!("fields[{index}].path"),
                &format!("the path {:?} is listed twice", field.path),
            ));
        }
        if field.name.trim().is_empty() {
            issues.push(ValidationIssue::error(
                &format!("fields[{index}].name"),
                "a field needs a name",
            ));
        }
    }
}

/// The fields as a SHACL node shape in JSON-LD. `base` is the source's
/// namespace; a path becomes `base` plus the escaped dataset key.
pub fn to_shape(fields: &[FieldSpec], base: &str) -> Value {
    let properties: Vec<Value> = fields
        .iter()
        .enumerate()
        .map(|(order, field)| {
            let mut property = json!({
                "sh:path": { "@id": format!("{base}{}", escape(&field.path)) },
                "sh:name": field.name,
                "sh:order": order,
            });
            if !field.description.is_empty() {
                property["sh:description"] = json!(field.description);
            }
            if let Some(group) = &field.group {
                property["sh:group"] = json!({ "@type": "sh:PropertyGroup", "rdfs:label": group });
            }
            if let Some(viewer) = &field.viewer {
                property["dash:viewer"] = json!({ "@id": viewer_iri(viewer) });
            }
            property
        })
        .collect();
    json!({
        "@context": { "sh": SHACL, "dash": DASH, "rdfs": RDFS },
        "@type": "sh:NodeShape",
        "sh:property": properties,
    })
}

/// Read a shape written by [`to_shape`], or one in the same compact form,
/// back into a fields list sorted by `sh:order`.
pub fn from_shape(shape: &Value, base: &str) -> Result<Vec<FieldSpec>, String> {
    let properties = shape
        .get("sh:property")
        .and_then(Value::as_array)
        .ok_or("the shape has no sh:property list")?;
    let mut ordered = Vec::with_capacity(properties.len());
    for (index, property) in properties.iter().enumerate() {
        let text = |key: &str| property.get(key).and_then(Value::as_str);
        let id = |key: &str| property.get(key).and_then(|v| v.get("@id")).and_then(Value::as_str);
        let path = id("sh:path").ok_or(format!("property {index} has no sh:path"))?;
        let name = text("sh:name").ok_or(format!("property {index} has no sh:name"))?;
        let order = property
            .get("sh:order")
            .and_then(Value::as_f64)
            .unwrap_or(index as f64);
        let field = FieldSpec {
            path: path
                .strip_prefix(base)
                .map_or_else(|| path.to_owned(), unescape),
            name: name.to_owned(),
            description: text("sh:description").unwrap_or_default().to_owned(),
            group: property
                .get("sh:group")
                .and_then(|g| g.get("rdfs:label"))
                .and_then(Value::as_str)
                .map(str::to_owned),
            viewer: id("dash:viewer").map(viewer_id),
        };
        ordered.push((order, index, field));
    }
    ordered.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    Ok(ordered.into_iter().map(|(_, _, field)| field).collect())
}

fn viewer_iri(id: &str) -> String {
    if id.contains("://") {
        return id.to_owned();
    }
    match DASH_VIEWERS.iter().find(|(ours, _)| *ours == id) {
        Some((_, dash)) => format!("dash:{dash}"),
        None => format!("{VIEWER_NAMESPACE}{}", escape(id)),
    }
}

fn viewer_id(iri: &str) -> String {
    if let Some(local) = iri.strip_prefix(VIEWER_NAMESPACE) {
        return unescape(local);
    }
    let dash = iri.strip_prefix("dash:").or_else(|| iri.strip_prefix(DASH));
    match dash.and_then(|name| DASH_VIEWERS.iter().find(|(_, d)| *d == name)) {
        Some((ours, _)) => (*ours).to_owned(),
        None => iri.to_owned(),
    }
}

/// Percent-escape everything outside RFC 3986's unreserved set.
fn escape(key: &str) -> String {
    let mut out = String::with_capacity(key.len());
    for byte in key.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

fn unescape(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = (bytes[i] == b'%')
            .then(|| text.get(i + 1..i + 3))
            .flatten()
            .and_then(|pair| u8::from_str_radix(pair, 16).ok());
        match hex {
            Some(byte) => {
                out.push(byte);
                i += 3;
            },
            None => {
                out.push(bytes[i]);
                i += 1;
            },
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
#[path = "fields_tests.rs"]
mod tests;
