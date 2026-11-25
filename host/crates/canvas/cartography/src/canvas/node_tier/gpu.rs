// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The node pass: instanced bodies and links into a premultiplied texture on
//! the host's device.
//!
//! Positions sit in their own storage buffer, read by slot in the vertex
//! stage. [`GpuNodes::bind_positions`] swaps in a device-resident buffer of
//! the same `[slots, 4]` f32 layout (Seiche's resident graph publishes one)
//! with no change to the shaders or the other buffers.

use super::data::NodeCpuData;
use super::{GpuNodeDevice, GpuNodeTarget};
use crate::canvas::palette;
use crate::canvas::scene_paint::Camera;

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const GLOBALS_BYTES: usize = 96;

/// What one pass needs besides the CPU data.
pub(crate) struct PassInput<'a> {
    /// Viewport in layout px.
    pub viewport: (f32, f32),
    /// Device px per layout px.
    pub pixel_scale: f32,
    pub camera: Camera,
    pub cpu: &'a NodeCpuData,
    pub key: u64,
}

pub(crate) struct PassReport {
    pub rendered: bool,
    pub uploaded_bytes: usize,
}

/// One growable storage buffer and the bytes last written to it.
struct Store {
    buffer: wgpu::Buffer,
    capacity: u64,
    last: Vec<u8>,
}

fn words_to_bytes(out: &mut Vec<u8>, words: &[u32]) {
    out.clear();
    out.extend(words.iter().flat_map(|w| w.to_le_bytes()));
}

fn floats_to_bytes(out: &mut Vec<u8>, floats: &[f32]) {
    out.clear();
    out.extend(floats.iter().flat_map(|f| f.to_le_bytes()));
}

impl Store {
    fn new(device: &wgpu::Device, label: &str) -> Self {
        Self {
            buffer: Self::alloc(device, label, 16),
            capacity: 16,
            last: Vec::new(),
        }
    }

    fn alloc(device: &wgpu::Device, label: &str, bytes: u64) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    /// Write `bytes` when they differ from the last write; reports whether the
    /// buffer was replaced (its bind group is stale) and the bytes sent.
    fn write(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        label: &str,
        bytes: &[u8],
    ) -> (bool, usize) {
        if self.last == bytes {
            return (false, 0);
        }
        let need = bytes.len() as u64;
        let grew = need > self.capacity;
        if grew {
            self.capacity = need.next_power_of_two();
            self.buffer = Self::alloc(device, label, self.capacity);
        }
        if !bytes.is_empty() {
            queue.write_buffer(&self.buffer, 0, bytes);
        }
        self.last.clear();
        self.last.extend_from_slice(bytes);
        (grew, bytes.len())
    }
}

pub(crate) struct GpuNodes {
    device: wgpu::Device,
    queue: wgpu::Queue,
    layout: wgpu::BindGroupLayout,
    node_pipeline: wgpu::RenderPipeline,
    link_pipeline: wgpu::RenderPipeline,
    globals: wgpu::Buffer,
    last_globals: [u8; GLOBALS_BYTES],
    positions: Store,
    attrs: Store,
    order: Store,
    links: Store,
    external_positions: Option<wgpu::Buffer>,
    bind_group: Option<wgpu::BindGroup>,
    texture: Option<(wgpu::Texture, wgpu::TextureView, [u32; 2])>,
    generation: u64,
    key: u64,
    bodies: u32,
    link_count: u32,
    scratch: Vec<u8>,
}

fn storage_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::VERTEX,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

