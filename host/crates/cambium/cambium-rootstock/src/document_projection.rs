// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

use std::collections::{HashMap, HashSet};

use document_session_api::{
    DocumentA11yAction, DocumentA11yBounds, DocumentA11yNode, DocumentA11yProjection,
};
use layout_dom_api::{LayoutDom, LocalName, Namespace, NodeKind};

use crate::{NodeId, OwnedLayout};

/// Attributes this host reads itself, beside Genet's projection: a textbox
/// role with committed text, its line mode, and the numeric `SetValue` opt-in.
pub(crate) const ROLE_ATTRIBUTE: &str = "role";
pub(crate) const TEXT_VALUE_ATTRIBUTE: &str = "data-cambium-text-value";
pub(crate) const MULTILINE_ATTRIBUTE: &str = "aria-multiline";
pub(crate) const SET_VALUE_ATTRIBUTE: &str = "data-cambium-set-value";
pub(crate) const HOST_ATTRIBUTES: [&str; 4] = [
    ROLE_ATTRIBUTE,
    TEXT_VALUE_ATTRIBUTE,
    MULTILINE_ATTRIBUTE,
    SET_VALUE_ATTRIBUTE,
];

fn attribute<'d, D: LayoutDom<NodeId = NodeId>>(
    dom: &'d D,
    node: NodeId,
    name: &str,
) -> Option<&'d str> {
    dom.attribute(node, &Namespace::default(), &LocalName::from(name))
}

/// One walked node: its opaque id, custom-leaf key and visible content origin.
pub(crate) struct WalkedNode {
    pub node: NodeId,
    pub opaque: u64,
    pub key: Option<u64>,
    pub content_origin: Option<(f64, f64)>,
}

/// Everything one painted-geometry walk yields for semantics (D74): the DOM
/// observation and the composition read the same walk.
pub(crate) struct PaintedFacts<'a> {
    pub order: Vec<WalkedNode>,
    pub focused: Option<NodeId>,
    styled: HashMap<NodeId, (bool, (&'a str, &'a str))>,
    pub painted: HashMap<u64, DocumentA11yBounds>,
    pub markers: HashMap<u64, (String, bool)>,
    pub numeric: HashSet<u64>,
}

impl<'a> PaintedFacts<'a> {
    pub(crate) fn collect<D: LayoutDom<NodeId = NodeId>>(
        dom: &D,
        layout: &'a OwnedLayout,
        focus: Option<u64>,
    ) -> Self {
        let mut facts = Self {
            order: Vec::new(),
            focused: None,
            styled: HashMap::new(),
            painted: HashMap::new(),
            markers: HashMap::new(),
            numeric: HashSet::new(),
        };
        // Geometry, visibility and generated content share the style owner's
        // flat traversal. No per-node walk back through the scroll ancestors.
        layout.walk_painted_geometry(dom, |node, geometry| {
            let opaque = dom.opaque_id(node);
            if focus == Some(opaque) {
                facts.focused = Some(node);
            }
            facts
                .styled
                .insert(node, (geometry.rendered, geometry.generated_text));
            if let Some((x, y, width, height)) = geometry.border_rect {
                facts.painted.insert(
                    opaque,
                    DocumentA11yBounds {
                        x,
                        y,
                        width,
                        height,
                    },
                );
            }
            if dom.kind(node) == NodeKind::Element
                && attribute(dom, node, ROLE_ATTRIBUTE)
                    .is_some_and(|role| role.trim().eq_ignore_ascii_case("textbox"))
                && let Some(value) = attribute(dom, node, TEXT_VALUE_ATTRIBUTE)
            {
                let multiline = attribute(dom, node, MULTILINE_ATTRIBUTE) == Some("true");
                facts.markers.insert(opaque, (value.to_owned(), multiline));
            }
            if attribute(dom, node, SET_VALUE_ATTRIBUTE) == Some("true") {
                facts.numeric.insert(opaque);
            }
            let content_origin = geometry
                .content_rect
                .filter(|_| geometry.placement_visible)
                .map(|(x, y, _, _)| (f64::from(x), f64::from(y)))
                .filter(|(x, y)| x.is_finite() && y.is_finite());
            facts.order.push(WalkedNode {
                node,
                opaque,
                key: crate::owned_layout::custom_leaf_key(dom, node),
                content_origin,
            });
        });
        facts
    }

    pub(crate) fn generated<D: LayoutDom<NodeId = NodeId>>(
        &self,
        dom: &D,
        layout: &OwnedLayout,
        node: NodeId,
    ) -> (String, String) {
        self.styled.get(&node).map_or_else(
            || layout.generated_text(dom, node),
            |fact| (fact.1.0.to_owned(), fact.1.1.to_owned()),
        )
    }

