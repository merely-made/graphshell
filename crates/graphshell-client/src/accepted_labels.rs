// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use chirograph::{
    CapabilityProfile, PresentationCapability, PresentationChange, PresentationKey,
    PresentationManifest, ProjectionDiff, ProjectionSession,
};
use sceno::InstanceId;
use scenotime::{Revision, SceneEpoch};

/// The labels affected by the latest accepted revision transaction. Consumers
/// can apply this only to a view of the same session and epoch at `base`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcceptedLabelUpdate {
    pub session: ProjectionSession,
    pub epoch: SceneEpoch,
    pub base: Revision,
    pub revision: Revision,
    pub changes: Vec<AcceptedLabelChange>,
}

/// The current ordered offer labels for one affected instance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcceptedLabelChange {
    pub instance: InstanceId,
    offers: Arc<[(PresentationCapability, String)]>,
}

impl AcceptedLabelChange {
    /// The board uses the first advertised offer.
    pub fn board_label(&self) -> &str {
        self.offers.first().map_or("", |(_, label)| label)
    }

    /// The semantic tree prefers the first supported offer.
    pub fn card_label(&self, profile: &CapabilityProfile) -> &str {
        self.offers
            .iter()
            .find(|(capability, _)| profile.supports(*capability))
            .map_or_else(|| self.board_label(), |(_, label)| label)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct AcceptedLabels {
    instances: BTreeMap<PresentationKey, Vec<InstanceId>>,
    pub(crate) update: Option<AcceptedLabelUpdate>,
}

impl AcceptedLabels {
    pub(crate) fn new(presentation: &PresentationManifest) -> Self {
        let mut labels = Self::default();
        for binding in &presentation.bindings {
            labels
                .instances
                .entry(binding.key.clone())
                .or_default()
                .push(binding.instance);
        }
        labels
    }

    pub(crate) fn update(
        &mut self,
        keys: BTreeSet<PresentationKey>,
        presentation: &PresentationManifest,
        session: &ProjectionSession,
        epoch: SceneEpoch,
        base: Revision,
        revision: Revision,
    ) {
        let mut changes = Vec::new();
        for key in keys {
            let Some(instances) = self.instances.get(&key) else {
                continue;
            };
            let Some(offers) = presentation.offers.get(&key) else {
                continue;
            };
            let offers: Arc<[_]> = offers
                .iter()
                .map(|offer| (offer.requires, offer.semantics.label.clone()))
                .collect();
            changes.extend(instances.iter().map(|instance| AcceptedLabelChange {
                instance: *instance,
                offers: offers.clone(),
            }));
        }
        self.update = Some(AcceptedLabelUpdate {
            session: session.clone(),
            epoch,
            base,
            revision,
            changes,
        });
    }
}

#[derive(Default)]
pub(crate) enum LabelChanges {
    #[default]
    Unchanged,
    Keys(BTreeSet<PresentationKey>),
    Full,
}

impl LabelChanges {
    pub(crate) fn record(&mut self, presentation: &PresentationManifest, diff: &ProjectionDiff) {
        if matches!(self, Self::Full) {
            return;
        }
        if !diff.scene.operations.is_empty() || diff.status.is_some() {
            *self = Self::Full;
            return;
        }
        if matches!(self, Self::Unchanged) {
            *self = Self::Keys(BTreeSet::new());
        }
        let Self::Keys(keys) = self else {
            unreachable!()
        };
        for change in &diff.presentation {
            let PresentationChange::ReplaceOffers { key, offers } = change else {
                *self = Self::Full;
                return;
            };
            let Some(previous) = presentation.offers.get(key) else {
                *self = Self::Full;
                return;
            };
            if offers.is_empty()
                || previous.len() != offers.len()
                || !previous.iter().zip(offers).all(|(old, new)| {
                    old.codec == new.codec
                        && old.requires == new.requires
                        && old.semantics.role == new.semantics.role
                        && old.semantics.bounds == new.semantics.bounds
                        && old.semantics.actions == new.semantics.actions
                })
            {
                *self = Self::Full;
                return;
            }
            keys.insert(key.clone());
        }
    }
}
