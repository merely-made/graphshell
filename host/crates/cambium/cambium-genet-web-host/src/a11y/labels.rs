// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Label-only publication: a producer's list whose item names changed is
//! patched in place, and a camera move shifts its items' common plane.

use std::collections::HashSet;

use cambium_rootstock::{
    A11yAdapterState, A11yPublication, A11yPublicationOutcome, A11ySync, LeafRegistry, OwnedLayout,
    ProducerLabelRevision, SemanticStamp, WindowDom,
};
use wasm_bindgen::JsCast;
use web_sys::HtmlElement;

use super::{DomAccessibility, update};
use crate::mirror::{self, MirrorNode};

/// What one sync reads, bundled for the label paths.
pub(super) struct SyncInputs<'a, 'd> {
    pub owner: &'a cambium_rootstock::A11yOwner,
    pub dom: &'a WindowDom<'d>,
    pub layout: &'a OwnedLayout,
    pub leaves: &'a LeafRegistry<u64>,
    pub producers: &'a cambium_rootstock::ProducerRegistry,
    pub focus: Option<u64>,
    pub scale: f32,
}

pub(super) struct LabelPublication {
    owner: cambium_rootstock::A11yOwner,
    layout: SemanticStamp,
    producer: ProducerLabelRevision,
    focus: Option<u64>,
    scale: f32,
    pub list: Option<PublishedList>,
}

pub(super) struct PublishedList {
    bounds: accesskit::Rect,
    content_rect: (f32, f32, f32, f32),
    name: Option<String>,
    children: HashSet<u64>,
    document_nodes: HashSet<u64>,
    plane: Option<PublishedPlane>,
}

/// The children retain their original CSS boxes. Camera changes move their
/// presentation-only parent, always relative to this immutable anchor.
struct PublishedPlane {
    element: HtmlElement,
    anchor: cambium_rootstock::ProducerSemanticGeometry,
    scale: f64,
    content_offset: [f64; 2],
    transform: [f64; 3],
}

/// A projection that kept the prior list's items in place.
pub(super) struct Retained {
    pub projection: cambium_rootstock::SemanticProjection,
    pub plan: Vec<MirrorNode>,
    bounds: accesskit::Rect,
    content_rect: (f32, f32, f32, f32),
    transform: Option<[f64; 3]>,
}

impl PublishedList {
    fn advance_labels(&mut self, current: &ProducerLabelRevision) {
        if let Some(plane) = self.plane.as_mut() {
            if let Some(geometry) = current.geometry() {
                // The separately admitted label delta proves all boxes equal.
                plane.anchor.revision = geometry.revision;
            } else {
                // Keep the wrapper for existing boxes, but refuse camera reuse.
                plane.anchor.revision = u64::MAX;
            }
        }
    }
}

pub(super) fn find_mirror(nodes: &[MirrorNode], id: u64) -> Option<&MirrorNode> {
    nodes.iter().find_map(|node| {
        if node.id == id {
            Some(node)
        } else {
            find_mirror(&node.children, id)
        }
    })
}

fn capture_list(
    projection: &cambium_rootstock::SemanticProjection,
    plan: &[MirrorNode],
    revision: &ProducerLabelRevision,
    layout: &OwnedLayout,
    dom: &WindowDom<'_>,
    live: &HashSet<u64>,
) -> Option<PublishedList> {
    let id = revision.slot_id();
    let (_, node) = projection
        .tree
        .nodes
        .iter()
        .find(|(node, _)| node.0 == id)?;
    let slot = find_mirror(plan, id)?;
    if node.role() != accesskit::Role::List
        || node.is_disabled()
        || slot.children.is_empty()
        || !slot.children.iter().all(|child| {
            child.action.is_none()
                && child.children.is_empty()
                && !child.focusable
                && projection.is_produced_list_item(child.id, revision)
                && child
                    .attrs
                    .iter()
                    .any(|(key, value)| *key == "role" && value == "listitem")
        })
    {
        return None;
    }
    let children: HashSet<_> = slot.children.iter().map(|child| child.id).collect();
    Some(PublishedList {
        bounds: projection.absolute_bounds(accesskit::NodeId(id))?,
        content_rect: revision.content_rect(layout, dom)?,
        name: node.label().map(str::to_owned),
        document_nodes: live.difference(&children).copied().collect(),
        children,
        plane: None,
    })
}

