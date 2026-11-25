// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! ARIA lowering of Rootstock's fully composed host semantics. Leaf callbacks,
//! producer children, IDs and action maps are composed by the shared host.
use accesskit::{Action, HasPopup, Live, Node, NodeId, Orientation, Role, Toggled};
use cambium_rootstock::SemanticProjection;
use std::collections::HashMap;

/// One mirror element: what the DOM element for one projected node carries.
#[derive(Clone, Debug, PartialEq)]
pub struct MirrorNode {
    /// The composed ID: a DOM opaque ID or a generated producer/action ID.
    pub id: u64,
    /// ARIA attributes, `role` first when there is one.
    pub attrs: Vec<(&'static str, String)>,
    /// The element's own text, ahead of its children.
    pub text: Option<String>,
    /// `[x, y, width, height]` in CSS pixels, relative to the nearest mirrored
    /// ancestor's box, or to the canvas at the top.
    pub rect: Option<[f32; 4]>,
    /// Whether a reader may move focus to it.
    pub focusable: bool,
    /// For a drawn node's action button, the action a reader's click on it
    /// names (dynamics grammar plan, F65 and F66).
    pub action: Option<cambium_rootstock::ProducedAction>,
    pub children: Vec<MirrorNode>,
}

/// Lower a copied host observation. Geometry stays in layout pixels until this
/// platform scale is applied; producer action targets come from the same read.
pub fn plan(projection: &SemanticProjection, scale: f32) -> Vec<MirrorNode> {
    let nodes: HashMap<_, _> = projection
        .tree
        .nodes
        .iter()
        .map(|(id, node)| (*id, node))
        .collect();
    let Some(root) = projection
        .tree
        .tree
        .as_ref()
        .and_then(|tree| nodes.get(&tree.root))
    else {
        return Vec::new();
    };
    let place = Place {
        space: translation(root),
        origin: [0.0, 0.0],
    };
    lower_children(&nodes, root, place, scale, &projection.produced_targets)
}

/// Where a parent puts its children: `space` is the parent's coordinate
/// origin, summed from the parent-relative translates (table plan D71);
/// `origin` is the nearest mirrored box's corner, which rects are relative to.
#[derive(Clone, Copy)]
struct Place {
    space: [f64; 2],
    origin: [f64; 2],
}

fn translation(node: &Node) -> [f64; 2] {
    node.transform().map_or([0.0, 0.0], |transform| {
        let [_, _, _, _, x, y] = transform.as_coeffs();
        [x, y]
    })
}

fn lower_children(
    nodes: &HashMap<NodeId, &Node>,
    parent: &Node,
    place: Place,
    scale: f32,
    actions: &HashMap<NodeId, cambium_rootstock::ProducedAction>,
) -> Vec<MirrorNode> {
    parent
        .children()
        .iter()
        .filter_map(|id| nodes.get(id).map(|node| (*id, *node)))
        .filter(|(_, node)| !node.is_hidden())
        .map(|(id, node)| lower(nodes, id, node, place, scale, actions))
        .collect()
}
fn lower(
    nodes: &HashMap<NodeId, &Node>,
    id: NodeId,
    node: &Node,
    place: Place,
    scale: f32,
    actions: &HashMap<NodeId, cambium_rootstock::ProducedAction>,
) -> MirrorNode {
    let offset = translation(node);
    let space = [place.space[0] + offset[0], place.space[1] + offset[1]];
    let origin = place.origin;
    let role = aria_role(node.role());
    let mut attrs = Vec::new();
    if let Some(role) = role {
        attrs.push(("role", role.to_owned()));
    }
    let mut text = None;
    if let Some(name) = node.label().filter(|name| !name.trim().is_empty()) {
        if label_role(node.role()) || (node.role() == Role::ListItem && !node.children().is_empty())
        {
            attrs.push(("aria-label", name.to_owned()));
        } else {
            text = Some(name.to_owned());
        }
    }
    if let Some(description) = node.description().filter(|s| !s.trim().is_empty()) {
        attrs.push(("aria-description", description.to_owned()));
    }
    if matches!(
        node.role(),
        Role::TextInput | Role::MultilineTextInput | Role::ComboBox
    ) {
        text = node.value().filter(|s| !s.is_empty()).map(str::to_owned);
    }
    attrs.extend(states(node));
    let (rect, child_origin) = node.bounds().map_or((None, origin), |bounds| {
        let (x0, y0) = (bounds.x0 + space[0], bounds.y0 + space[1]);
        (
            Some([
                ((x0 - origin[0]) * f64::from(scale)) as f32,
                ((y0 - origin[1]) * f64::from(scale)) as f32,
                ((bounds.x1 - bounds.x0) * f64::from(scale)) as f32,
                ((bounds.y1 - bounds.y0) * f64::from(scale)) as f32,
            ]),
            [x0, y0],
        )
    });
    let place = Place {
        space,
        origin: child_origin,
    };
    MirrorNode {
        id: id.0,
        attrs,
        text,
        rect,
        focusable: node.supports_action(Action::Focus),
        action: actions.get(&id).cloned(),
        children: lower_children(nodes, node, place, scale, actions),
    }
}

/// Every drawn node's action button in `nodes` and their descendants: its
/// mirror id and the action a click on it names.
pub fn produced_actions(nodes: &[MirrorNode]) -> Vec<(u64, cambium_rootstock::ProducedAction)> {
    let mut out = Vec::new();
    let mut stack: Vec<&MirrorNode> = nodes.iter().collect();
    while let Some(node) = stack.pop() {
        if let Some(action) = &node.action {
            out.push((node.id, action.clone()));
        }
        stack.extend(node.children.iter());
    }
    out
}

fn label_role(role: Role) -> bool {
    matches!(
        role,
        Role::TextInput
            | Role::MultilineTextInput
            | Role::ComboBox
            | Role::Slider
            | Role::SpinButton
            | Role::Splitter
            | Role::Document
            | Role::Article
            | Role::Region
            | Role::Group
            | Role::Navigation
            | Role::Main
            | Role::Form
            | Role::Dialog
            | Role::List
            | Role::ListBox
            | Role::ListBoxOption
            | Role::Table
            | Role::Row
            | Role::TabList
            | Role::TabPanel
            | Role::Tree
            | Role::Menu
            | Role::MenuBar
            | Role::Toolbar
            | Role::RadioGroup
            | Role::ProgressIndicator
            | Role::Meter
            | Role::Image
            | Role::Canvas
            | Role::ColorWell
            | Role::GraphicsDocument
            | Role::GraphicsObject
            | Role::GraphicsSymbol
    )
}
pub use document_session_api::a11y::accesskit_aria_role as aria_role;

fn states(node: &Node) -> Vec<(&'static str, String)> {
    let mut attrs = Vec::new();
    let mut push = |name, value: Option<String>| {
        if let Some(value) = value {
            attrs.push((name, value));
        }
    };
    let flag = |value: bool| value.then(|| "true".to_owned());
    if node.role() == Role::Heading {
        // AccessKit levels are zero-based; aria-level is one-based.
        push("aria-level", node.level().map(|v| (v + 1).to_string()));
    }
    push("aria-disabled", flag(node.is_disabled()));
    push("aria-selected", node.is_selected().map(|v| v.to_string()));
    push("aria-expanded", node.is_expanded().map(|v| v.to_string()));
    let toggled = node.toggled().map(|v| {
        match v {
            Toggled::True => "true",
            Toggled::False => "false",
            Toggled::Mixed => "mixed",
        }
        .to_owned()
    });
    push(
        if node.role() == Role::Button {
            "aria-pressed"
        } else {
            "aria-checked"
        },
        toggled,
    );
    if matches!(node.role(), Role::TextInput | Role::MultilineTextInput) {
        push(
            "aria-multiline",
            flag(node.role() == Role::MultilineTextInput),
        );
    }
    push("aria-readonly", flag(node.is_read_only()));
    push("aria-required", flag(node.is_required()));
    push(
        "aria-live",
        node.live().map(|v| {
            match v {
                Live::Off => "off",
                Live::Polite => "polite",
                Live::Assertive => "assertive",
            }
            .to_owned()
        }),
    );
    push(
        "aria-orientation",
        node.orientation().map(|v| {
            match v {
                Orientation::Horizontal => "horizontal",
                Orientation::Vertical => "vertical",
            }
            .to_owned()
        }),
    );
    push(
        "aria-haspopup",
        node.has_popup().map(|v| {
            match v {
                HasPopup::Menu => "menu",
                HasPopup::Listbox => "listbox",
                HasPopup::Tree => "tree",
                HasPopup::Grid => "grid",
                HasPopup::Dialog => "dialog",
            }
            .to_owned()
        }),
    );
    push(
        "aria-valuenow",
        node.numeric_value()
            .filter(|v| v.is_finite())
            .map(|v| v.to_string()),
    );
    push(
        "aria-valuemin",
        node.min_numeric_value()
            .filter(|v| v.is_finite())
            .map(|v| v.to_string()),
    );
    push(
        "aria-valuemax",
        node.max_numeric_value()
            .filter(|v| v.is_finite())
            .map(|v| v.to_string()),
    );
    attrs
}
#[cfg(test)]
mod tests;
