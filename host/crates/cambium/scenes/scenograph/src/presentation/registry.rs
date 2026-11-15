// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The representation registry: a node's class picks a profile, which names
//! its mark's detail ladder and body plus the host gestures it binds.
//!
//! A graph class does not prescribe application truth or execute a script.
//! Endpoint-authored domain actions remain Graphshell presentation offers;
//! these bindings cover graph-host mechanics such as inspect, follow,
//! recenter, and pin. Paint is the host's per node; the extent a mark takes is
//! imposed by the arrangement.

use sceno::Representation;
use serde::{Deserialize, Serialize};

use super::ladder::{RepresentationLadder, RepresentationState, default_representation_ladder, non_document_ladder};
use super::mark::{Body, Extent, NodeMark, Paint, Silhouette};

/// Stable schema for the built-in graph representation registry.
pub const GRAPH_REPRESENTATION_REGISTRY_SCHEMA: &str = "mere.graph-representation-registry/v2";

/// One named primitive a renderer can realize in its own paint system.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrimitiveProfile {
    pub id: String,
    pub label: String,
    /// The body and hard collider use the same geometry.
    pub body: Silhouette,
}

/// A host-level gesture. This is a binding surface, not executable script.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostGesture {
    Select,
    Activate,
    Drag,
    Follow,
    Compare,
}

/// Graph-host behavior that survives renderer replacement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostBehavior {
    Inspect,
    OpenPresentation,
    Pin,
    RecenterNeighborhood,
    FollowRelations,
    CompareReplacement,
}

/// Connects one gesture to one host behavior.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BehaviorBinding {
    pub gesture: HostGesture,
    pub behavior: HostBehavior,
}

/// Presentation profile selected by one or more graph class labels.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RepresentationProfile {
    pub id: String,
    pub classes: Vec<String>,
    pub primitive: PrimitiveProfile,
    pub behaviors: Vec<BehaviorBinding>,
    /// Host-side policy for choosing the `sceno` representation slot.
    pub ladder: RepresentationLadder,
}

/// Ordered serialized registry. Unknown classes resolve to `fallback`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GraphRepresentationRegistry {
    pub schema: String,
    pub profiles: Vec<RepresentationProfile>,
    pub fallback: RepresentationProfile,
}

impl GraphRepresentationRegistry {
    /// Resolve a graph class without promoting the class into presentation truth.
    pub fn resolve(&self, class: &str) -> &RepresentationProfile {
        self.profiles
            .iter()
            .find(|profile| profile.classes.iter().any(|candidate| candidate == class))
            .unwrap_or(&self.fallback)
    }

    /// Resolve a set of class labels in registry order. This makes selection
    /// deterministic even when the source stores its labels in a hash set.
    pub fn resolve_classes<'a>(
        &self,
        classes: impl IntoIterator<Item = &'a str>,
    ) -> &RepresentationProfile {
        let classes: Vec<&str> = classes.into_iter().collect();
        self.profiles
            .iter()
            .find(|profile| {
                profile
                    .classes
                    .iter()
                    .any(|candidate| classes.contains(&candidate.as_str()))
            })
            .unwrap_or(&self.fallback)
    }

    /// The mark for a node of these classes, its rung chosen from the host's
    /// facts (D30).
    pub fn mark<'a>(
        &self,
        classes: impl IntoIterator<Item = &'a str>,
        state: RepresentationState,
        previous: Option<&Representation>,
    ) -> NodeMark {
        self.resolve_classes(classes).mark(state, previous)
    }
}

/// The extents a registry mark accepts: measured on a canvas, or filling a
/// cell an arrangement assigns.
const PROFILE_EXTENTS: [Extent; 2] = [Extent::Measured, Extent::Assigned];

impl RepresentationProfile {
    /// This profile's mark at the rung its ladder picks.
    pub fn mark(&self, state: RepresentationState, previous: Option<&Representation>) -> NodeMark {
        NodeMark {
            material: None,
            detail: self.ladder.select(state, previous),
            paint: Paint::default(),
            body: Body::Silhouette(self.primitive.body),
            accepts: PROFILE_EXTENTS.into(),
        }
    }
}

/// Classes whose data is a document, and so keep the Card rung (D11, D30).
const DOCUMENT_PROFILE: &str = "document";

