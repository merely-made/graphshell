# Compositor placement plan: does moving a layer in the OS compositor beat recomposing it

**Status:** 2026-11-20. Plan. Nothing built, nothing measured. D1 to D3 ruled the same day.

**For:** answering one question before any dependency or seam change is considered. Pelt's desktop viewer
moves an imported layer by recomposing the whole window through the host renderer. Genet's compositor
crate can hand a layer to the operating system's compositor instead. For an imported pane whose pixels do
not change, does handing *placement* to the OS compositor save host work or improve pacing on Windows?

The question came from reading forest-rs's `subduction` (a retained layer tree with native presenters)
and Raph Levien's draft "Advice for the next 100 Rust GUIs", which calls the system compositor the
unsolved part of Rust GUI. Genet already owns the native handoff, so the experiment uses Genet's own
backend and takes no new dependency.

**Not here:** adopting `subduction` or `frameclock`; a placement-only method on the
`OsCompositorBackend` trait; macOS and Wayland arms; any change to how Pelt's viewer presents. Each
waits on the readout in P4 and returns to Mark as its own question.

## Constraints

- The harness extends Pelt's existing `host/ports/pelt/desktop/smoke_windows.rs`. Import and lifetime
  ownership of the layer stay in Pelt.
- Motion is self-driven by the harness. No synthetic OS input.
- One Genet change is allowed, and only one: an inherent placement method on `WindowsDxgiBackend` (D2).
- The missed-refresh instrument is in-process (D3) and is not believed until its positive control passes.
- Conclusions are scoped to what ran: Windows, DX12, the machine and display named in the receipts.
- The machine stays calm during runs: no other builds or lanes.

## Phases

### P0. The instrument (gate)

Count refreshes the compositor showed without a new frame from us, in process, from DXGI frame
statistics on a swapchain we own or from DWM composition timing.

**Done-conditions:** a run with one frame stalled on purpose for at least three refresh intervals
reports at least two missed refreshes at that frame and none elsewhere in an otherwise idle run; the
same counter reads on the wgpu-surface path (arm A) and on Genet's swapchain path (arms B1, B2), or the
arms it cannot read are named. If arm A cannot be read in process, this returns to Mark before P1
(open question 2).

### P1. Arms A and B1 in one harness

Arm A is the viewer's path: the imported texture composed into the backbuffer with
`compose_external_texture`, then a `Fifo` present. Arm B1 is Genet's `WindowsDxgiBackend` as built: the
imported texture copied once into the surface the backend declares, then `present` each frame.

**Done-conditions:** both arms show the same imported layer at the same rectangle, checked by one
captured frame per arm; each arm reports per frame the producer redraw count, host CPU time, GPU
submissions and the P0 counter; the producer redraw count is zero while the layer is still.

### P2. Arm B2: placement only (touches genet.git)

Add an inherent method to `WindowsDxgiBackend` that sets a declared surface's transform, clip and
opacity and commits, without copying or presenting its swapchain.

**Done-conditions:** a surface presented once and then moved by this method alone shows the same pixels
at the new rectangle, checked by captured frames before and after; Genet's existing compositor tests
pass; the trait and the other platforms' backends are unchanged.

### P3. The runs

Two motions (D1). The scripted translation moves the layer along a fixed path at a fixed speed without
resizing it. The divider drag drives the viewer's own split resize, which resizes the pane.

**Done-conditions:** each arm runs each motion for the same duration with the same seed and path;
the scripted translation shows zero producer redraws in every arm, or the arm that redraws is
named and explained; each run's numbers and conditions are in the receipts file.

### P4. Readout

**Done-conditions:** one table, arm by motion, of host CPU time per frame, GPU submissions per frame and
missed refreshes; a plain statement of whether placement in the OS compositor saved host work, improved
pacing, both or neither on this machine; the follow-on questions put to Mark (trait method, viewer
path, frame timing, other platforms), each with its evidence.

## Decisions

Each records the evidence put to Mark, what was offered, his answer in his words, what follows, and an
outcome. D1 to D3 are in answer order.

