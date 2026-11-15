// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The representation registry, which lives in scenograph beside the node
//! mark (D28, G1); re-exported so this crate's callers do not change.

pub use scenograph::presentation::{
    BehaviorBinding, ConditionOperation, GRAPH_REPRESENTATION_REGISTRY_SCHEMA,
    GraphRepresentationRegistry, HostBehavior, HostGesture, PrimitiveProfile,
    RepresentationCondition, RepresentationLadder, RepresentationMeasure, RepresentationProfile,
    RepresentationRung, RepresentationState, default_graph_representation_registry,
    default_representation_ladder, non_document_ladder,
};
/// The merged silhouette list (D33), under the registry's older name.
pub use scenograph::presentation::Silhouette as PrimitiveBody;