/// Mere's renderer-neutral baseline registry.
pub fn default_graph_representation_registry() -> GraphRepresentationRegistry {
    let inspect = BehaviorBinding {
        gesture: HostGesture::Select,
        behavior: HostBehavior::Inspect,
    };
    let pin = BehaviorBinding {
        gesture: HostGesture::Drag,
        behavior: HostBehavior::Pin,
    };
    let follow = BehaviorBinding {
        gesture: HostGesture::Follow,
        behavior: HostBehavior::FollowRelations,
    };
    let open = BehaviorBinding {
        gesture: HostGesture::Activate,
        behavior: HostBehavior::OpenPresentation,
    };

    GraphRepresentationRegistry {
        schema: GRAPH_REPRESENTATION_REGISTRY_SCHEMA.to_owned(),
        profiles: vec![
            profile(
                "software",
                &["product", "platform", "software", "foundation"],
                "hexagonal-software",
                "hexagonal software body + collider",
                Silhouette::Hexagon,
                &[inspect, open, pin, follow],
            ),
            profile(
                "device",
                &["device", "tool"],
                "rounded-device",
                "rounded device body + collider",
                Silhouette::RoundedSquare,
                &[inspect, pin, follow],
            ),
            profile(
                "document",
                &["document", "page", "note"],
                "square-document",
                "square document face + collider",
                Silhouette::Square,
                &[inspect, open, pin, follow],
            ),
            profile(
                "event",
                &["event"],
                "diamond-event",
                "diamond event hull + collider",
                Silhouette::Diamond,
                &[inspect, pin, follow],
            ),
            profile(
                "actor",
                &["place", "person", "community"],
                "circular-actor",
                "circular actor + collider",
                Silhouette::Circle,
                &[inspect, pin, follow],
            ),
        ],
        fallback: profile(
            "unknown",
            &[],
            "circular-unknown",
            "circular unknown body + collider",
            Silhouette::Circle,
            &[inspect, pin, follow],
        ),
    }
}

fn profile(
    id: &str,
    classes: &[&str],
    primitive_id: &str,
    primitive_label: &str,
    body: Silhouette,
    behaviors: &[BehaviorBinding],
) -> RepresentationProfile {
    RepresentationProfile {
        id: id.to_owned(),
        classes: classes.iter().map(|class| (*class).to_owned()).collect(),
        primitive: PrimitiveProfile {
            id: primitive_id.to_owned(),
            label: primitive_label.to_owned(),
            body,
        },
        behaviors: behaviors.to_vec(),
        ladder: if id == DOCUMENT_PROFILE {
            default_representation_ladder()
        } else {
            non_document_ladder()
        },
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn every_class_has_one_profile_and_unknown_classes_fall_back() {
        let registry = default_graph_representation_registry();
        let mut classes = BTreeSet::new();
        for profile in &registry.profiles {
            for class in &profile.classes {
                assert!(classes.insert(class), "class {class} appears twice");
            }
        }
        assert_eq!(
            registry.resolve("document").primitive.body,
            Silhouette::Square
        );
        assert_eq!(
            registry.resolve("future-class").primitive.body,
            Silhouette::Circle
        );
    }

    #[test]
    fn primitive_and_behavior_axes_are_serializable_and_distinct() {
        let registry = default_graph_representation_registry();
        let encoded = serde_json::to_value(&registry).expect("registry serializes");
        assert_eq!(encoded["schema"], GRAPH_REPRESENTATION_REGISTRY_SCHEMA);
        assert_eq!(encoded["profiles"][0]["primitive"]["body"], "hexagon");
        assert_eq!(
            encoded["profiles"][0]["behaviors"][0]["behavior"],
            "inspect"
        );
        assert_eq!(
            encoded["profiles"][2]["ladder"]["rungs"][1]["conditions"][2]["measure"],
            "zoom_level"
        );
        assert_eq!(
            serde_json::from_value::<GraphRepresentationRegistry>(encoded)
                .expect("registry deserializes"),
            registry
        );
    }

    #[test]
    fn host_bindings_do_not_claim_endpoint_domain_actions() {
        let registry = default_graph_representation_registry();
        let encoded = serde_json::to_string(&registry).expect("registry serializes");
        for forbidden in ["domain_truth", "external_effect", "payload_schema"] {
            assert!(!encoded.contains(forbidden));
        }
    }

    #[test]
    fn only_document_classes_resolve_to_a_card() {
        let registry = default_graph_representation_registry();
        let near = RepresentationState {
            screen_width: 128.0,
            screen_height: 128.0,
            zoom_level: 2.0,
            recency: 1.0,
            focused: false,
        };
        let document = registry.mark(["note"], near, None);
        assert_eq!(document.detail, Representation::Card);
        assert_eq!(document.body, Body::Silhouette(Silhouette::Square));
        for classes in [&["event"][..], &["person"], &["software"], &["future-class"], &[]] {
            let mark = registry.mark(classes.iter().copied(), near, None);
            assert_eq!(mark.detail, Representation::Glyph, "{classes:?}");
            assert!(mark.accepts(Extent::Measured) && mark.accepts(Extent::Assigned));
        }
        assert_eq!(
            registry.mark(["event"], near, None).body,
            Body::Silhouette(Silhouette::Diamond)
        );
    }

    #[test]
    fn class_set_resolution_follows_registry_order() {
        let registry = default_graph_representation_registry();
        let profile = registry.resolve_classes(["device", "document"]);
        assert_eq!(profile.id, "device");
    }
}