impl GpuNodes {
    pub(crate) fn new(handle: GpuNodeDevice) -> Self {
        let GpuNodeDevice { device, queue } = handle;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("cartography node tier"),
            source: wgpu::ShaderSource::Wgsl(include_str!("nodes.wgsl").into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("cartography node tier layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                storage_entry(1),
                storage_entry(2),
                storage_entry(3),
                storage_entry(4),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("cartography node tier pipeline layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let make = |label: &str, vs: &str, fs: &str| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some(vs),
                    buffers: &[],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(fs),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: FORMAT,
                        blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            })
        };
        let node_pipeline = make("cartography node bodies", "vs_node", "fs_node");
        let link_pipeline = make("cartography node links", "vs_link", "fs_link");
        let globals = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cartography node tier globals"),
            size: GLOBALS_BYTES as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            positions: Store::new(&device, "node positions"),
            attrs: Store::new(&device, "node attributes"),
            order: Store::new(&device, "node draw order"),
            links: Store::new(&device, "node links"),
            device,
            queue,
            layout,
            node_pipeline,
            link_pipeline,
            globals,
            last_globals: [0; GLOBALS_BYTES],
            external_positions: None,
            bind_group: None,
            texture: None,
            generation: 0,
            key: 0,
            bodies: 0,
            link_count: 0,
            scratch: Vec::new(),
        }
    }

    /// Read positions from `buffer` (`[slots, 4]` f32, `x y 0 0`, slot order as
    /// the canvas's) instead of the canvas's CPU upload, or (`None`) return to
    /// the upload. The caller keeps the buffer alive and rebinds when its
    /// allocation changes (Seiche's `allocation_epoch`).
    #[allow(dead_code)]
    pub(crate) fn bind_positions(&mut self, buffer: Option<wgpu::Buffer>) {
        self.external_positions = buffer;
        self.bind_group = None;
        self.positions.last.clear();
    }

    pub(crate) fn target(&self) -> Option<GpuNodeTarget> {
        let (_, view, size) = self.texture.as_ref()?;
        Some(GpuNodeTarget {
            view: view.clone(),
            key: self.key,
            size: *size,
            generation: self.generation,
        })
    }

    pub(crate) fn texture(&self) -> Option<&wgpu::Texture> {
        self.texture.as_ref().map(|(texture, _, _)| texture)
    }

    fn globals_bytes(input: &PassInput) -> [u8; GLOBALS_BYTES] {
        let cam = input.camera;
        let (sin, cos) = cam.yaw.sin_cos();
        let accent = |rgb: [u8; 3], a: f32| {
            [
                f32::from(rgb[0]) / 255.0,
                f32::from(rgb[1]) / 255.0,
                f32::from(rgb[2]) / 255.0,
                a,
            ]
        };
        let mut words = [0f32; GLOBALS_BYTES / 4];
        let head = [
            cam.offset.0,
            cam.offset.1,
            input.viewport.0,
            input.viewport.1,
            cam.zoom,
            cam.tilt,
            sin,
            cos,
            input.pixel_scale,
            0.0,
            0.0,
            0.0,
        ];
        words[..12].copy_from_slice(&head);
        // Selection: the selected fill's amber, lightened. Focus and pin follow
        // the caption ink and a dark mark.
        words[12..16].copy_from_slice(&accent([250, 196, 112], 0.95));
        words[16..20].copy_from_slice(&accent(palette::CAPTION_FG, 0.9));
        words[20..24].copy_from_slice(&accent([20, 22, 30], 0.85));
        let mut out = [0u8; GLOBALS_BYTES];
        for (chunk, word) in out.chunks_exact_mut(4).zip(words) {
            chunk.copy_from_slice(&word.to_le_bytes());
        }
        out
    }

    fn ensure_texture(&mut self, size: [u32; 2]) -> bool {
        if self
            .texture
            .as_ref()
            .is_some_and(|(_, _, have)| *have == size)
        {
            return false;
        }
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("cartography node tier"),
            size: wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.texture = Some((texture, view, size));
        true
    }

    /// Upload what changed and draw links, then bodies, when anything did. The
    /// pass is submitted on the device's queue before this returns.
    pub(crate) fn render(&mut self, input: &PassInput) -> PassReport {
        let size = [
            (input.viewport.0 * input.pixel_scale).round().max(1.0) as u32,
            (input.viewport.1 * input.pixel_scale).round().max(1.0) as u32,
        ];
        let mut stale = self.ensure_texture(size) || self.key != input.key;
        self.key = input.key;
        let mut uploaded = 0;
        let cpu = input.cpu;
        if self.external_positions.is_none() {
            floats_to_bytes(&mut self.scratch, &cpu.positions);
            let (replaced, bytes) =
                self.positions
                    .write(&self.device, &self.queue, "node positions", &self.scratch);
            stale |= bytes > 0;
            if replaced {
                self.bind_group = None;
            }
            uploaded += bytes;
        }
        for (store, label, words) in [
            (&mut self.attrs, "node attributes", &cpu.attrs[..]),
            (&mut self.order, "node draw order", &cpu.order[..]),
            (&mut self.links, "node links", &cpu.links[..]),
        ] {
            words_to_bytes(&mut self.scratch, words);
            let (replaced, bytes) = store.write(&self.device, &self.queue, label, &self.scratch);
            stale |= bytes > 0;
            if replaced {
                self.bind_group = None;
            }
            uploaded += bytes;
        }
        let globals = Self::globals_bytes(input);
        if globals != self.last_globals {
            self.queue.write_buffer(&self.globals, 0, &globals);
            self.last_globals = globals;
            stale = true;
            uploaded += GLOBALS_BYTES;
        }
        self.bodies = cpu.order.len() as u32;
        self.link_count = (cpu.links.len() / 4) as u32;
        if !stale {
            return PassReport {
                rendered: false,
                uploaded_bytes: 0,
            };
        }
        let group = self.bind_group.get_or_insert_with(|| {
            let positions = self
                .external_positions
                .as_ref()
                .unwrap_or(&self.positions.buffer);
            fn entry(binding: u32, buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
                wgpu::BindGroupEntry {
                    binding,
                    resource: buffer.as_entire_binding(),
                }
            }
            self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("cartography node tier bind group"),
                layout: &self.layout,
                entries: &[
                    entry(0, &self.globals),
                    entry(1, positions),
                    entry(2, &self.attrs.buffer),
                    entry(3, &self.order.buffer),
                    entry(4, &self.links.buffer),
                ],
            })
        });
        let (_, view, _) = self.texture.as_ref().expect("texture ensured above");
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("cartography node tier"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("cartography node tier"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &*group, &[]);
            if self.link_count > 0 {
                pass.set_pipeline(&self.link_pipeline);
                pass.draw(0..6, 0..self.link_count);
            }
            if self.bodies > 0 {
                pass.set_pipeline(&self.node_pipeline);
                pass.draw(0..6, 0..self.bodies);
            }
        }
        self.queue.submit([encoder.finish()]);
        self.generation += 1;
        PassReport {
            rendered: true,
            uploaded_bytes: uploaded,
        }
    }
}