    pub(crate) fn rendered<D: LayoutDom<NodeId = NodeId>>(
        &self,
        dom: &D,
        layout: &OwnedLayout,
        node: NodeId,
    ) -> bool {
        self.styled
            .get(&node)
            .map_or_else(|| layout.rendered_visible(dom, node), |fact| fact.0)
    }

    /// The host's own data on one projected node: the numeric `SetValue`
    /// opt-in and the painted border box. Markers apply to the whole tree.
    pub(crate) fn host_node(&self, node: &mut DocumentA11yNode) {
        if self.numeric.contains(&node.id.get())
            && node.actions.contains(&DocumentA11yAction::Click)
            && !node.state.disabled
            && !node.state.read_only
            && !node.actions.contains(&DocumentA11yAction::SetValue)
        {
            node.actions.push(DocumentA11yAction::SetValue);
        }
        if node.bounds.is_some()
            && let Some(bounds) = self.painted.get(&node.id.get())
        {
            node.bounds = Some(*bounds);
        }
    }
}

/// The renderer-neutral DOM observation used to seed host semantics, including
/// Cambium's app-owned committed text for marked textboxes. Contributed leaf
/// and producer semantics are added by [`crate::project_semantics`], which
/// native and browser publication consume. This read does not invoke those
/// contributors. `focus` is the focused node's opaque id, as
/// [`crate::Accessibility::sync`] receives it.
pub fn document_projection<D: LayoutDom<NodeId = NodeId>>(
    dom: &D,
    layout: &OwnedLayout,
    focus: Option<u64>,
) -> DocumentA11yProjection {
    let facts = PaintedFacts::collect(dom, layout, focus);
    document_projection_with(dom, layout, focus, &facts)
}

pub(crate) fn document_projection_with<D: LayoutDom<NodeId = NodeId>>(
    dom: &D,
    layout: &OwnedLayout,
    focus: Option<u64>,
    facts: &PaintedFacts<'_>,
) -> DocumentA11yProjection {
    let projection = genet_render::document_a11y_projection_with_style(
        dom,
        layout.fragments(),
        facts.focused,
        0,
        None,
        &genet_render::A11yStyleQueries {
            // Name references may name an unassigned light-DOM node outside
            // the flat tree. Preserve the existing style query for that case.
            generated: &|node| facts.generated(dom, layout, node),
            rendered: &|node| facts.rendered(dom, layout, node),
        },
    );
    host_projection(&projection, facts, focus)
}

/// Genet supplies semantic names and actions; this host owns retained scroll
/// planes and viewport placement. Apply the shared translated border bounds
/// before platform lowering. Action routing remains bound to node identity.
pub(crate) fn host_projection(
    projection: &DocumentA11yProjection,
    facts: &PaintedFacts<'_>,
    focus: Option<u64>,
) -> DocumentA11yProjection {
    let mut nodes = projection.nodes().to_vec();
    for node in &mut nodes {
        facts.host_node(node);
    }
    if !facts.markers.is_empty() {
        apply_markers(&mut nodes, projection.root(), facts, focus);
    }
    DocumentA11yProjection::new(
        projection.revision(),
        projection.support().clone(),
        projection.root(),
        nodes,
    )
}

/// An outer committed textbox owns its entire projected subtree. Carry that
/// owner once instead of enumerating every marker's descendants.
fn apply_markers(
    nodes: &mut Vec<DocumentA11yNode>,
    root: document_session_api::DocumentA11yNodeId,
    facts: &PaintedFacts<'_>,
    focus: Option<u64>,
) {
    let markers = &facts.markers;
    let children: HashMap<_, _> = nodes
        .iter()
        .map(|node| (node.id, node.children.clone()))
        .collect();
    let mut hidden = HashSet::new();
    let mut focused_markers = HashSet::new();
    let mut pending = vec![(root, None)];
    while let Some((id, owner)) = pending.pop() {
        if let Some(owner) = owner {
            hidden.insert(id);
            if focus == Some(id.get()) {
                focused_markers.insert(owner);
            }
        }
        let owner = owner.or_else(|| markers.contains_key(&id.get()).then_some(id.get()));
        pending.extend(
            children
                .get(&id)
                .into_iter()
                .flat_map(|ids| ids.iter())
                .map(|child| (*child, owner)),
        );
    }
    nodes.retain(|node| !hidden.contains(&node.id));
    for node in nodes {
        let Some((value, multiline)) = markers.get(&node.id.get()) else {
            continue;
        };
        node.value = Some(value.clone());
        node.state.multiline = *multiline;
        node.state.editable = !node.state.disabled && !node.state.read_only;
        node.children.clear();
        node.state.focused |= focused_markers.contains(&node.id.get());
    }
}
