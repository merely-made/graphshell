// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Staged portable capture. Admission precedes registry lifecycle/cache changes.
use super::*;
use paint_list_render::bundle::{BundleError, ResourceAlpha, ResourceEncoding};
use std::sync::Arc;

#[derive(Debug)]
pub enum PortableCaptureError {
    MissingLayout,
    DuplicateDomKey {
        texture_key: u64,
    },
    InvalidExtent {
        texture_key: u64,
    },
    Producer {
        texture_key: u64,
        error: PortableProducerError,
    },
    GenerationExhausted,
    Bundle(BundleError),
}

#[cfg(test)]
#[path = "portable_tests.rs"]
mod tests;

pub(super) struct PortableSlot {
    pub(super) owner: NodeId,
    pub(super) active: bool,
    source: Option<Arc<ProducerSource>>,
    digest: Option<[u8; 32]>,
}

pub(crate) struct PortablePlan {
    slots: BTreeMap<u64, PortableSlot>,
    retire: Vec<u64>,
    rebound: Vec<u64>,
    next_stamp: u64,
    purpose: ProducerFramePurpose,
}

fn same_sources<'a>(
    sources: impl Iterator<Item = &'a ProducerSource>,
    capture: &PortablePaintCapture,
) -> bool {
    sources
        .map(|source| (source.texture_key, source.content_generation))
        .eq(capture
            .producers
            .iter()
            .map(|source| (source.texture_key, source.content_generation)))
}

impl PortablePlan {
    pub(crate) fn commands(&self, key: u64) -> Option<Vec<PaintCmd>> {
        let slot = self.slots.get(&key)?;
        let Some(source) = &slot.source else {
            return Some(Vec::new());
        };
        Some(vec![PaintCmd::DrawExternalTexture(ExternalTextureItem {
            placement: CommonPlacement::new(LayoutRect::new(
                LayoutPoint::new(0.0, 0.0),
                LayoutPoint::new(source.logical_size[0], source.logical_size[1]),
            )),
            texture_key: key,
            opacity: 1.0,
            content_generation: Some(source.content_generation),
        })])
    }
}

impl ProducerRegistry {
    #[cfg(test)]
    pub(crate) fn plan_portable<D: LayoutDom<NodeId = NodeId>>(
        &mut self,
        layout: &OwnedLayout,
        dom: &D,
        scale: f32,
        elsewhere: Elsewhere<'_>,
    ) -> Result<PortablePlan, PortableCaptureError> {
        self.plan_portable_for(layout, dom, scale, elsewhere, ProducerFramePurpose::Export)
    }

