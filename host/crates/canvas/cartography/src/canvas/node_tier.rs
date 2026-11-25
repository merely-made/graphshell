// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The in-scene node tier: node bodies and links drawn by instanced GPU
//! passes on the host's device, with real `.gnode` DOM elements only for a
//! bounded, LOD-chosen set.
//!
//! Without the tier every graph node owns a `.gnode` that is restyled, laid
//! out and painted each frame, so a frame costs the same however few nodes are
//! on screen. With it (feature `gpu`, a device from [`GpuNodeDevice`], and
//! [`Canvas::set_gpu_node_tier`]) the canvas keeps a recycled pool of at most
//! [`DEFAULT_MAX_DOM_GNODES`] gnodes for the nodes that are large enough to
//! read or that the person is working with, and draws every other node and
//! every link on the GPU into a texture the scene places as an external
//! texture. The DOM path is unchanged when the tier is off.
//!
//! # Host contract
//!
//! After [`Canvas::frame`], read [`Canvas::gpu_node_target`] and stage its view
//! under `netrender::external_image_key(target.key)` (premultiplied alpha, its
//! `generation` as the staging generation) before the scene is rasterized. The
//! canvas has already submitted its node pass on the device's queue inside
//! `frame`, so the host's staging and composite submissions, made later on the
//! same queue, observe the finished texture.

mod data;
mod frame;
mod lod;
mod pool;

#[cfg(feature = "gpu")]
mod gpu;

#[cfg(all(test, feature = "gpu"))]
mod gpu_tests;
#[cfg(test)]
mod tests;

pub(crate) use data::NodeCpuData;
pub(crate) use lod::{Candidate, SlotAssigner, select_dom};
pub(crate) use pool::{DomMode, PoolDom};

use super::Canvas;

/// How many `.gnode` elements the tier's pool holds unless the host says.
pub const DEFAULT_MAX_DOM_GNODES: usize = 256;

/// A node earns a DOM gnode (caption, face) once its body is this many screen
/// pixels across, or when it is selected, hovered, focused or pinned.
pub const DOM_MIN_SIDE_PX: f32 = 12.0;

/// What the tier cost on its last frame, for a host's profile.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NodeTierStats {
    /// Nodes and links in the instance buffers.
    pub nodes: usize,
    pub links: usize,
    /// Nodes that held a DOM gnode.
    pub dom_nodes: usize,
    /// Bytes written to the device this frame (positions, order, attributes,
    /// links, camera).
    pub uploaded_bytes: usize,
    /// Whether the pass ran (false when nothing it draws had changed).
    pub rendered: bool,
    /// CPU milliseconds in the tier's data build, upload and encode.
    pub cpu_ms: f64,
}

/// The finished node texture, for the host to stage under `key`.
#[cfg(feature = "gpu")]
#[derive(Clone, Debug)]
pub struct GpuNodeTarget {
    /// A texture view of the pass's output: `Rgba8Unorm`, premultiplied alpha.
    pub view: wgpu::TextureView,
    /// The producer key the scene's external-texture item names; the host
    /// stages under `netrender::external_image_key(key)`.
    pub key: u64,
    /// Texture size in device pixels (viewport times the pixel scale).
    pub size: [u32; 2],
    /// Advances whenever the pixels change; pass it as the staging generation.
    pub generation: u64,
}

/// The host's wgpu device and queue, from the handles its renderer uses.
/// Never a second device: the texture the pass writes is sampled by the
/// host's own composite.
#[cfg(feature = "gpu")]
#[derive(Clone)]
pub struct GpuNodeDevice {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

/// The node tier's device from the renderer's handles.
#[cfg(feature = "gpu")]
pub fn gpu_node_device_for(handles: &netrender::WgpuHandles) -> GpuNodeDevice {
    GpuNodeDevice {
        device: handles.device.clone(),
        queue: handles.queue.clone(),
    }
}

/// Tier state a [`Canvas`] carries.
pub(crate) struct NodeTier {
    pub(crate) enabled: bool,
    pub(crate) producer_key: u64,
    pub(crate) pixel_scale: f32,
    pub(crate) max_dom: usize,
    pub(crate) assigner: SlotAssigner,
    pub(crate) dom: Vec<PoolDom>,
    pub(crate) mode: DomMode,
    pub(crate) cpu: NodeCpuData,
    pub(crate) stats: NodeTierStats,
    #[cfg(feature = "gpu")]
    pub(crate) gpu: Option<gpu::GpuNodes>,
    /// Lets tests drive the bounded DOM tier without an adapter.
    #[cfg(test)]
    pub(crate) forced: bool,
}

impl NodeTier {
    pub(crate) fn new() -> Self {
        Self {
            enabled: false,
            producer_key: 0,
            pixel_scale: 1.0,
            max_dom: DEFAULT_MAX_DOM_GNODES,
            assigner: SlotAssigner::default(),
            dom: Vec::new(),
            mode: DomMode::Full,
            cpu: NodeCpuData::default(),
            stats: NodeTierStats::default(),
            #[cfg(feature = "gpu")]
            gpu: None,
            #[cfg(test)]
            forced: false,
        }
    }

    /// Whether frames take the tiered path: asked for, and able to draw.
    pub(crate) fn active(&self) -> bool {
        #[cfg(test)]
        if self.forced {
            return true;
        }
        #[cfg(feature = "gpu")]
        {
            self.enabled && self.gpu.is_some()
        }
        #[cfg(not(feature = "gpu"))]
        {
            false
        }
    }
}

impl Canvas {
    /// Draw nodes and links on the GPU with a bounded DOM tier over them
    /// (default off). Takes effect with a device; see
    /// [`Canvas::set_gpu_node_device`].
    pub fn set_gpu_node_tier(&mut self, on: bool) {
        self.node_tier.enabled = on;
    }

    /// Whether the next frame takes the GPU tier's path.
    pub fn gpu_node_tier_active(&self) -> bool {
        !self.render_gnodes_as_dom && self.node_tier.active()
    }

    /// The producer key the scene's external-texture item names, and the
    /// device pixels per layout pixel the host rasterizes at. Set before
    /// `frame`; two panes over one canvas need distinct keys.
    pub fn set_gpu_node_target(&mut self, key: u64, pixel_scale: f32) {
        self.node_tier.producer_key = key;
        self.node_tier.pixel_scale = if pixel_scale.is_finite() && pixel_scale > 0.0 {
            pixel_scale
        } else {
            1.0
        };
    }

    /// The most `.gnode` elements the tier keeps (at least one).
    pub fn set_max_dom_gnodes(&mut self, max: usize) {
        self.node_tier.max_dom = max.max(1);
    }

    pub fn max_dom_gnodes(&self) -> usize {
        self.node_tier.max_dom
    }

    /// The last frame's tier cost.
    pub fn gpu_node_stats(&self) -> NodeTierStats {
        self.node_tier.stats
    }
}
