// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Rebind compiled recipe marks at the existing graph/canvas boundary.

use super::*;
use scenomise::projection::CompiledProjection;

impl Canvas {
    /// Adopt the marks and score of a compiled recipe. Source references use
    /// the same rebinding and first-occurrence policy as set_projection_score.
    /// Unknown adapters and absent graph members retain their host treatment.
    /// Invalid mark data leaves the current projection intact.
    pub fn set_compiled_projection(
        &mut self,
        projection: &CompiledProjection,
    ) -> Result<(), String> {
        let mut marks = HashMap::new();
        for (index, item) in projection.score.items.iter().enumerate() {
            if item.source.adapter != crate::MERE_GRAPH_ADAPTER {
                continue;
            }
            let Some(key) = uuid::Uuid::parse_str(&item.source.id)
                .ok()
                .and_then(|id| self.graph.get_node_key_by_id(id))
            else {
                continue;
            };
            let resolved = projection
                .node_marks
                .get(&sceno::InstanceId(index as u32))
                .and_then(|resolved| resolved.marks.node.as_ref())
                .ok_or_else(|| format!("Missing compiled node mark at item {index}"))?;
            resolved.validate()?;
            marks.entry(key).or_insert_with(|| resolved.clone());
        }
        self.set_projection_score(Some(projection.score.clone()));
        self.projection_marks = marks;
        self.refresh_projection_bodies();
        Ok(())
    }

    /// The recipe or rule's mark for this graph node, before local overrides.
    pub fn projection_mark(&self, key: NodeKey) -> Option<&scenograph::presentation::NodeMark> {
        self.projection_marks.get(&key)
    }

    pub(crate) fn requested_node_mark(
        &self,
        key: NodeKey,
    ) -> Option<&scenograph::presentation::NodeMark> {
        self.member_of(key)
            .and_then(|id| self.node_rule_marks.get(&id))
            .or_else(|| self.projection_marks.get(&key))
    }

    /// Whether the live canvas realizes the complete mark requested by a rule.
    pub fn supports_node_rule_mark(mark: &scenograph::presentation::NodeMark) -> bool {
        mark.validate().is_ok()
            && matches!(
                mark.detail,
                sceno::Representation::Glyph | sceno::Representation::Card
            )
    }

    /// Replace this view's rule effects as one validated transaction.
    /// A whole mark and a detail-only effect compete for the same node slot.
    pub fn set_node_rule_presentation(
        &mut self,
        epoch: u64,
        rungs: HashMap<uuid::Uuid, sceno::Representation>,
        marks: HashMap<uuid::Uuid, scenograph::presentation::NodeMark>,
    ) -> Result<bool, String> {
        if epoch != self.graph_view_epoch {
            return Err("Node representation view was replaced".into());
        }
        for (member, rung) in &rungs {
            if self.graph.get_node_by_id(*member).is_none() {
                return Err(format!("Access {member} is absent"));
            }
            if !matches!(
                rung,
                sceno::Representation::Glyph | sceno::Representation::Card
            ) {
                return Err("Only Glyph and Card rule representations are supported".into());
            }
        }
        let extent = self
            .layout_strategy()
            .and_then(|id| {
                scenomise::catalog::Family::resolve(id.strip_suffix(".default").unwrap_or(id))
            })
            .map_or(
                scenograph::presentation::Extent::Measured,
                scenomise::catalog::Family::extent,
            );
        for (member, mark) in &marks {
            if self.graph.get_node_by_id(*member).is_none() {
                return Err(format!("Access {member} is absent"));
            }
            if rungs.contains_key(member)
                || !Self::supports_node_rule_mark(mark)
                || !mark.accepts(extent)
            {
                return Err(format!("Unsupported or conflicting node mark for {member}"));
            }
        }
        if self.node_rule_representations == rungs && self.node_rule_marks == marks {
            return Ok(false);
        }
        let changed_bodies = self.node_rule_marks != marks;
        self.node_rule_representations = rungs;
        self.node_rule_marks = marks;
        if changed_bodies {
            self.refresh_projection_bodies();
        }
        Ok(true)
    }

    pub(super) fn refresh_projection_bodies(&mut self) {
        self.push_node_geometry();
        let materials = self
            .graph
            .nodes()
            .map(|(key, _)| (key, self.node_material(key)))
            .collect();
        self.physics.set_node_materials(materials);
    }
}

#[cfg(test)]
#[path = "projection_marks_tests.rs"]
mod tests;