    pub(crate) fn plan_portable_for<D: LayoutDom<NodeId = NodeId>>(
        &mut self,
        layout: &OwnedLayout,
        dom: &D,
        scale: f32,
        elsewhere: Elsewhere<'_>,
        purpose: ProducerFramePurpose,
    ) -> Result<PortablePlan, PortableCaptureError> {
        let mut nodes = BTreeMap::new();
        for (key, node) in layout.custom_leaf_nodes(dom) {
            if nodes.insert(key, node).is_some() && self.entries.contains_key(&key) {
                return Err(PortableCaptureError::DuplicateDomKey { texture_key: key });
            }
        }
        let mut plan = PortablePlan {
            slots: BTreeMap::new(),
            retire: Vec::new(),
            rebound: Vec::new(),
            next_stamp: self.next_stamp,
            purpose,
        };
        for (&key, entry) in &mut self.entries {
            let previous_owner = entry
                .portable
                .as_ref()
                .map(|slot| slot.owner)
                .or(entry.owner);
            let Some(&owner) = nodes.get(&key) else {
                if previous_owner.is_some() && !elsewhere(key, previous_owner) {
                    plan.retire.push(key);
                }
                continue;
            };
            if previous_owner
                .is_some_and(|previous| previous != owner && elsewhere(key, Some(previous)))
            {
                return Err(PortableCaptureError::DuplicateDomKey { texture_key: key });
            }
            if previous_owner.is_some_and(|previous| previous != owner) {
                plan.rebound.push(key);
            }
            let Some(geometry) = layout.element_geometry(dom, owner) else {
                plan.slots.insert(
                    key,
                    PortableSlot {
                        owner,
                        active: false,
                        source: None,
                        digest: None,
                    },
                );
                continue;
            };
            let logical = geometry.content_size();
            if logical.0 == 0.0 || logical.1 == 0.0 {
                plan.slots.insert(
                    key,
                    PortableSlot {
                        owner,
                        active: false,
                        source: None,
                        digest: None,
                    },
                );
                continue;
            }
            let limits = paint_list_render::bundle::BundleLimits::default();
            if [logical.0, logical.1].iter().any(|value| {
                !value.is_finite() || *value < 0.0 || value.round() > limits.max_dimension as f32
            }) {
                return Err(PortableCaptureError::InvalidExtent { texture_key: key });
            }
            let size = physical_size(logical, scale, limits.max_dimension)
                .ok_or(PortableCaptureError::InvalidExtent { texture_key: key })?;
            if u64::from(size[0]) * u64::from(size[1]) > limits.max_pixels as u64 {
                return Err(PortableCaptureError::InvalidExtent { texture_key: key });
            }
            let old_slot = entry.portable.as_ref().filter(|slot| slot.owner == owner);
            let old = old_slot.and_then(|slot| slot.source.as_ref());
            let frame = ProducerFrameInfo {
                purpose,
                logical_size: logical,
                physical_size: size,
                layout_scale: scale,
                needs_frame: old_slot.is_none_or(|slot| !slot.active)
                    || old.is_none_or(|source| {
                        source.logical_size != [logical.0, logical.1]
                            || source.physical_size != size
                            || source.layout_scale != scale
                    }),
                timestamp: self.timestamp,
                appearance: layout.resolved_appearance(owner, &entry.properties),
            };
            let output = match purpose {
                ProducerFramePurpose::Presentation => entry.producer.portable_scene_update(
                    &frame,
                    old.map(|source| (source.source_identity, source.source_revision)),
                ),
                ProducerFramePurpose::Export => entry
                    .producer
                    .portable_scene(&frame)
                    .map(PortableSceneUpdate::Produced),
            }
            .map_err(|error| PortableCaptureError::Producer {
                texture_key: key,
                error,
            })?;
            let output = match output {
                PortableSceneUpdate::Produced(output) => output,
                PortableSceneUpdate::Retain {
                    source_identity,
                    source_revision,
                } if !frame.needs_frame
                    && old.is_some_and(|source| {
                        source.source_identity == source_identity
                            && source.source_revision == source_revision
                    }) =>
                {
                    let old = old_slot.expect("a retained extent has an admitted slot");
                    plan.slots.insert(
                        key,
                        PortableSlot {
                            owner,
                            active: true,
                            source: old.source.clone(),
                            digest: old.digest,
                        },
                    );
                    continue;
                },
                PortableSceneUpdate::Retain { .. } => {
                    return Err(PortableCaptureError::Producer {
                        texture_key: key,
                        error: PortableProducerError::InvalidSource,
                    });
                },
            };
            if output.source_identity == 0 {
                return Err(PortableCaptureError::Producer {
                    texture_key: key,
                    error: PortableProducerError::InvalidSource,
                });
            }
            let alpha = match output.alpha {
                SourceAlpha::Straight => ResourceAlpha::Straight,
                SourceAlpha::Premultiplied => ResourceAlpha::Premultiplied,
            };
            let encoding = match output.encoding {
                SourceEncoding::Srgb => ResourceEncoding::Srgb,
                SourceEncoding::LinearSrgb => ResourceEncoding::LinearSrgb,
            };
            plan.slots.insert(
                key,
                PortableSlot {
                    owner,
                    active: purpose == ProducerFramePurpose::Presentation
                        || entry
                            .portable
                            .as_ref()
                            .is_some_and(|slot| slot.owner == owner && slot.active),
                    source: Some(Arc::new(ProducerSource {
                        texture_key: key,
                        content_generation: 0,
                        source_identity: output.source_identity,
                        source_revision: output.source_revision,
                        logical_size: [logical.0, logical.1],
                        physical_size: size,
                        layout_scale: scale,
                        alpha,
                        encoding,
                        scene: output.scene,
                    })),
                    digest: None,
                },
            );
        }
        let limits = paint_list_render::bundle::BundleLimits::default();
        // Own the callbacks' outputs first. The entire borrowed plan must fit
        // its aggregate budget before a digest clones or serializes any Scene.
        let sources = || {
            plan.slots
                .values()
                .filter_map(|slot| slot.source.as_deref())
        };
        let already_admitted = self.portable_capture.as_ref().is_some_and(|capture| {
            plan.slots
                .values()
                .all(|slot| slot.source.is_none() || slot.digest.is_some())
                && same_sources(sources(), capture)
        });
        if !already_admitted {
            paint_list_render::bundle::validate_source_scenes(
                sources().map(|source| &source.scene),
                limits,
            )
            .map_err(PortableCaptureError::Bundle)?;
        }
        for (&key, slot) in &mut plan.slots {
            if slot.digest.is_some() {
                continue;
            }
            let Some(source) = slot.source.as_mut() else {
                continue;
            };
            let source =
                Arc::get_mut(source).expect("a newly produced source is owned by the plan");
            let old_slot = self.entries[&key]
                .portable
                .as_ref()
                .filter(|previous| previous.owner == slot.owner);
            let old = old_slot.and_then(|slot| slot.source.as_ref());
            let digest =
                paint_list_render::bundle::bounded_source_scene_digest(&source.scene, limits)
                    .map_err(PortableCaptureError::Bundle)?;
            let old_digest = old_slot.and_then(|slot| slot.digest);
            slot.digest = Some(digest);
            let unchanged = old.filter(|old| {
                old.source_identity == source.source_identity
                    && old.source_revision == source.source_revision
                    && old.logical_size == source.logical_size
                    && old.physical_size == source.physical_size
                    && old.layout_scale == source.layout_scale
                    && old.alpha == source.alpha
                    && old.encoding == source.encoding
                    && old_digest == Some(digest)
            });
            source.content_generation = if let Some(old) = unchanged {
                old.content_generation
            } else {
                plan.next_stamp = plan
                    .next_stamp
                    .checked_add(1)
                    .ok_or(PortableCaptureError::GenerationExhausted)?;
                plan.next_stamp
            };
        }
        Ok(plan)
    }

