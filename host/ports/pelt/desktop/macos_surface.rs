/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::any::Any;
use std::collections::HashMap;
use std::sync::{
    Arc, Weak,
    atomic::{AtomicBool, Ordering},
};
use std::task::{Wake, Waker};

use inker::{OwnedSurfaceFrame, SurfaceFrame, SurfaceSyncHandle, SurfaceTextureFormat};
use objc2_core_foundation::CFRetained;
use objc2_io_surface::IOSurfaceRef;
use paint::{MacosImportedPane, OpaquePaneDrag};
use pelt_core::{PeltContent, PeltSurfaceLayer, PeltWorkspace, WorkspaceRect};
use raw_window_handle::HasWindowHandle;
use winit::window::Window;
use workbench::TileId;

/// A completed, fully opaque producer image. The producer transfers a retain
/// and must leave its pixels stable while the compositor may display the image.
pub struct OpaqueIOSurfaceFrame {
    pub image: CFRetained<IOSurfaceRef>,
    /// Optionally switch to notification-driven acquisition. Install the waker
    /// and wake it if a newer frame was already published during registration.
    pub register_waker: Option<Box<dyn FnOnce(Waker)>>,
}

impl std::fmt::Debug for OpaqueIOSurfaceFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("OpaqueIOSurfaceFrame")
            .field(&self.image.id())
            .finish()
    }
}

struct FrameWake {
    ready: Arc<AtomicBool>,
    window: Weak<dyn Window>,
}

impl Wake for FrameWake {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        self.ready.store(true, Ordering::Release);
        if let Some(window) = self.window.upgrade() {
            window.request_redraw();
        }
    }
}

impl OwnedSurfaceFrame for OpaqueIOSurfaceFrame {
    fn payload_kind(&self) -> &'static str {
        "pelt.opaque-iosurface"
    }
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

struct Pane {
    instance: u64,
    layer: MacosImportedPane,
    rect: WorkspaceRect,
}

#[derive(Default)]
pub(super) struct ImportedPanes {
    panes: HashMap<TileId, Pane>,
    dragging: Option<(TileId, OpaquePaneDrag)>,
    pub(super) consumed_escape: bool,
}

fn bounds(rect: WorkspaceRect) -> [f64; 4] {
    [rect.x, rect.y, rect.width, rect.height].map(f64::from)
}

impl ImportedPanes {
    pub fn sync_visibility(
        &mut self,
        workspace: &PeltWorkspace<netrender::Scene>,
        visible: &[PeltSurfaceLayer],
    ) {
        let shown = |id| visible.iter().any(|surface| surface.tile == id);
        let instance = |id| {
            workspace
                .content(id)
                .and_then(PeltContent::surface_instance_id)
        };
        if self.dragging.as_ref().is_some_and(|(id, _)| {
            !shown(*id) || self.panes.get(id).map(|p| p.instance) != instance(*id)
        }) {
            self.end();
        }
        self.panes.retain(|id, pane| {
            if Some(pane.instance) != instance(*id) {
                return false;
            }
            pane.layer.set_visible(shown(*id));
            true
        });
    }

    pub fn accept(
        &mut self,
        window: &Arc<dyn Window>,
        content: &mut PeltContent<netrender::Scene>,
        tile: TileId,
        rect: WorkspaceRect,
        frame: SurfaceFrame,
    ) -> Result<(), &'static str> {
        let fourcc = match frame.format {
            SurfaceTextureFormat::Bgra8Unorm => *b"BGRA",
            SurfaceTextureFormat::Rgba8Unorm => *b"RGBA",
            _ => return Err("expected an 8-bit RGBA or BGRA surface"),
        };
        if !matches!(frame.sync, SurfaceSyncHandle::None) {
            return Err("expected a completed producer frame");
        }
        let OpaqueIOSurfaceFrame {
            image: surface,
            register_waker,
        } = frame
            .texture
            .into_owned_payload::<OpaqueIOSurfaceFrame>()
            .map_err(|_| "expected an owned opaque IOSurface")?;
        if surface.pixel_format() != u32::from_be_bytes(fourcc)
            || surface.width() != frame.width as usize
            || surface.height() != frame.height as usize
            || frame.width == 0
            || frame.height == 0
            || surface.bytes_per_element() != 4
            || surface.plane_count() != 0
        {
            return Err("IOSurface dimensions or format disagree with the frame");
        }
        if let Some(pane) = self.panes.get(&tile) {
            pane.layer.set_surface(&surface);
        } else {
            let handle = window
                .window_handle()
                .map_err(|_| "AppKit window unavailable")?;
            let layer = MacosImportedPane::attach(handle, &surface, bounds(rect))?;
            let instance = content
                .surface_instance_id()
                .ok_or("surface producer retired")?;
            self.panes.insert(
                tile,
                Pane {
                    instance,
                    layer,
                    rect,
                },
            );
        }
        if let Some(register) = register_waker {
            let ready = Arc::new(AtomicBool::new(false));
            content.set_surface_frame_signal(ready.clone());
            register(Waker::from(Arc::new(FrameWake {
                ready,
                window: Arc::downgrade(window),
            })));
        }
        Ok(())
    }

    pub fn layout(&mut self, tile: TileId, rect: WorkspaceRect) {
        if let Some(pane) = self.panes.get_mut(&tile) {
            if pane.rect != rect {
                pane.rect = rect;
                if !self.dragging.as_ref().is_some_and(|(id, _)| *id == tile) {
                    pane.layer.place(bounds(rect));
                }
            }
        }
    }

    pub fn begin(&mut self, tile: TileId) -> bool {
        self.end();
        self.dragging = self
            .panes
            .get(&tile)
            .and_then(|pane| pane.layer.begin_drag())
            .map(|guard| (tile, guard));
        self.dragging.is_some()
    }

    pub fn drag_to(&self, tile: TileId, dx: f32, dy: f32) -> bool {
        if self.dragging.as_ref().is_some_and(|(id, _)| *id == tile)
            && let Some(pane) = self.panes.get(&tile)
        {
            pane.layer.translate_drag(f64::from(dx), f64::from(dy));
            return true;
        }
        false
    }

    pub fn end(&mut self) -> bool {
        let Some((id, guard)) = self.dragging.take() else {
            return false;
        };
        if let Some(pane) = self.panes.get(&id) {
            pane.layer.place(bounds(pane.rect));
        }
        drop(guard);
        true
    }
}
