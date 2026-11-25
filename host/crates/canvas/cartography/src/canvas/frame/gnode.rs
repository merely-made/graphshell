// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

use super::*;
use crate::canvas::grammar_view::GrammarFrame;

impl Canvas {
    /// The inline style and class one gnode carries this frame, or `None` when
    /// the node has no body to show.
    pub(super) fn gnode_presentation(
        &self,
        key: NodeKey,
        pos: PortablePoint,
        grammar: &Option<GrammarFrame>,
    ) -> Option<(String, String)> {
        let body = self.node_billboard_at(key, pos)?;
        let bounds = body.screen_rect();
        let mut style = format!(
            "transform: translate({}px, {}px) scale({}); z-index: {}; width: {}px; height: {}px; border-radius: {}px;",
            bounds.min.x,
            bounds.min.y,
            body.zoom,
            body.depth as i32,
            body.local_side,
            body.local_side,
            body.corner_radius,
        );
        if let Some(frame) = grammar {
            style.push_str(&self.grammar_fill(key, frame));
        }
        // Selection wins (orange); otherwise color by activation state —
        // green open, red closed, blue idle (the default for an unset node).
        let state_class = if self.selected.contains(&key) {
            "gnode-selected"
        } else {
            match self.node_states.get(&key) {
                Some(NodeState::Open) => "gnode-open",
                Some(NodeState::Closed) => "gnode-closed",
                _ => "gnode-idle",
            }
        };
        // Shape rides as a second class so it merges with the color class:
        // rounded and circle are border-radius, diamond and hexagon a clip;
        // square is the default (no shape class).
        let shape_class = match body.silhouette {
            NodeShape::RoundedSquare if grammar.is_none() => " gnode-rounded",
            NodeShape::Circle if grammar.is_none() => " gnode-circle",
            NodeShape::Diamond => " gnode-diamond",
            NodeShape::Hexagon => " gnode-hexagon",
            _ => "",
        };
        // The representation rung changes how the same measured footprint
        // is realized, not how much space it occupies. Glyph suppresses
        // the caption and rounds the body; Card retains the ordinary Canvas
        // face; LivePane receives a framed focus treatment. It remains a
        // face request here, not a claim that interactive content has been
        // embedded. The open representation tail falls back to Card.
        // (Projection proofs — P3b renderer consumption.)
        let representation_class = self.node_representation_class(key);
        let class = match grammar {
            Some(frame) => {
                format!(
                    "{}{shape_class}{representation_class}",
                    self.grammar_class(key, frame)
                )
            },
            None => format!("{state_class}{shape_class}{representation_class}"),
        };
        Some((style, class))
    }
}
