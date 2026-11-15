// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The detail ladder: which rung of a mark a host draws, from its own facts.

use sceno::Representation;
use serde::{Deserialize, Serialize};

/// A host fact that can select one rung of a representation ladder.
///
/// Screen measures describe the realized item, in pixels. `ZoomLevel` and
/// `Recency` are host-side view facts; recency is normalized to `0..=1`, newest
/// first. `Focused` reads as `1.0` or `0.0`, which keeps every condition in one
/// small comparison grammar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepresentationMeasure {
    ScreenWidth,
    ScreenHeight,
    ZoomLevel,
    Recency,
    Focused,
}

/// Numeric comparisons understood by representation conditions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionOperation {
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
}

/// One declarative test for entering or retaining a representation rung.
///
/// `hysteresis` relaxes the threshold only while this condition's rung is
/// already selected. A card selected by `zoom_level >= 1.0` with `0.1`
/// hysteresis therefore remains a card down to zoom `0.9`, but a glyph still
/// has to reach `1.0` before it becomes a card.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RepresentationCondition {
    pub measure: RepresentationMeasure,
    pub operation: ConditionOperation,
    pub threshold: f32,
    pub hysteresis: f32,
}

/// The host facts evaluated by a representation ladder for one item.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RepresentationState {
    pub screen_width: f32,
    pub screen_height: f32,
    pub zoom_level: f32,
    pub recency: f32,
    pub focused: bool,
}

impl RepresentationState {
    fn measure(self, measure: RepresentationMeasure) -> f32 {
        match measure {
            RepresentationMeasure::ScreenWidth => self.screen_width,
            RepresentationMeasure::ScreenHeight => self.screen_height,
            RepresentationMeasure::ZoomLevel => self.zoom_level,
            RepresentationMeasure::Recency => self.recency,
            RepresentationMeasure::Focused => {
                if self.focused {
                    1.0
                } else {
                    0.0
                }
            },
        }
    }
}

impl RepresentationCondition {
    /// Evaluate this condition. Invalid numeric input never selects a rung.
    pub fn matches(self, state: RepresentationState, retaining: bool) -> bool {
        let value = state.measure(self.measure);
        if !value.is_finite() || !self.threshold.is_finite() || !self.hysteresis.is_finite() {
            return false;
        }
        let padding = if retaining {
            self.hysteresis.max(0.0)
        } else {
            0.0
        };
        match self.operation {
            ConditionOperation::LessThan => value < self.threshold + padding,
            ConditionOperation::LessThanOrEqual => value <= self.threshold + padding,
            ConditionOperation::GreaterThan => value > self.threshold - padding,
            ConditionOperation::GreaterThanOrEqual => value >= self.threshold - padding,
        }
    }
}

/// One target representation and the conditions that all must hold to use it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RepresentationRung {
    pub representation: Representation,
    pub conditions: Vec<RepresentationCondition>,
}

/// Ordered representation policy. The first matching rung wins.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RepresentationLadder {
    pub rungs: Vec<RepresentationRung>,
    pub fallback: Representation,
}

impl RepresentationLadder {
    /// Select a rung from host facts, retaining `previous` through its declared
    /// hysteresis band when possible.
    pub fn select(
        &self,
        state: RepresentationState,
        previous: Option<&Representation>,
    ) -> Representation {
        self.rungs
            .iter()
            .find(|rung| {
                let retaining = previous == Some(&rung.representation);
                !rung.conditions.is_empty()
                    && rung
                        .conditions
                        .iter()
                        .all(|condition| condition.matches(state, retaining))
            })
            .map(|rung| rung.representation.clone())
            .unwrap_or_else(|| self.fallback.clone())
    }
}

/// Mere's first host-side LOD ladder: focused content stays live; recent
/// content earns a card at close zoom; everything else is a glyph.
pub fn default_representation_ladder() -> RepresentationLadder {
    RepresentationLadder {
        rungs: vec![
            RepresentationRung {
                representation: Representation::LivePane,
                conditions: vec![RepresentationCondition {
                    measure: RepresentationMeasure::Focused,
                    operation: ConditionOperation::GreaterThanOrEqual,
                    threshold: 1.0,
                    hysteresis: 0.0,
                }],
            },
            RepresentationRung {
                representation: Representation::Card,
                conditions: vec![
                    RepresentationCondition {
                        measure: RepresentationMeasure::ScreenWidth,
                        operation: ConditionOperation::GreaterThan,
                        threshold: 0.0,
                        hysteresis: 0.0,
                    },
                    RepresentationCondition {
                        measure: RepresentationMeasure::Recency,
                        operation: ConditionOperation::GreaterThanOrEqual,
                        threshold: 0.5,
                        hysteresis: 0.0,
                    },
                    RepresentationCondition {
                        measure: RepresentationMeasure::ZoomLevel,
                        operation: ConditionOperation::GreaterThanOrEqual,
                        threshold: 1.0,
                        hysteresis: 0.1,
                    },
                ],
            },
        ],
        fallback: Representation::Glyph,
    }
}

/// The ladder for data that is not a document (D11, D30): the same ladder
/// without its Card rung.
pub fn non_document_ladder() -> RepresentationLadder {
    let mut ladder = default_representation_ladder();
    ladder
        .rungs
        .retain(|rung| rung.representation != Representation::Card);
    ladder
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_ladder_selects_from_data_and_retains_through_hysteresis() {
        let ladder = default_representation_ladder();
        let state = |zoom_level, focused| RepresentationState {
            screen_width: 64.0 * zoom_level,
            screen_height: 64.0 * zoom_level,
            zoom_level,
            recency: 1.0,
            focused,
        };

        assert_eq!(ladder.select(state(1.0, false), None), Representation::Card);
        assert_eq!(
            ladder.select(state(0.95, false), Some(&Representation::Card)),
            Representation::Card
        );
        assert_eq!(
            ladder.select(state(0.89, false), Some(&Representation::Card)),
            Representation::Glyph
        );
        assert_eq!(
            ladder.select(state(0.95, false), Some(&Representation::Glyph)),
            Representation::Glyph
        );
        assert_eq!(
            ladder.select(state(0.4, true), Some(&Representation::Glyph)),
            Representation::LivePane
        );
    }

    #[test]
    fn data_that_is_not_a_document_never_reaches_the_card_rung() {
        let ladder = non_document_ladder();
        let state = |zoom_level, focused| RepresentationState {
            screen_width: 64.0 * zoom_level,
            screen_height: 64.0 * zoom_level,
            zoom_level,
            recency: 1.0,
            focused,
        };
        assert_eq!(ladder.select(state(2.0, false), None), Representation::Glyph);
        assert_eq!(ladder.select(state(0.4, true), None), Representation::LivePane);
    }
}
