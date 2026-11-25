// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The tier's part of a canvas frame: the gnode pool's document, which nodes
//! hold a gnode, the instance data, and the pass.

use std::collections::{HashMap, HashSet};

use euclid::default::Point2D;
use kernel::geometry::PortablePoint;
use kernel::graph::NodeKey;
use paint_list_api::PaintCmd;
use scenograph::link::LinkStratum;

use super::data::{self, flags, shape};
use super::pool::{self, DomMode};
use super::{Candidate, DOM_MIN_SIDE_PX, select_dom};
use crate::canvas::build::build_pool_dom;
use crate::canvas::node_billboard::NodeBillboard;
use crate::canvas::{Canvas, NodeShape, NodeState, palette};

/// Who holds a gnode this frame, and who is under the cursor.
pub(crate) struct TierChoice {
    pub chosen: Vec<NodeKey>,
    pub hovered: Option<NodeKey>,
}

#[cfg(feature = "gpu")]
impl Canvas {
    /// Give the node tier the host's device (from the renderer's own handles),
    /// or (`None`) take it away, as on device loss: the next frame falls back
    /// to the DOM path for every node.
    pub fn set_gpu_node_device(&mut self, device: Option<super::GpuNodeDevice>) {
        self.node_tier.gpu = device.map(super::gpu::GpuNodes::new);
    }

    /// The finished node texture for the host to stage, while the tier is on.
    pub fn gpu_node_target(&self) -> Option<super::GpuNodeTarget> {
        if !self.gpu_node_tier_active() {
            return None;
        }
        self.node_tier.gpu.as_ref()?.target()
    }
}

impl Canvas {
    /// Rebuild the node document for the current mode: one gnode per node, or
    /// a bounded pool.
    pub(crate) fn rebuild_node_document(&mut self) {
        let (dom, gnode_of, stage) = if self.gpu_node_tier_active() {
            let cap = self.node_tier.max_dom.min(self.graph.node_count()).max(1);
            let (dom, slots, stage) = pool::build_bounded_pool_dom(cap);
            self.node_tier.assigner.reset(cap);
            self.node_tier.dom = slots;
            self.node_tier.mode = DomMode::Bounded(self.node_tier.max_dom);
            (dom, HashMap::new(), stage)
        } else {
            self.node_tier.assigner.reset(0);
            self.node_tier.dom.clear();
            self.node_tier.mode = DomMode::Full;
            build_pool_dom(&self.graph)
        };
        self.node_document = genet_livery::LiveryDocument::new(
            dom,
            genet_livery::StyleSet::cambium(&crate::canvas::build::NODE_SHEET),
            genet_livery::Device::screen(1.0, 1.0),
        );
        self.gnode_of = gnode_of;
        self.stage_node = stage;
        self.fold_badge_nodes.clear();
    }

    /// Switch the document when the tier turned on or off since it was built.
    pub(crate) fn sync_node_document_mode(&mut self) {
        let want = if self.gpu_node_tier_active() {
            DomMode::Bounded(self.node_tier.max_dom)
        } else {
            DomMode::Full
        };
        if self.node_tier.mode != want {
            self.rebuild_node_document();
        }
    }

    /// Choose the on-screen nodes that get a gnode, and find the hovered node.
    pub(crate) fn tier_choose(&self, on_screen: &HashSet<NodeKey>, w: u32, h: u32) -> TierChoice {
        let bodies: Vec<(NodeKey, NodeBillboard)> = on_screen
            .iter()
            .filter_map(|&key| {
                let p = self.view.position_of(key)?;
                Some((
                    key,
                    self.node_billboard_at(key, PortablePoint::new(p.x, p.y))?,
                ))
            })
            .collect();
        let cursor = Point2D::new(self.cursor.0, self.cursor.1);
        let hovered = bodies
            .iter()
            .filter(|(_, body)| body.contains_screen(cursor))
            .max_by(|a, b| a.1.depth.total_cmp(&b.1.depth))
            .map(|(key, _)| *key);
        let (cx, cy) = (w as f32 * 0.5, h as f32 * 0.5);
        let mut candidates: Vec<Candidate> = bodies
            .iter()
            .map(|(key, body)| Candidate {
                key: *key,
                side_px: body.local_side * body.zoom,
                dist2: (body.center.x - cx).powi(2) + (body.center.y - cy).powi(2),
                priority: self.selected.contains(key)
                    || self.pinned_nodes.contains(key)
                    || hovered == Some(*key),
                assigned: self.node_tier.assigner.holds(*key),
            })
            .collect();
        let cap = self.node_tier.assigner.capacity();
        TierChoice {
            chosen: select_dom(&mut candidates, cap, DOM_MIN_SIDE_PX),
            hovered,
        }
    }

    /// Seat `chosen` in the pool, rewriting only elements whose holder
    /// changed, and rebuild the node-to-gnode map from the holders.
    pub(crate) fn tier_seat(&mut self, chosen: &[NodeKey]) {
        self.node_tier.assigner.assign(chosen);
        self.gnode_of.clear();
        let dom = &self.node_tier.dom;
        self.gnode_of.extend(
            self.node_tier
                .assigner
                .holders()
                .map(|(key, slot)| (key, dom[slot].gnode)),
        );
        self.node_tier.stats.dom_nodes = self.node_tier.assigner.len();
    }

