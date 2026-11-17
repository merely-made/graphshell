// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Recipe defaults for the three graph primitives (primitive representations M2).

use crate::presentation::{
    Context, Effect, InputAvailability, Issue, NodeMark, RelationForm, Resolution, RuleSet,
};
use serde::{Deserialize, Serialize};

/// How a set or field is shown. The realization owns drawing these forms.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldForm {
    Contour,
    Backdrop,
    MemberTint,
    Guide,
}

/// Missing slots retain the host's existing defaults.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MarkSlots {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node: Option<NodeMark>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relation: Option<RelationForm>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<FieldForm>,
}

impl MarkSlots {
    pub fn is_empty(&self) -> bool {
        self.node.is_none() && self.relation.is_none() && self.field.is_none()
    }

    pub(crate) fn validation_issues(&self) -> Vec<crate::ValidationIssue> {
        self.node
            .as_ref()
            .and_then(|mark| mark.validate().err())
            .map(|message| crate::ValidationIssue::error("marks.node", &message))
            .into_iter()
            .collect()
    }
}

/// Effective forms and the defaults and rule trace that explain them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResolvedMarks {
    pub defaults: MarkSlots,
    pub marks: MarkSlots,
    pub resolution: Resolution,
}

impl RuleSet {
    /// Resolve one occurrence with host-disclosed inputs over recipe defaults.
    /// Whole node marks and legacy detail effects compete for the same property.
    pub fn resolve_marks(
        &self,
        defaults: &MarkSlots,
        context: &Context,
        availability: &InputAvailability,
    ) -> Result<ResolvedMarks, Vec<Issue>> {
        if let Some(node) = &defaults.node {
            node.validate().map_err(|message| {
                vec![Issue {
                    field: "marks.node".into(),
                    message,
                }]
            })?;
        }
        let resolution = self.resolve_with_availability(context, availability)?;
        let mut marks = defaults.clone();
        for decision in resolution.winners.values() {
            match &decision.effect {
                Effect::NodeMark { value } => marks.node = Some(value.clone()),
                Effect::Representation { value } => {
                    let mark = marks.node.as_mut().ok_or_else(|| {
                        vec![Issue {
                            field: "marks.node".into(),
                            message: "a detail rule needs a default node mark".into(),
                        }]
                    })?;
                    mark.detail = value.clone();
                },
                Effect::RelationForm { value } => marks.relation = Some(*value),
                Effect::FieldForm { value } => marks.field = Some(*value),
                _ => {},
            }
        }
        Ok(ResolvedMarks {
            defaults: defaults.clone(),
            marks,
            resolution,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presentation::{Body, Condition, Extent, Material, PrimitiveKind, Rule, Silhouette};

    #[test]
    fn relation_and_field_rules_resolve_over_recipe_defaults() {
        let defaults = MarkSlots {
            relation: Some(RelationForm::Stroke),
            field: Some(FieldForm::Guide),
            ..Default::default()
        };
        for (kind, effect) in [
            (
                PrimitiveKind::Relation,
                Effect::RelationForm {
                    value: RelationForm::Containment,
                },
            ),
            (
                PrimitiveKind::Field,
                Effect::FieldForm {
                    value: FieldForm::Backdrop,
                },
            ),
        ] {
            let rules = RuleSet {
                version: 1,
                rules: vec![Rule {
                    id: "change".into(),
                    label: "Change".into(),
                    enabled: true,
                    priority: 0,
                    target: crate::presentation::Target {
                        kind,
                        ids: Default::default(),
                    },
                    condition: Condition::Always,
                    effects: vec![effect],
                }],
            };
            let context = Context {
                id: "one".into(),
                kind,
                facts: Default::default(),
                states: Default::default(),
                screen_size: None,
                previous_matches: Default::default(),
            };
            let result = rules
                .resolve_marks(&defaults, &context, &InputAvailability::default())
                .unwrap();
            assert_eq!(result.defaults, defaults);
            assert!(!result.resolution.traces[0].winning_properties.is_empty());
            if kind == PrimitiveKind::Field {
                assert_eq!(result.marks.field, Some(FieldForm::Backdrop));
                assert_eq!(result.marks.relation, defaults.relation);
            } else {
                assert_eq!(result.marks.relation, Some(RelationForm::Containment));
                assert_eq!(result.marks.field, defaults.field);
            }
        }
    }

    #[test]
    fn authored_and_bound_marks_survive_promotion_json_and_toml() {
        let mut authored = crate::tests::authored();
        let mut node = NodeMark::card();
        node.body = Body::Silhouette(Silhouette::Diamond);
        node.material = Some(Material {
            restitution: 0.2,
            friction: 0.4,
            density: 1.0,
        });
        authored.marks = MarkSlots {
            node: Some(node),
            relation: Some(RelationForm::Stroke),
            field: Some(FieldForm::Guide),
        };
        let key = authored.sources.keys().next().unwrap().clone();
        let bound = authored.bind(&key, None).unwrap();
        assert_eq!(bound.marks, authored.marks);
        let carried: crate::AuthoredProjectionDefinition =
            serde_json::from_slice(&serde_json::to_vec(&authored).unwrap()).unwrap();
        assert_eq!(carried, authored);
        #[cfg(feature = "toml")]
        {
            let text = authored.to_toml().unwrap();
            assert_eq!(
                crate::AuthoredProjectionDefinition::from_toml(&text).unwrap(),
                authored
            );
            assert_eq!(
                crate::ProjectionDefinition::from_toml(&bound.to_toml().unwrap()).unwrap(),
                bound
            );
        }
        let mut invalid = crate::tests::valid_draft();
        invalid.marks = bound.marks;
        invalid
            .marks
            .node
            .as_mut()
            .unwrap()
            .material
            .as_mut()
            .unwrap()
            .density = f32::NAN;
        assert!(
            invalid
                .validate()
                .unwrap_err()
                .iter()
                .any(|issue| issue.field == "marks.node")
        );
        let mut invalid_mark = NodeMark::card();
        invalid_mark.accepts.clear();
        assert!(invalid_mark.validate().is_err());
        assert!(
            NodeMark::new(sceno::Representation::Glyph, [Extent::Assigned])
                .validate()
                .is_ok()
        );
    }
}