impl DomAccessibility {
    fn install_plane(
        &self,
        list: &mut PublishedList,
        revision: &ProducerLabelRevision,
        plan: &[MirrorNode],
        scale: f32,
    ) -> Result<(), wasm_bindgen::JsValue> {
        let Some(anchor) = revision.geometry() else {
            return Ok(());
        };
        let Some(slot) = find_mirror(plan, revision.slot_id()) else {
            return Ok(());
        };
        let parent = &self.elements[&revision.slot_id()].element;
        let element: HtmlElement = self.document.create_element("div")?.unchecked_into();
        element.set_attribute("role", "presentation")?;
        element.set_attribute("data-cambium-producer-plane", "")?;
        element.style().set_css_text(
            "position:absolute;left:0;top:0;margin:0;padding:0;border:0;transform-origin:0 0;overflow:visible;",
        );
        parent.append_child(&element)?;
        for child in &slot.children {
            element.append_child(&self.elements[&child.id].element)?;
        }
        list.plane = Some(PublishedPlane {
            element,
            anchor,
            scale: f64::from(scale),
            content_offset: [
                f64::from(list.content_rect.0) - list.bounds.x0,
                f64::from(list.content_rect.1) - list.bounds.y0,
            ],
            transform: [1.0, 0.0, 0.0],
        });
        Ok(())
    }

    fn patch_labels(&mut self, changes: Vec<(u64, String)>) -> Result<bool, wasm_bindgen::JsValue> {
        if !changes.iter().all(|(id, _)| {
            self.elements.get(id).is_some_and(|node| {
                self.handle.root.contains(Some(node.element.as_ref()))
                    && node
                        .written
                        .attrs
                        .iter()
                        .any(|(key, value)| *key == "role" && value == "listitem")
                    && !node
                        .written
                        .attrs
                        .iter()
                        .any(|(key, _)| *key == "aria-label")
                    && node.element.child_element_count() == 0
                    && !node.written.focusable
            })
        }) {
            return Ok(false);
        }
        for (id, name) in changes {
            let node = self.elements.get_mut(&id).expect("checked above");
            let mut wanted = node.written.clone();
            wanted.text = (!name.trim().is_empty()).then_some(name);
            update(&self.document, node, &wanted)?;
            node.written = wanted;
        }
        Ok(true)
    }

    /// Patch item names alone when nothing else about the publication moved.
    /// Consumes `source` only when the patch is the whole sync.
    pub(super) fn sync_labels_only(
        &mut self,
        inputs: &SyncInputs<'_, '_>,
        source: &mut Option<ProducerLabelRevision>,
    ) -> Option<A11ySync> {
        let previous = self.labels.as_ref()?;
        let current = source.as_ref()?;
        if &previous.owner != inputs.owner
            || previous.scale != inputs.scale
            || previous.focus != inputs.focus
            || !previous.layout.matches(inputs.layout, inputs.dom)
        {
            return None;
        }
        let changes = inputs
            .producers
            .producer_label_changes(&previous.producer, current)?;
        match self.patch_labels(changes) {
            Ok(true) => {
                let current = source.take().expect("matched current revision");
                let publication = self.labels.as_mut().expect("matched prior publication");
                if let Some(list) = publication.list.as_mut() {
                    list.advance_labels(&current);
                }
                publication.producer = current;
                Some(A11ySync {
                    requests: Vec::new(),
                    publication: A11yPublication {
                        adapter: A11yAdapterState::Installed,
                        outcome: A11yPublicationOutcome::Mirrored {
                            nodes: self.elements.len(),
                            contribution_issues: 0,
                        },
                    },
                })
            },
            Ok(false) => None,
            Err(_) => {
                self.clear_publication();
                None
            },
        }
    }

