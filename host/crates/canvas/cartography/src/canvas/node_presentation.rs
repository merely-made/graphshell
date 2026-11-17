// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0
//! Rule-chosen detail rungs over the existing resolved body footprint.
use super::*;
impl Canvas {
    /// Local replacement epoch; never a source/session identity.
    pub fn graph_view_epoch(&self) -> u64 {
        self.graph_view_epoch
    }
    /// Current outer body side in screen pixels. Caption/occlusion changes
    /// cannot feed back into the resolved footprint used by face paint.
    pub fn node_presentation_screen_size(&self, member: uuid::Uuid) -> Option<f64> {
        let key = self.graph.get_node_key_by_id(member)?;
        let billboard = self.node_billboard(key)?;
        let size = f64::from(billboard.local_side * billboard.zoom);
        (size.is_finite() && size >= 0.).then_some(size)
    }
    /// Effective CSS body corner radius in local (pre-camera-zoom) pixels.
    /// Polygonal bodies are clipped to their outline. For other bodies, the
    /// graph grammar and Glyph treatment provide their usual corner radii.
    pub(crate) fn node_effective_local_corner_radius(&self, key: NodeKey, side: f32) -> f32 {
        let half = side * 0.5;
        let shape = self.node_shape(key);
        let mark_body = matches!(
            self.requested_node_mark(key).map(|mark| mark.body),
            Some(scenograph::presentation::Body::Silhouette(_)),
        );
        if shape.outline().is_some() {
            return 0.0;
        }
        if self.graph_grammar && !mark_body {
            let attention = self
                .graph
                .get_node(key)
                .and_then(|node| self.member_attention(node.id));
            return match attention {
                Some(scenograph::attention::Attention::Forward)
                | Some(scenograph::attention::Attention::Suggestion) => half,
                _ => 2.0_f32.min(half), // `.gnode-site { border-radius: 2px }`
            };
        }
        if !mark_body && self.node_representation_class(key).contains("glyph") {
            return half;
        }
        match shape {
            NodeShape::Square => 0.0,
            NodeShape::RoundedSquare => 9.0_f32.min(half),
            NodeShape::Circle => half,
            // Clipped to their outline, so no rounding.
            NodeShape::Diamond | NodeShape::Hexagon => 0.0,
        }
    }
    /// Replace this view's rule-chosen detail rungs atomically. No geometry/physics edits.
    pub fn set_node_rule_representations(
        &mut self,
        epoch: u64,
        rungs: HashMap<uuid::Uuid, sceno::Representation>,
    ) -> Result<bool, String> {
        self.set_node_rule_presentation(epoch, rungs, HashMap::new())
    }
    pub fn node_rule_representation(&self, member: uuid::Uuid) -> Option<&sceno::Representation> {
        self.node_rule_representations
            .get(&member)
            .or_else(|| self.node_rule_marks.get(&member).map(|mark| &mark.detail))
    }
    pub(crate) fn node_representation_class(&self, key: NodeKey) -> &'static str {
        let requested = self
            .graph
            .get_node(key)
            .and_then(|node| self.node_rule_representations.get(&node.id))
            .or_else(|| {
                self.member_of(key)
                    .and_then(|id| self.node_rule_marks.get(&id))
                    .map(|mark| &mark.detail)
            })
            .or_else(|| self.projection_representation(key));
        match requested {
            Some(sceno::Representation::Glyph) => " gnode-representation-glyph",
            Some(sceno::Representation::LivePane) => " gnode-representation-live-pane",
            _ => " gnode-representation-card",
        }
    }
}
#[cfg(test)]
#[path = "node_presentation_tests.rs"]
mod tests;
