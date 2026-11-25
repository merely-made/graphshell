// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Structural admission for a scene's product-free presentation sidecar.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use sceno::InstanceId;

use crate::{
    PresentationBinding, PresentationCapability, PresentationChange, PresentationCodec,
    PresentationManifest, SceneSnapshot,
};

/// A presentation table does not describe bindings or offers the scene can use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentationValidationError {
    AbsentOrTombstonedItem(InstanceId),
    DuplicateBinding(InstanceId),
    CodecCapabilityMismatch,
}

impl fmt::Display for PresentationValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AbsentOrTombstonedItem(instance) => write!(
                formatter,
                "binding for absent or tombstoned item {}",
                instance.0
            ),
            Self::DuplicateBinding(instance) => {
                write!(formatter, "duplicate binding for item {}", instance.0)
            },
            Self::CodecCapabilityMismatch => {
                formatter.write_str("presentation codec names the wrong required capability")
            },
        }
    }
}

impl std::error::Error for PresentationValidationError {}

/// Validate the structural relationship between a scene and its presentation
/// metadata. Session-local retention and resource availability stay with the
/// caller that owns those policies. Callers validate the scene tables separately.
pub fn validate_presentation(
    scene: &SceneSnapshot,
    presentation: &PresentationManifest,
) -> Result<(), PresentationValidationError> {
    let mut instances = BTreeSet::new();
    for binding in &presentation.bindings {
        if scene.active_item(binding.instance).is_none() {
            return Err(PresentationValidationError::AbsentOrTombstonedItem(
                binding.instance,
            ));
        }
        if !instances.insert(binding.instance.0) {
            return Err(PresentationValidationError::DuplicateBinding(
                binding.instance,
            ));
        }
    }
    for offer in presentation.offers.values().flatten() {
        let expected = match offer.codec {
            PresentationCodec::NativeGlyphV1 => PresentationCapability::NativeGlyph,
            PresentationCodec::PortableCardV1 => PresentationCapability::PortableCard,
            PresentationCodec::ImageV1 { .. } => PresentationCapability::Image,
            PresentationCodec::EditableTextV1 => PresentationCapability::EditableText,
        };
        if offer.requires != expected {
            return Err(PresentationValidationError::CodecCapabilityMismatch);
        }
    }
    Ok(())
}

impl PresentationManifest {
    /// The changes that take this manifest to `next`, bindings by instance
    /// and offers by key, with every resource `next` no longer advertises
    /// invalidated, as a fresh keyframe would leave the cache (engine binding
    /// E3).
    pub fn changes_to(&self, next: &PresentationManifest) -> Vec<PresentationChange> {
        let held: BTreeMap<_, _> = self.bindings.iter().map(|b| (b.instance.0, &b.key)).collect();
        let wanted: BTreeMap<_, _> = next.bindings.iter().map(|b| (b.instance.0, &b.key)).collect();
        let mut changes = Vec::new();
        for instance in held.keys().filter(|instance| !wanted.contains_key(instance)) {
            changes.push(PresentationChange::Unbind {
                instance: InstanceId(*instance),
            });
        }
        for binding in &next.bindings {
            if held.get(&binding.instance.0) != Some(&&binding.key) {
                changes.push(PresentationChange::Bind(binding.clone()));
            }
        }
        for key in self.offers.keys().filter(|key| !next.offers.contains_key(*key)) {
            changes.push(PresentationChange::RemoveOffers { key: key.clone() });
        }
        for (key, offers) in &next.offers {
            if self.offers.get(key) != Some(offers) {
                changes.push(PresentationChange::ReplaceOffers {
                    key: key.clone(),
                    offers: offers.clone(),
                });
            }
        }
        let advertised = |manifest: &PresentationManifest| {
            manifest
                .offers
                .values()
                .flatten()
                .map(|offer| offer.resource)
                .collect::<BTreeSet<_>>()
        };
        let kept = advertised(next);
        for resource in advertised(self).difference(&kept) {
            changes.push(PresentationChange::InvalidateResource {
                resource: *resource,
            });
        }
        changes
    }

    /// Apply a diff's changes in order, in one pass over the bindings rather
    /// than one per change: a bind replaces the instance's binding and moves
    /// it to the end, an unbind removes it. A resource invalidation names the
    /// cache, not the manifest, and leaves it as it was.
    pub fn apply_changes(&mut self, changes: &[PresentationChange]) {
        // Each touched instance's last word, and when it was said.
        let mut touched: BTreeMap<u32, Option<(usize, &PresentationBinding)>> = BTreeMap::new();
        for (at, change) in changes.iter().enumerate() {
            match change {
                PresentationChange::Bind(binding) => {
                    touched.insert(binding.instance.0, Some((at, binding)));
                },
                PresentationChange::Unbind { instance } => {
                    touched.insert(instance.0, None);
                },
                PresentationChange::ReplaceOffers { key, offers } => {
                    self.offers.insert(key.clone(), offers.clone());
                },
                PresentationChange::RemoveOffers { key } => {
                    self.offers.remove(key);
                },
                PresentationChange::InvalidateResource { .. } => {},
            }
        }
        if touched.is_empty() {
            return;
        }
        self.bindings
            .retain(|binding| !touched.contains_key(&binding.instance.0));
        let mut bound: Vec<_> = touched.into_values().flatten().collect();
        bound.sort_by_key(|(at, _)| *at);
        self.bindings
            .extend(bound.into_iter().map(|(_, binding)| binding.clone()));
    }
}
