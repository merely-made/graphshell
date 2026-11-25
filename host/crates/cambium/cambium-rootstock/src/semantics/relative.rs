// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Parent-relative placement (table plan D71): each node carries a translate
//! from its accessibility parent and bounds at its own origin, so a moved
//! container changes one node. Final positions are unchanged.
use super::SemanticProjection;
use accesskit::{Affine, Node, NodeId as A11yNodeId, Rect, TreeUpdate, Vec2};
use std::collections::HashMap;

/// A node's origin: its absolute box corner, or its parent's when it has none.
pub(crate) fn origin(absolute: Option<Rect>, parent: (f64, f64)) -> (f64, f64) {
    absolute.map_or(parent, |rect| (rect.x0, rect.y0))
}

/// Write `absolute` onto `node` relative to `parent`, the parent's origin.
pub(crate) fn place(node: &mut Node, absolute: Option<Rect>, parent: (f64, f64)) {
    let Some(rect) = absolute else {
        node.clear_transform();
        node.clear_bounds();
        return;
    };
    let offset = (rect.x0 - parent.0, rect.y0 - parent.1);
    if offset == (0.0, 0.0) {
        node.clear_transform();
    } else {
        node.set_transform(Affine::translate(Vec2::new(offset.0, offset.1)));
    }
    node.set_bounds(Rect::new(0.0, 0.0, rect.width(), rect.height()));
}

/// The translation a node's transform carries; host transforms are
/// translation-only below the window root.
pub(crate) fn translation(node: &Node) -> (f64, f64) {
    node.transform().map_or((0.0, 0.0), |transform| {
        let [_, _, _, _, x, y] = transform.as_coeffs();
        (x, y)
    })
}

/// Rewrite a tree composed in absolute coordinates as parent-relative.
pub(crate) fn make_relative(tree: &mut TreeUpdate) {
    let Some(root) = tree.tree.as_ref().map(|info| info.root) else {
        return;
    };
    let index: HashMap<_, _> = tree
        .nodes
        .iter()
        .enumerate()
        .map(|(i, (id, _))| (*id, i))
        .collect();
    let mut pending = vec![(root, (0.0, 0.0))];
    while let Some((id, parent)) = pending.pop() {
        let Some(&i) = index.get(&id) else {
            continue;
        };
        let node = &mut tree.nodes[i].1;
        let absolute = node.bounds();
        place(node, absolute, parent);
        let here = origin(absolute, parent);
        pending.extend(node.children().iter().map(|child| (*child, here)));
    }
}

impl SemanticProjection {
    /// A node's bounds in the host's layout coordinates, summed through its
    /// ancestors' translates.
    pub fn absolute_bounds(&self, id: A11yNodeId) -> Option<Rect> {
        let mut parents = HashMap::new();
        let mut nodes = HashMap::new();
        for (node_id, node) in &self.tree.nodes {
            nodes.insert(*node_id, node);
            for child in node.children() {
                parents.insert(*child, *node_id);
            }
        }
        let bounds = nodes.get(&id)?.bounds()?;
        let (mut x, mut y) = (0.0, 0.0);
        let mut at = Some(id);
        while let Some(node) = at.and_then(|at| nodes.get(&at)) {
            let (dx, dy) = translation(node);
            x += dx;
            y += dy;
            at = at.and_then(|at| parents.get(&at).copied());
        }
        Some(Rect::new(
            bounds.x0 + x,
            bounds.y0 + y,
            bounds.x1 + x,
            bounds.y1 + y,
        ))
    }
}