    pub(crate) fn admit_portable(
        &mut self,
        mut plan: PortablePlan,
        envelope: paint_list_api::PaintEnvelope,
    ) -> Result<Arc<PortablePaintCapture>, PortableCaptureError> {
        let referenced: std::collections::BTreeSet<_> = envelope
            .commands
            .iter()
            .filter_map(|command| {
                if let PaintCmd::DrawExternalTexture(item) = command {
                    Some(item.texture_key)
                } else {
                    None
                }
            })
            .collect();
        let presentation = plan.purpose == ProducerFramePurpose::Presentation;
        let sources = || {
            plan.slots
                .iter()
                .filter(|(key, _)| referenced.contains(*key))
                .filter_map(|(_, slot)| slot.source.as_deref())
        };
        let previous = self.portable_capture.as_ref().filter(|previous| {
            presentation && previous.envelope == envelope && same_sources(sources(), previous)
        });
        let capture = if let Some(previous) = previous {
            previous.clone()
        } else {
            let producers = plan
                .slots
                .iter_mut()
                .filter(|(key, _)| referenced.contains(*key))
                .filter_map(|(_, slot)| slot.source.take())
                .map(Arc::unwrap_or_clone)
                .collect();
            let capture = PortablePaintCapture {
                envelope,
                producers,
            };
            paint_list_render::bundle::validate_capture(&capture)
                .map_err(PortableCaptureError::Bundle)?;
            for source in &capture.producers {
                plan.slots
                    .get_mut(&source.texture_key)
                    .expect("source came from plan")
                    .source = Some(Arc::new(source.clone()));
            }
            Arc::new(capture)
        };
        for (key, slot) in &mut plan.slots {
            if !referenced.contains(key) {
                slot.source = None;
                slot.digest = None;
            }
        }
        if presentation {
            self.portable_capture = Some(capture.clone());
        }
        self.commit_portable(plan);
        Ok(capture)
    }

    /// Called only after the intact envelope and entire resource ledger pass admission.
    pub(crate) fn commit_portable(&mut self, plan: PortablePlan) {
        for key in plan.retire {
            self.remove(key);
        }
        for key in plan.rebound {
            if let Some(entry) = self.entries.get_mut(&key) {
                entry.producer.retire();
                entry.active = false;
                entry.image = None;
                entry.owner = None;
                self.pending_retire.push(entry.renderer_key);
            }
        }
        for (key, slot) in plan.slots {
            if let Some(entry) = self.entries.get_mut(&key) {
                if entry
                    .portable
                    .as_ref()
                    .is_some_and(|previous| previous.active)
                    && !slot.active
                    && !entry.active
                {
                    entry.producer.suspend();
                }
                entry.portable = Some(slot);
            }
        }
        self.next_stamp = plan.next_stamp;
    }
}