    /// Fill the CPU buffers the pass uploads: positions by slot, node
    /// attributes, the link list when the structure changed, and the draw order.
    pub(crate) fn tier_fill(
        &mut self,
        chosen: &HashSet<NodeKey>,
        hovered: Option<NodeKey>,
        visible: &dyn Fn(NodeKey) -> bool,
    ) {
        let mut cpu = std::mem::take(&mut self.node_tier.cpu);
        cpu.sync_slots(
            (self.graph.structural_input_key(), self.graph.node_count()),
            self.graph.nodes().map(|(key, _)| key),
        );
        cpu.positions.fill(0.0);
        cpu.attrs.fill(0);
        let focused = (self.selected.len() == 1)
            .then(|| self.selected.iter().next().copied())
            .flatten();
        for (key, p) in self.view.positions() {
            let Some(&slot) = cpu.slot_of.get(&key) else {
                continue;
            };
            cpu.set_position(slot, p.x, p.y);
            if !visible(key) {
                continue;
            }
            let Some(body) = self.node_billboard_at(key, PortablePoint::new(p.x, p.y)) else {
                continue;
            };
            let selected = self.selected.contains(&key);
            let state = self
                .node_states
                .get(&key)
                .copied()
                .unwrap_or(NodeState::Idle);
            let mut bits = flags::VISIBLE;
            for (on, bit) in [
                (!chosen.contains(&key), flags::BODY),
                (selected, flags::SELECTED),
                (hovered == Some(key), flags::HOVERED),
                (focused == Some(key), flags::FOCUSED),
                (self.pinned_nodes.contains(&key), flags::PINNED),
            ] {
                if on {
                    bits |= bit;
                }
            }
            let half = body.local_side * 0.5;
            let silhouette = match body.silhouette {
                NodeShape::Diamond => shape::DIAMOND,
                NodeShape::Hexagon => shape::HEXAGON,
                _ => shape::ROUNDED,
            };
            cpu.set_attr(
                slot,
                data::pack_node(
                    half,
                    self.node_height(key),
                    palette::accent(selected, state).bg,
                    silhouette,
                    body.corner_radius / half,
                    bits,
                ),
            );
        }
        self.tier_fill_links(&mut cpu);
        cpu.sort_bodies(self.camera.yaw);
        self.node_tier.stats.nodes = cpu.slots();
        self.node_tier.stats.links = cpu.links.len() / 4;
        self.node_tier.cpu = cpu;
    }

    /// The link list, rebuilt only when the structure or the hidden sets move.
    fn tier_fill_links(&self, cpu: &mut data::NodeCpuData) {
        let key = (
            self.graph.structural_input_key(),
            [
                self.hidden_links.len(),
                self.hidden_edges.len(),
                self.hidden_assertions.len(),
            ],
        );
        if cpu.links_key == Some(key) {
            return;
        }
        let segments = crate::canvas::edge_cells::visible_link_segments_with_assertion_hides(
            &self.graph,
            &self.view,
            &self.hidden_links,
            &self.hidden_edges,
            &self.hidden_assertions,
            |key| self.node_size(key),
        );
        cpu.links.clear();
        for segment in &segments {
            let appearance = &segment.appearance;
            let (Some(&from), Some(&to)) = (
                cpu.slot_of.get(&appearance.display_from),
                cpu.slot_of.get(&appearance.display_to),
            ) else {
                continue;
            };
            if from == to {
                continue;
            }
            let rgba = match appearance.key.stratum {
                LinkStratum::Surface => [237, 179, 87, 194],
                LinkStratum::Resource => [140, 184, 235, 209],
            };
            let width = 1.35 + appearance.strength.max(0.0).ln_1p().min(3.0) * 0.55;
            cpu.links.extend(data::pack_link(from, to, rgba, width));
        }
        cpu.links_key = Some(key);
    }

    /// Prepare the tier's data and run the pass. The scene command places the
    /// finished texture; empty without a device or when the pass cannot run.
    pub(crate) fn tier_render_pass(
        &mut self,
        w: u32,
        h: u32,
        chosen: &HashSet<NodeKey>,
        hovered: Option<NodeKey>,
        visible: &dyn Fn(NodeKey) -> bool,
    ) -> Vec<PaintCmd> {
        #[cfg(feature = "gpu")]
        let started = std::time::Instant::now();
        self.tier_fill(chosen, hovered, visible);
        #[cfg(feature = "gpu")]
        {
            let camera = self.camera;
            let (key, scale) = (self.node_tier.producer_key, self.node_tier.pixel_scale);
            let Some(gpu) = self.node_tier.gpu.as_mut() else {
                return Vec::new();
            };
            let report = gpu.render(&super::gpu::PassInput {
                viewport: (w as f32, h as f32),
                pixel_scale: scale,
                camera,
                cpu: &self.node_tier.cpu,
                key,
            });
            let target = gpu.target();
            let stats = &mut self.node_tier.stats;
            stats.rendered = report.rendered;
            stats.uploaded_bytes = report.uploaded_bytes;
            stats.cpu_ms = started.elapsed().as_secs_f64() * 1e3;
            let Some(target) = target else {
                return Vec::new();
            };
            use paint_list_api::{CommonPlacement, ExternalTextureItem, LayoutPoint, LayoutRect};
            let bounds = LayoutRect::new(LayoutPoint::zero(), LayoutPoint::new(w as f32, h as f32));
            return vec![PaintCmd::DrawExternalTexture(ExternalTextureItem {
                placement: CommonPlacement::new(bounds),
                texture_key: target.key,
                opacity: 1.0,
                content_generation: Some(target.generation),
            })];
        }
        #[cfg(not(feature = "gpu"))]
        {
            let _ = (w, h);
            Vec::new()
        }
    }
}