    /// Project the document around the prior list, keeping its items, when
    /// only item names or the camera changed.
    pub(super) fn retain_document(
        &mut self,
        inputs: &SyncInputs<'_, '_>,
        current: Option<&ProducerLabelRevision>,
    ) -> Option<Retained> {
        let SyncInputs {
            owner,
            dom,
            layout,
            leaves,
            producers,
            focus,
            scale,
        } = *inputs;
        let previous = self.labels.as_ref()?;
        let list = previous.list.as_ref()?;
        let current = current?;
        if &previous.owner != owner || !previous.producer.same_registration(current) {
            return None;
        }
        let content_rect = current.content_rect(layout, dom)?;
        let geometry = list.plane.as_ref().and_then(|plane| {
            current.geometry()?.transform_from(
                plane.anchor,
                plane.scale,
                f64::from(scale),
                plane.content_offset,
                [0.0, 0.0],
            )
        });
        // A common-transform proof admits camera-only changes even when
        // the producer's last name delta no longer covers this revision.
        let changes = if geometry.is_some() {
            Vec::new()
        } else {
            if previous.scale != scale || content_rect != list.content_rect {
                return None;
            }
            producers.producer_label_changes(&previous.producer, current)?
        };
        if changes.iter().any(|(id, _)| !list.children.contains(id)) {
            return None;
        }
        let projection = cambium_rootstock::project_semantics_retaining_list(
            dom,
            layout,
            leaves,
            producers,
            focus,
            current,
            list.name.as_deref(),
        );
        let (_, slot) = projection
            .tree
            .nodes
            .iter()
            .find(|(id, _)| id.0 == current.slot_id())?;
        let bounds = projection.absolute_bounds(accesskit::NodeId(current.slot_id()))?;
        if (geometry.is_none() && bounds != list.bounds)
            || ![bounds.x0, bounds.y0, bounds.x1, bounds.y1]
                .iter()
                .all(|v| v.is_finite())
            || slot.is_hidden()
            || slot.is_disabled()
            || !slot.children().is_empty()
            || !projection.producer_issues.is_empty()
            || projection
                .tree
                .nodes
                .iter()
                .any(|(id, _)| list.children.contains(&id.0))
        {
            return None;
        }
        let plan = mirror::plan(&projection, scale);
        find_mirror(&plan, current.slot_id())?;
        let transform = if geometry.is_some() {
            let plane = list.plane.as_ref()?;
            Some(current.geometry()?.transform_from(
                plane.anchor,
                plane.scale,
                f64::from(scale),
                plane.content_offset,
                [
                    f64::from(content_rect.0) - bounds.x0,
                    f64::from(content_rect.1) - bounds.y0,
                ],
            )?)
        } else {
            None
        };
        self.patch_labels(changes).ok()?.then_some(Retained {
            projection,
            plan,
            bounds,
            content_rect,
            transform,
        })
    }

    /// After `write`: move the kept list's plane, or drop the old plane, and
    /// remove elements no longer live. Err means a DOM write failed.
    pub(super) fn settle_list(
        &mut self,
        prior_list: &mut Option<PublishedList>,
        retained: Option<&Retained>,
        source: Option<&ProducerLabelRevision>,
        live: &HashSet<u64>,
    ) -> Result<(), wasm_bindgen::JsValue> {
        if let Some(list) = prior_list.as_mut() {
            if let Some(kept) = retained {
                list.bounds = kept.bounds;
                list.content_rect = kept.content_rect;
                if let (Some(plane), Some(transform)) = (list.plane.as_mut(), kept.transform)
                    && plane.transform != transform
                {
                    let [s, x, y] = transform;
                    plane
                        .element
                        .style()
                        .set_property("transform", &format!("matrix({s},0,0,{s},{x},{y})"))?;
                    plane.transform = transform;
                }
                if let Some(current) = source {
                    list.advance_labels(current);
                }
            } else if let Some(plane) = list.plane.take() {
                // `write` has already moved retained live nodes to their new
                // parents. The old anonymous container carries no identity.
                plane.element.remove();
            }
        }
        if let Some(list) = prior_list.as_mut().filter(|_| retained.is_some()) {
            for id in list.document_nodes.difference(live) {
                if let Some(old) = self.elements.remove(id) {
                    old.element.remove();
                }
            }
        } else {
            self.elements.retain(|id, mirrored| {
                let keep = live.contains(id);
                if !keep {
                    mirrored.element.remove();
                }
                keep
            });
        }
        Ok(())
    }

    /// Remember this publication for the next label-only sync; `kept` is the
    /// retained list, if any. Err means the list's plane could not be installed.
    pub(super) fn record_labels(
        &mut self,
        inputs: &SyncInputs<'_, '_>,
        source: Option<ProducerLabelRevision>,
        kept: Option<PublishedList>,
        projection: &cambium_rootstock::SemanticProjection,
        plan: &[MirrorNode],
        live: &HashSet<u64>,
    ) -> Result<(), wasm_bindgen::JsValue> {
        let Some(producer) = source else {
            self.labels = None;
            return Ok(());
        };
        let mut result = Ok(());
        let list = match kept {
            Some(mut list) => {
                list.document_nodes.clone_from(live);
                Some(list)
            },
            None => capture_list(projection, plan, &producer, inputs.layout, inputs.dom, live).map(
                |mut list| {
                    result = self.install_plane(&mut list, &producer, plan, inputs.scale);
                    list
                },
            ),
        };
        self.labels = Some(LabelPublication {
            owner: inputs.owner.clone(),
            layout: inputs.layout.semantic_stamp(inputs.dom),
            producer,
            focus: inputs.focus,
            scale: inputs.scale,
            list,
        });
        result
    }
}
