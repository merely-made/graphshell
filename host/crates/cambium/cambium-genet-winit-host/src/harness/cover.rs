// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Whether a click target is covered at the point the harness would click.

use genet_scripted_dom::NodeId;
use layout_dom_api::{LayoutDom, LocalName, Namespace};

/// Longest label text quoted in a description.
const TEXT_LIMIT: usize = 40;

/// `None` when `hit` is `target` or inside it; otherwise what covers it.
pub(super) fn covered<D: LayoutDom<NodeId = NodeId>>(
    dom: &D,
    target: NodeId,
    hit: Option<NodeId>,
) -> Option<String> {
    let covering = match hit {
        Some(hit) if within(dom, target, hit) => return None,
        Some(hit) => describe(dom, hit),
        None => "nothing (no node is hit there)".to_string(),
    };
    Some(format!(
        "{} is covered by {covering}",
        describe(dom, target)
    ))
}

/// Whether `node` is `target` or a descendant of it, shadow trees included.
fn within<D: LayoutDom<NodeId = NodeId>>(dom: &D, target: NodeId, node: NodeId) -> bool {
    let mut current = Some(node);
    while let Some(id) = current {
        if id == target {
            return true;
        }
        current = dom.parent(id).or_else(|| dom.shadow_host(id));
    }
    false
}

/// `<tag id="…" class="…"> "text"` for an element; the parent's for a text node.
fn describe<D: LayoutDom<NodeId = NodeId>>(dom: &D, node: NodeId) -> String {
    let Some(name) = dom.element_name(node) else {
        return match dom.parent(node) {
            Some(parent) => format!("text in {}", describe(dom, parent)),
            None => format!("node {node:?}"),
        };
    };
    let mut out = format!("<{}", name.local);
    for attr in ["id", "class", "role"] {
        if let Some(value) = dom.attribute(node, &Namespace::from(""), &LocalName::from(attr)) {
            out.push_str(&format!(" {attr}=\"{value}\""));
        }
    }
    out.push('>');
    let mut text = String::new();
    collect_text(dom, node, &mut text);
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if !text.is_empty() {
        let quoted: String = text.chars().take(TEXT_LIMIT).collect();
        let more = if text.chars().count() > TEXT_LIMIT {
            "…"
        } else {
            ""
        };
        out.push_str(&format!(" \"{quoted}{more}\""));
    }
    out
}

fn collect_text<D: LayoutDom<NodeId = NodeId>>(dom: &D, node: NodeId, out: &mut String) {
    if out.len() > TEXT_LIMIT * 4 {
        return;
    }
    if let Some(text) = dom.text(node) {
        out.push_str(text);
        out.push(' ');
    }
    for child in dom.dom_children(node) {
        collect_text(dom, child, out);
    }
}
