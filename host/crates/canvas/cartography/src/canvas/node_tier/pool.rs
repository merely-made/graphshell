// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The bounded gnode pool: recycled `.gnode` elements, none tied to a node.

use genet_scripted_dom::{NodeId as DomNodeId, ScriptedDom};
use layout_dom_api::{LayoutDom, LayoutDomMut};

use crate::canvas::build::{qual, stage_style};

/// Whether the node document holds one gnode per graph node or a bounded pool.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DomMode {
    Full,
    Bounded(usize),
}

/// One recycled element: the gnode, its caption, and what was last written.
pub(crate) struct PoolDom {
    pub gnode: DomNodeId,
    pub caption: DomNodeId,
    pub label: String,
    pub hidden: bool,
}

/// A `.stage` over `capacity` hidden gnodes, each with an empty caption.
pub(crate) fn build_bounded_pool_dom(capacity: usize) -> (ScriptedDom, Vec<PoolDom>, DomNodeId) {
    let mut dom = ScriptedDom::new();
    let root = dom.document();
    let stage = dom.create_element(qual("div"));
    dom.set_attribute(stage, qual("class"), "stage");
    dom.set_attribute(stage, qual("style"), &stage_style(None));
    dom.append_child(root, stage);
    let slots = (0..capacity)
        .map(|_| {
            let gnode = dom.create_element(qual("div"));
            dom.set_attribute(gnode, qual("class"), "gnode");
            dom.set_attribute(gnode, qual("style"), HIDDEN);
            let caption = dom.create_element(qual("div"));
            dom.set_attribute(caption, qual("class"), "gcaption");
            dom.append_child(gnode, caption);
            dom.append_child(stage, gnode);
            PoolDom {
                gnode,
                caption,
                label: String::new(),
                hidden: true,
            }
        })
        .collect();
    (dom, slots, stage)
}

/// The style an unassigned element holds.
pub(crate) const HIDDEN: &str = "display: none;";