- **D1. Both motions (Mark, 2026-11-20):** Evidence: the viewer has no gesture that moves a pane without
  resizing it; its gestures are divider drag, tab drag and window drag (Findings 1). Offered: scripted
  translation (recommended); divider drag; both. Answer: "Both". What follows: P3 runs the scripted
  translation as the clean comparison and the divider drag as the realism check; producers are expected
  to redraw under the divider drag, so its numbers mix placement cost with redraw cost and are read that
  way. Outcome: a method choice for this plan; it bears on no doctrine statement.

  Links: area The stack · plans compositor-placement P3 · summary The experiment runs a scripted translation and the viewer's divider drag.

- **D2. Placement lives on the Windows backend only (Mark, 2026-11-20):** Evidence: `WindowsDxgiBackend`'s
  `present` always copies, presents the surface's swapchain and commits; there is no placement-only call
  (Findings 3). Offered: an inherent method on the Windows backend (recommended); a method on the
  `OsCompositorBackend` trait with CALayer and Wayland implementations; no Genet change. Answer: "Method
  on Windows backend only (Recommended)". What follows: P2 adds the inherent method; the trait question
  waits for P4. Outcome: applies `mere:stack.3` (the seam is not widened before there is evidence).

  Links: area The stack · draws on mere:stack.3 · plans compositor-placement P2 · summary The placement-only call is an inherent method on Genet's Windows backend; the trait is unchanged until the result is in.

- **D3. The instrument is in-process (Mark, 2026-11-20):** Evidence: neither path counts missed
  refreshes; `PresentationOutcome` reports that the handoff was accepted, not that it was shown
  (Findings 4). Offered: in-process DXGI or DWM timing (recommended); external PresentMon; both,
  cross-checked. Answer: "In-process DXGI/DWM timing (Recommended)". What follows: P0 builds it with a
  stalled frame as its positive control; nothing is downloaded. Outcome: a method choice for this plan;
  it bears on no doctrine statement.

  Links: area The stack · plans compositor-placement P0 · summary Missed refreshes are counted in process from DXGI or DWM timing, proved by a stalled frame.

## Open questions

1. How the imported texture reaches the backend's surface. Pelt imports through `Dx12SurfaceCache`; the
   backend's `declare` returns a destination texture on the host's device. Whether both sit on one wgpu
   device in the harness is unverified.
2. Whether arm A's wgpu surface can be read by the P0 counter. If not, arm A's pacing is known only
   from clock reads around present, and the missed-refresh comparison weakens. Returns to Mark at P0.
3. What the divider drag arm compares. All three arms redraw producers during a resize, so B2 may
   reduce to B1 there.
4. Frame timing. Pelt has no vsync tick and wgpu reports no present feedback, so `frameclock` does not
   drop in on Windows (Findings 5). It becomes a question only if P4 shows a pacing problem.

## Findings

2026-11-20, from reading source; nothing was run.

1. Viewer gestures are `DividerDrag` and `TabDrag` (`host/ports/pelt/desktop/workspace_viewer.rs:893`,
   `:902`) plus window drag. None translates a pane without resizing it.
2. The viewer composes each imported layer into the backbuffer and presents once per redraw
   (`workspace_viewer.rs:2798`, `:2915`). The surface is `Fifo` with a frame latency of 2
   (`genet/components/genet-render-host/src/lib.rs:748`).
3. Genet's `OsCompositorBackend::present` on Windows goes to `present_surface`, which copies, presents
   the swapchain and commits (`genet/components/genet-compositor/compositor_dxgi.rs:419`, `:807`).
   Each declared surface is its own DirectComposition visual with its own swapchain.
4. `PresentationOutcome::DisplayHandoff` means the native call accepted the frame, not that the OS
   scanned it out (`genet/components/genet-compositor/compositor.rs:64`).
5. Pelt's loop is event-driven: changes call `request_redraw`, idle maps Cambium's `IdlePolicy` to
   winit's wait (`workspace_viewer.rs:5186`), and animation time is read when the code runs
   (`workspace_viewer.rs:688`). `frameclock_windows` supplies clock reads and tick construction but
   expects the host to supply vsync-paced callbacks.
6. In graphshell's `host/`, only Pelt's `smoke_windows.rs`, `smoke_macos.rs`, `smoke_wayland.rs` and
   `smoke_webgl.rs` reference Genet's compositor backends; Turnstone's `ports/pelt/viewer.rs` names one
   in a comment on its macOS smoke launcher. `smoke_windows.rs` presents a synthetic scene through
   `render_with_compositor`; it does not use the imported texture. Other repositories were not searched.

## Progress

- 2026-11-20: plan written; D1 to D3 ruled.
