// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! A node's mark: its detail rung, paint, body and the extents it accepts
//! (primitive representations plan, M1).

use std::collections::BTreeSet;

use sceno::Representation;
use serde::{Deserialize, Serialize};

/// Who decides an item's size. A mark declares the ones it accepts; an
/// arrangement imposes one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Extent {
    /// The mark sizes itself and the arrangement clears it.
    Measured,
    /// The arrangement hands the mark a rectangle to fill.
    Assigned,
    /// Data sets a dimension.
    Encoded,
}

impl Extent {
    pub fn id(self) -> &'static str {
        match self {
            Self::Measured => "measured",
            Self::Assigned => "assigned",
            Self::Encoded => "encoded",
        }
    }
}

/// What is painted on a mark. Recognized core plus an open tail a host
/// recognizes by name.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Paint {
    /// The fetched favicon, with the caption beside it.
    #[default]
    Favicon,
    /// A vector face derived from the node's address, in palette slots.
    Derived,
    /// An image drawn for the subject.
    Sprite,
    /// The bare state color, nothing painted on it.
    Bare,
    Open {
        kind: String,
    },
}

impl Paint {
    /// The persisted code. The four core codes are the canvas sidecar's.
    pub fn as_code(&self) -> &str {
        match self {
            Self::Favicon => "favicon",
            Self::Derived => "derived",
            Self::Sprite => "sprite",
            Self::Bare => "bare",
            Self::Open { kind } => kind,
        }
    }

    /// Parse a persisted code; an unknown code reads as [`Paint::Favicon`],
    /// as the sidecar always has.
    pub fn from_code(code: &str) -> Self {
        match code {
            "derived" => Self::Derived,
            "sprite" => Self::Sprite,
            "bare" => Self::Bare,
            _ => Self::Favicon,
        }
    }
}

/// One silhouette list for bodies and their colliders.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Silhouette {
    #[default]
    Square,
    RoundedSquare,
    Circle,
    Diamond,
    Hexagon,
}

impl Silhouette {
    /// The outline of a polygonal silhouette in a unit box centred on the
    /// origin, so a collider and a style clip share one shape. `None` for the
    /// square, rounded square and circle, which draw by corner radius.
    pub fn outline(self) -> Option<&'static [(f32, f32)]> {
        match self {
            Self::Diamond => Some(&[(0.0, -0.5), (0.5, 0.0), (0.0, 0.5), (-0.5, 0.0)]),
            Self::Hexagon => Some(&[
                (0.0, -0.5),
                (0.5, -0.25),
                (0.5, 0.25),
                (0.0, 0.5),
                (-0.5, 0.25),
                (-0.5, -0.25),
            ]),
            Self::Square | Self::RoundedSquare | Self::Circle => None,
        }
    }
}

/// A mark's part in physics. An authored hull is per node, kept by the host.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Body {
    #[default]
    None,
    Silhouette(Silhouette),
    /// The node's own authored hull.
    Hull,
}

/// How a node is drawn, described once.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeMark {
    pub detail: Representation,
    #[serde(default)]
    pub paint: Paint,
    #[serde(default)]
    pub body: Body,
    pub accepts: BTreeSet<Extent>,
}

impl NodeMark {
    pub fn new(detail: Representation, accepts: impl IntoIterator<Item = Extent>) -> Self {
        Self {
            detail,
            paint: Paint::default(),
            body: Body::default(),
            accepts: accepts.into_iter().collect(),
        }
    }

    /// A card: measured on a canvas, or filling a cell it is given.
    pub fn card() -> Self {
        Self::new(Representation::Card, [Extent::Measured, Extent::Assigned])
    }

    pub fn accepts(&self, extent: Extent) -> bool {
        self.accepts.contains(&extent)
    }

    /// The mark's name for a refusal: its rung.
    pub fn label(&self) -> String {
        match &self.detail {
            Representation::Open { kind } => kind.clone(),
            other => format!("{other:?}").to_lowercase(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paint_codes_are_the_sidecars_byte_for_byte() {
        for (paint, code) in [
            (Paint::Favicon, "favicon"),
            (Paint::Derived, "derived"),
            (Paint::Sprite, "sprite"),
            (Paint::Bare, "bare"),
        ] {
            assert_eq!(paint.as_code(), code);
            assert_eq!(Paint::from_code(code), paint);
        }
        assert_eq!(Paint::from_code("nonsense"), Paint::Favicon);
    }

    #[test]
    fn a_mark_without_paint_or_body_reads_with_defaults() {
        let mark: NodeMark =
            serde_json::from_str(r#"{"detail":"Glyph","accepts":["measured"]}"#).unwrap();
        assert_eq!(mark.paint, Paint::Favicon);
        assert_eq!(mark.body, Body::None);
        assert!(mark.accepts(Extent::Measured));
        assert!(!mark.accepts(Extent::Assigned));
    }

    #[test]
    fn a_mark_round_trips() {
        let mark = NodeMark {
            detail: Representation::Card,
            paint: Paint::Open {
                kind: "meter".into(),
            },
            body: Body::Silhouette(Silhouette::Hexagon),
            accepts: [Extent::Measured, Extent::Encoded].into(),
        };
        let json = serde_json::to_string(&mark).unwrap();
        assert_eq!(serde_json::from_str::<NodeMark>(&json).unwrap(), mark);
        assert_eq!(
            serde_json::to_value(Body::Silhouette(Silhouette::RoundedSquare)).unwrap(),
            serde_json::json!({ "silhouette": "rounded_square" })
        );
    }

    #[test]
    fn polygonal_silhouettes_fit_the_unit_box() {
        for shape in [Silhouette::Diamond, Silhouette::Hexagon] {
            let outline = shape.outline().unwrap();
            assert!(outline.len() >= 4);
            assert!(outline.iter().all(|(x, y)| x.abs() <= 0.5 && y.abs() <= 0.5));
        }
        assert!(Silhouette::Circle.outline().is_none());
    }

    #[test]
    fn a_card_accepts_a_measured_or_assigned_extent() {
        let card = NodeMark::card();
        assert!(card.accepts(Extent::Measured) && card.accepts(Extent::Assigned));
        assert!(!card.accepts(Extent::Encoded));
        assert_eq!(card.label(), "card");
    }
}
