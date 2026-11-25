// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Relation forms and the outcomes of arrangement constraints.

use crate::{InstanceId, Rect, RoutedRelation, Size2, Vec2};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationForm {
    #[default]
    Stroke,
    Containment,
    Adjacency,
    MatrixCell,
    Order,
}

impl RelationForm {
    pub fn is_stroke(&self) -> bool {
        *self == Self::Stroke
    }

    pub fn is_placed(self) -> bool {
        matches!(self, Self::Containment | Self::Adjacency | Self::Order)
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::Stroke => "stroke",
            Self::Containment => "containment",
            Self::Adjacency => "adjacency",
            Self::MatrixCell => "matrix_cell",
            Self::Order => "order",
        }
    }
}

/// A disclosed relation asking the arrangement to place two occurrences.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RelationConstraint {
    pub id: String,
    pub from: InstanceId,
    pub to: InstanceId,
    pub form: RelationForm,
}

/// Every placed request is reported, including requests incompatible with pins
/// or with other relations in the same reading.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RelationConstraintReport {
    pub constraint: RelationConstraint,
    pub honored: bool,
    pub reason: String,
}

impl RoutedRelation {
    /// The four corner points of a matrix cell bound a filled rectangle.
    pub fn cell_bounds(&self) -> Option<Rect> {
        if self.form != RelationForm::MatrixCell || self.points.len() != 4 {
            return None;
        }
        let first = self.points[0];
        let (mut low, mut high) = (first, first);
        for point in &self.points {
            if !point.x.is_finite() || !point.y.is_finite() {
                return None;
            }
            low.x = low.x.min(point.x);
            low.y = low.y.min(point.y);
            high.x = high.x.max(point.x);
            high.y = high.y.max(point.y);
        }
        let size = Size2::new(high.x - low.x, high.y - low.y);
        (size.w > 0.0 && size.h > 0.0).then(|| Rect::new(Vec2::new(low.x, low.y), size))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_scene_wire_defaults_to_strokes_and_omits_empty_form_metadata() {
        let old = serde_json::json!({
            "from": 0, "to": 1, "space": 0, "points": [],
            "kind": null, "weight": null
        });
        let relation: RoutedRelation = serde_json::from_value(old.clone()).unwrap();
        assert_eq!(relation.form, RelationForm::Stroke);
        assert_eq!(serde_json::to_value(relation).unwrap(), old);
        let scene = crate::Scene::new();
        let wire = serde_json::to_value(&scene).unwrap();
        assert!(wire.get("arrangement_order").is_none());
        assert!(wire.get("relation_constraints").is_none());
        assert_eq!(serde_json::from_value::<crate::Scene>(wire).unwrap(), scene);
    }
}
