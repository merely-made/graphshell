# Receipts: primitive representations plan

## 2026-11-14: Crystal probe 1 (ceiling, no geometry)

A throwaway script, not in the repository. Model: a pinned tile holds at most
`floor(520 / smallest side)` linked tiles on its border (a 160 by 100 tile); a link is a
border only if one end is pinned (D8); a nucleus shows at most that many of its links (D9);
a link between two nuclei uses one place on each; conflicts between neighbouring nuclei are
ignored, so every figure is a ceiling. Pins are taken breadth-first from the best-connected
node. The graph is every `.md` file under a `design_docs/` as a node and every mention of
another file's name as a link, with `DOC_README.md` and `README.md` left out.

Controls, same run: a star of 30 leaves at 10 places gives 10 borders; a path of 20 with
every node pinned gives 19 of 19; with one end pinned, 1.

Share of all links that can be borders after k pins:

| Graph | Nodes | Links | Mean, median, max links | Smallest side | Places | Nodes over | 1 | 5 | 10 | 25 | 50 | All |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Graphshell docs | 70 | 118 | 3.4, 3, 10 | 32 | 16 | 0.0% | 8% | 23% | 31% | 58% | 89% | 100% |
| | | | | 48 | 10 | 0.0% | 8% | 23% | 31% | 58% | 89% | 100% |
| | | | | 64 | 8 | 2.9% | 7% | 21% | 29% | 54% | 86% | 97% |
| | | | | 96 | 5 | 17.1% | 4% | 15% | 25% | 45% | 73% | 85% |
| Mere docs | 437 | 1468 | 6.7, 5, 116 | 32 | 16 | 5.5% | 1% | 5% | 10% | 22% | 32% | 86% |
| | | | | 48 | 10 | 16.0% | 1% | 3% | 6% | 14% | 24% | 73% |
| | | | | 64 | 8 | 25.6% | 1% | 2% | 4% | 11% | 20% | 65% |
| | | | | 96 | 5 | 45.3% | 0% | 1% | 3% | 6% | 12% | 47% |
| Turnstone docs | 74 | 134 | 3.6, 3, 15 | 32 | 16 | 0.0% | 11% | 37% | 52% | 68% | 94% | 100% |
| | | | | 48 | 10 | 2.7% | 7% | 31% | 46% | 62% | 88% | 94% |
| | | | | 64 | 8 | 6.8% | 6% | 26% | 41% | 57% | 82% | 88% |
| | | | | 96 | 5 | 18.9% | 4% | 15% | 27% | 46% | 66% | 72% |

Planar limit (at most 3n - 6 links can be borders): 100% for the two small graphs, 89% for
Mere's.

With the index file kept, Graphshell's graph is 82 nodes and 198 links with one node of 80
links, and the all-pinned figure at 10 places falls to 65%.

Read at Graphshell `a0469fc1`; Mere and Turnstone at their local checkouts' working trees.

## 2026-11-14: Crystal probe 2 (greedy packer, real rectangles)

A throwaway script, not in the repository. Same graphs, index files left out, and the same
breadth-first pin order as probe 1. Rules: a pinned tile does not move; pinning packs its
linked tiles against its border at one common size, the largest of 160 by 100, 128 by 80,
96 by 60, 80 by 48 and 48 by 48 that lets the most of them touch it; a border counts when it
is at least half the smallest side long (24 px at 48), 16 px between other linked tiles;
tiles that are not linked stay 8 px apart; an unpinned tile already packed elsewhere moves
only to a place touching more of its linked tiles; a linked tile with no place stays loose.
Simplifications: loose tiles are not obstacles; a nucleus never packed is dropped clear of
the structure; candidate places are tried every 16 px; the search is greedy.

Controls, same run: a star of 30 leaves gives 9 borders against a ceiling of 10, none
false; a path of 20 gives 19 of 19; a triangle 3 of 3. Every run asserted no overlaps.

Share of all links shown as borders, with probe 1's ceiling in brackets:

| Graph | Smallest side | Pinned tile | 1 pin | 5 | 10 | 25 | 50 | All | Unlinked tiles touching |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Graphshell docs (70, 118) | 48 | keeps size | 10% (8) | 17% (23) | 19% (31) | 38% (58) | 53% (89) | 64% (100) | 0 |
| | 48 | grows back | 10% (8) | 18% (23) | 19% (31) | 40% (58) | 55% (89) | 65% (100) | 0 |
| | 64 | keeps size | 10% (7) | 16% (21) | 18% (29) | 41% (54) | 57% (86) | 67% (97) | 0 |
| | 64 | grows back | 10% (7) | 16% (21) | 18% (29) | 40% (54) | 56% (86) | 66% (97) | 0 |
| Turnstone docs (74, 134) | 48 | keeps size | 10% (7) | 15% (31) | 19% (46) | 31% (62) | 42% (88) | 47% (94) | 0 |
| | 48 | grows back | 10% (7) | 18% (31) | 23% (46) | 37% (62) | 46% (88) | 51% (94) | 0 |
| | 64 | keeps size | 8% (6) | 14% (26) | 20% (41) | 32% (57) | 43% (82) | 49% (88) | 0 |
| | 64 | grows back | 8% (6) | 16% (26) | 23% (41) | 34% (57) | 46% (82) | 51% (88) | 0 |
| Mere docs (437, 1468) | 48 | keeps size | 1% (1) | 2% (3) | 2% (6) | 8% (14) | 14% (24) | 31% (73) | 0 |
| | 48 | grows back | 1% (1) | 2% (3) | 2% (6) | 8% (14) | 14% (24) | 31% (73) | 0 |
| | 64 | keeps size | 1% (1) | 2% (2) | 3% (4) | 9% (11) | 14% (20) | 32% (65) | 0 |
| | 64 | grows back | 1% (1) | 2% (2) | 3% (4) | 9% (11) | 15% (20) | 33% (65) | 0 |

A single pin can beat probe 1's ceiling by a point or two because the ceiling counted places
as perimeter over smallest side and the packer lets a tile overhang a corner.

Each run took about a second. Read at Graphshell `1b1a181c`; Mere and Turnstone at their
local checkouts' working trees.

## 2026-11-17 G8 polygon picking and link attachment

The full Cartography library suite passed on Windows and on Fedora T-PC with
`canvas,physics-engine` and `--test-threads=1`. Both runs passed 758 tests with 18 ignored.
The tested source landed as Graphshell `8624f9f5`.

Turnstone's Windows test `native_canvas_picks_and_attaches_to_polygon_silhouettes`
passed against that resolved Git revision. The fixture supplies two registry bodies to
the live canvas through `set_node_shapes`. The native shell pointer route missed the
clipped corner and selected the center on both the event diamond and the software
hexagon. Each link tip was within the test's 0.05 px tolerance of its painted outline.
The 1024 by 600 capture shows both bodies and the connecting link. The diamond clip
cuts part of the event caption.

The capture and log are in the workspace directory
`testing/turnstone/receipts/2026-11-17_m1_silhouettes/windows-git/`.
The SHA-256 of `diamond-and-hexagon.png` is
`2449f1982d7ea4e0e575def643d1e8417006b4cf54360d85534427e9f27d07bf`.

## 2026-11-17 M2 recipe mark slots

Shared implementation at Graphshell `411d961`, checked on Fedora T-PC.

- Scenograph and Scenomise passed their all-feature tests and doctests. Library
  counts were 81 and 224. The rule tests cover whole-mark and detail-only
  precedence and the winning occurrence trace.
- Cartography passed 761 library tests with 18 ignored, using
  `--features canvas,physics-engine -- --test-threads=1`.
- The pre-commit checks passed, including both workspaces' all-target Clippy
  checks, the Mere View query feature, dependency bans and patch consistency.
- JSON and TOML round trips preserve explicit mark slots. Empty slots stay absent
  from saved legacy recipes. Binding and draft promotion retain all three slots,
  and the mark edit is undoable.
- Canvas tests take a compiled recipe through body classes, picking, colliders,
  paint and material. A malformed replacement retains the active state. Whole
  live marks replace the recipe mark, and clearing the rule restores it.
- The live-rule regression exposed a material lookup that still read the recipe
  after a body override. The lookup now reads the complete winning mark.
  The same regression checks the native arrangement id `partition.default`
  against its assigned extent.
- These runs exercise the compiler and canvas rendering paths. The earlier G8
  entry holds the native headed capture.

Commands were `cargo test --manifest-path host/Cargo.toml -p scenograph -p scenomise --all-features`,
`cargo test --manifest-path host/Cargo.toml -p mere-cartography --lib --features canvas,physics-engine -- --test-threads=1`
and `prek run --hook-stage pre-commit`, with the reusable target at
`~/Code/targets/graphshell`.

Turnstone integration was checked from its `ef91de16` baseline with the Scenograph,
Scenomise and Cartography source paths overlaid for this change. The Graphshell
web-library suite passed 278 tests with 6 ignored. The full product Wasm check
passed with `product,remote,image-decode,presentation-webgpu,presentation-webgl2,physics-engine`
and `--no-default-features`. This covers the normal live-rule installer, its
capability filter and the source-shaped recipe-to-draft copy.

The default GPU-physics Wasm profile failed in registry
`cubecl-runtime-0.12.0-pre.2` at its import of `crate::tune::sampler::measured`.
The recorded Turnstone resolution already carried that same registry package.
The CPU-physics profile above keeps both presentation routes enabled.

### Final silhouette and consumer checks

Graphshell `ac71400` makes an explicit mark silhouette authoritative over
detail and attention rounding. The regression checks all five silhouettes with
both canvas grammars, including the inline radius and corner picking.
The full serial Cartography suite then passed 762 tests with 18 ignored.

Turnstone integration landed as `985a4060`. Its Graphshell web-library suite
passed 278 tests with 6 ignored against committed Graphshell Git sources.
The full product CPU-physics Wasm profile above also passed against committed
Git sources after bringing the viewer's Cue resolution to `6592694`, the
revision already qualified by the native root. The older viewer Cue revision
`10cdfe0` lacked the `SceneRasterizer::render_with` API used by current Pelt.
The default GPU-physics Wasm failure remains the CubeCL sampler import recorded
above.

Both repositories passed their configured pre-commit checks. Turnstone's root
and browser resolved locks record the shared Git sources; the integration uses
the existing floor-only manifest rows.


## 2026-11-17 M3 relation form readers

Scope: effective recipe/rule relation forms through Scenomise compilation,
arrangement, portable scene metadata, rendering and motion refresh. Adjacency
remains the Crystal phase; field-form realization and B4 recipe channel parity
remain open. Retained evidence is this compact receipt, below the 100 MB budget.

Implementation:
- Sceno owns the wire form and placement reports; Scenograph re-exports
  RelationForm. Default Stroke and empty report/order vectors preserve the old
  JSON shape.
- Both host-dataset and explained-relationship compilation resolve relation
  marks, retaining the default and winning rule trace. Matched unsupported
  forms are rejected before a custom solver is called.
- Grid, Spiral and Mosaic declare Order. Stable occurrence/source identities
  survive reordering. Mosaic retains its groups and weighted bisection.
  Partition Tiles declares Containment, subdividing a parent with the existing
  squarifier. Sibling weights already disclosed in a Score retain their ratios.
  Source group boundaries and pins remain binding. Cycles, competing parents,
  extent conflicts and pins produce explicit unmet outcomes.
- MatrixCell is a directed grid beside the node layout, with occurrence row
  and column headings. Sparse matrices retain empty axes in their bounds.
  Matrix routing does not create source occurrences. Stroke keeps the existing
  line path; cells use closed, filled paths.
- Refresh starts form changes from the unconstrained scene. Motion refresh
  retains cells and recomputes containment outcomes without moving items home.
  Scenotime snapshots and transactional diffs retain reports and slot order.

Shared checks on T-PC, reusing ~/Code/targets/graphshell:
- cargo test --manifest-path host/Cargo.toml -p scenograph -p scenomise
  -p sceno -p scenotime --all-features --offline: Scenograph 81 library and
  9 integration tests; Scenomise 235 library, 4 integration and 3 doc tests;
  Sceno 39 library and 6 doc tests; Scenotime 44 library and 2 doc tests passed.
- After adding pre-placement rule capability validation, the full Scenomise
  suite above passed again.
- cargo test --manifest-path host/Cargo.toml -p mere-cartography --lib
  --features canvas,physics-engine --offline -- --test-threads=1:
  763 passed, 18 ignored. Includes the open-stroke versus closed-filled-cell
  paint-command assertion. This is automated renderer evidence; no headed M3
  interaction is claimed.

The clean T-PC main checkout was used. The Windows Graphshell and Turnstone
primary checkouts carry concurrent work and were preserved.

- Review added an explicit configured-group-order Mosaic regression. Order
  reports now read the realized group regions, including configured group
  order; tile area remains proportional to disclosed weights with zero gutters.

- Final matrix checks put row labels outside the first cell and centre each
  column label over its column. A fully pinned Mosaic regression checks an
  already-satisfied order against configured group order, including the
  fallback path. Scenomise's full suite now passes 237 library, 4 integration
  and 3 doc tests.


### Consumer integration and final main checks

The shared M3 code ends at 00c3d89. Every consumer's recorded shared Git source
is that commit, with existing floor-only manifest rows retained.

| Consumer | Change | Validation |
| --- | --- | --- |
| Turnstone f7d21cd7 | Directed filled matrix controls and headings; placed forms do not draw a stroke; motion routes retain their form; spoken relation descriptions include constraint outcomes | 278 web-library tests passed, 6 ignored; all configured pre-commit checks passed; final shared revision passed all-target Graphshell Clippy with query-views and the full product CPU-physics Wasm profile |
| Mer3ly dd6dc45 | Portable relation literals explicitly use the default Stroke | All configured checks passed; repo-graph all-target Clippy passed against final shared Git sources |
| Knot 12b3552 | Rosette relation literals explicitly use the default Stroke | All configured checks passed against final shared Git sources |
| Woodshed fd3b60d4 | Nine hand-built stage and overview relations explicitly use Stroke | All-target Clippy, bans and patch consistency passed against final shared Git sources |
| Cleromancy 4b7dcf1 | Default Stroke on hand-built relations; PartialEq on AmbientView, AmbientContext and ConsultationAction because saved mark material includes floats | All-target Clippy, bans and patch consistency passed against final shared Git sources |

The Turnstone library command was cargo test -p graphshell --no-default-features
--features web,physics-engine --lib --offline --locked. The browser command was
cargo check --manifest-path ports/graphshell/web/Cargo.toml
--target wasm32-unknown-unknown --no-default-features
--features product,remote,image-decode,presentation-webgpu,presentation-webgl2,physics-engine
--offline --locked.

Before pushing, origin advanced with Graphshell uxtree P0 (2581ba6) and
Turnstone Vano engine work (b270652c). These were merged without conflicts as
3f80f27 and d91d1ffb. Their recorded dependency resolutions were retained, with
the Turnstone shared source advanced to 00c3d89. The merged Graphshell and
Turnstone branches passed all configured pre-commit checks, and the browser
profile above passed again. Resolved locks are recorded on the commits under
refs/notes/resolved.

Woodshed needed its recorded Wavicle Git dependency and platform crates
downloaded on T-PC. Cleromancy's guest metadata check also needed cached
dependencies fetched. These were cache misses; the completed bans and patch
checks passed.

Reusable build caches remain in ~/Code/targets/{graphshell,turnstone,mer3ly,
knot-editor,woodshed,cleromancy}, owned by their corresponding repositories.
The seven Graphshell guest fixture aliases continue to use
~/Code/targets/graphshell/guests. The task used primary T-PC checkouts, including
a new primary Mer3ly checkout; Windows concurrent work was preserved.


## 2026-11-17 M4 frame slot

Scope: recipe frame data through editing, binding, placement and portable scene
state. Retained evidence is this compact receipt plus committed tests, under
the default 100 MB receipt cap. Work uses the clean T-PC primary checkout and
the reusable ~/Code/targets/graphshell cache.

- Sceno owns Frame and FrameKind: Unbounded(origin), Bounded(bounds),
  Nested(local bounds and a Space transform), Flow(origin and page width).
  Scenograph re-exports the vocabulary. Authored, bound and draft recipes carry
  an optional frame; binding, promotion and undo preserve it. Missing frames
  retain existing geometry and remain absent from legacy recipe JSON.
- Score version 6 carries the slot. Older Scores retain their family geometry
  and hold semantics. A version-5 reader rejects version 6 rather than silently
  discard the frame. Legacy family bounds/origin fields remain compatibility
  inputs; explicit frames supply the effective geometry during placement.
- Partition and Sets declare bounded frames; Mosaic declares unbounded.
  Grid also declares Nested and Flow; Hulls declares Bounded and the other
  built-ins declare Unbounded. Custom solvers declare accepted kinds and
  implement place_in_frame. Unsupported combinations are refused before
  placement; a declaration without a reader also refuses.
- Flow caps ordinal/generated grid columns to page width and grows height.
  Authored cell coordinates must fit. Nested Grid uses a genuine Space and
  keeps authored pins in local coordinates. SceneFrame retains the domain and
  its live Space, so routing and motion refresh follow later Space movement.
  Snapshots and transactional diffs retain and validate that metadata.
- With an explicit bounded frame, legacy width/height options are refused so
  they cannot compete with the rectangle. Changing the frame invalidates
  placement reuse; unchanged frames retain it.

Shared checks:
- cargo test --manifest-path host/Cargo.toml -p sceno -p scenograph
  -p scenomise -p scenotime --all-features --offline: Sceno 41 library and
  6 doc tests; Scenograph 83 library and 9 integration tests; Scenomise
  250 library, 4 integration and 3 doc tests; Scenotime 45 library and
  2 doc tests passed.
- Full serial Cartography run: 762 passed, 18 ignored, with one expected
  score-byte golden failure caused by the writer version changing from 5
  to 6. Every position hash remained unchanged. Only four score-byte hashes
  were updated. A new migration guard serializes those scores at version 5
  and matches their preceding hashes, proving the remaining content is
  unchanged. The targeted arrangement_goldens rerun passed all 3 tests.
- All configured Graphshell pre-commit checks passed before that golden
  refresh. The commit hooks repeat the applicable checks on the final files.

These are compiler, scene and automated renderer checks. The combined headed
frame/primitive picker demonstration remains the M6 acceptance gate.

- A combined nested-frame/matrix regression checks direct relation rerouting,
  metadata-only recipe refresh, and another reroute after moving the live
  Space. Rebuilding relation placement now preserves that Space pose when
  the authored frame is unchanged. The full Scenomise suite passes 251
  library, 4 integration and 3 doc tests after this refinement.


### Consumer and merged-main qualification

The shared implementation is fbb05e2, with the nested-pose refresh refinement
in 75e391d. Turnstone 1c31de2d copies an explicit frame into the editable draft.
Its new workflow regression saves a framed recipe, changes the working frame,
reopens the snapshot, refreshes the editor and checks that both the frame and
its placement remain intact.

Turnstone passed all configured pre-commit checks. Its Graphshell library suite
passed 279 tests with 6 ignored against committed shared revision 75e391d.
The full product browser check also passed, using:
cargo check --manifest-path ports/graphshell/web/Cargo.toml
--target wasm32-unknown-unknown --no-default-features
--features product,remote,image-decode,presentation-webgpu,presentation-webgl2,physics-engine
--offline --locked.
The library command was cargo test -p graphshell --no-default-features
--features web,physics-engine --lib --offline --locked.

The other consumers passed their configured pre-commit checks against the M4
API revision fbb05e2 without source edits:

| Consumer | Checked source | Resolution baseline |
| --- | --- | --- |
| Mer3ly | dd6dc45 | recorded on the same commit |
| Knot | 12b3552 | recorded on the same commit |
| Cleromancy | ba7442a | nearest recorded ancestor 4b7dcf1 |
| Woodshed | fd3b60d4 | recorded on the same commit |

Their recorded locks were replayed with only the Graphshell Git source
advanced for qualification. After checking, the original lock bytes were
restored. Package-name/version/source comparison confirmed that no other
dependency changed; their source trees and existing resolution notes remain
unchanged. The later pose refinement changes the nested-frame implementation,
with its complete Scenomise regression suite recorded above.

While integration ran, uxtree P1 (5267dd2) landed on origin. It was merged
without source conflicts as b5ac7c2. All configured Graphshell checks passed
on that combined main. Turnstone's recorded lock notes retain its checked
shared source revision.

The task reused stable per-repository targets on T-PC. Seven Graphshell guest
target aliases share the existing ~/Code/targets/graphshell/guests fixtures.
Windows primary checkouts and concurrent work were preserved.


## 2026-11-17 M5 assigned node marks

Scenomise checks the resolved occurrence mark after presentation rules apply.
A measured default can yield to an assigned mark, and refusals name the
occurrences whose effective marks cannot fill the extent. A detail change
reuses placement for Mosaic and Partition when the other inputs match.

The regression fixtures compare the complete realized scene for the legacy
tile, an explicit square tile, a card, a snapshot and a host-defined mark.
Mosaic and both Partition forms produced identical positions, footprints,
regions and occurrence mappings. A second check changes a tile to a card
after adjusting a position and verifies that the compiler reuses that
placement. Shared code is 51f55dd and 5206d4f.

Turnstone 66716cd5 renders the resolved Card as a title and details inside
the assigned cell. The spatial preview clips ring items to their polygonal
footprints. Other detail requests retain their identity in the scene and use
a label in this preview. The headed demonstration remains part of M6.

Verification on T-PC used the existing Graphshell and Turnstone Cargo targets.
Scenomise passed 254 library, 4 integration and 3 doc tests with all features.
Cartography passed four projection mark tests and three arrangement golden
tests. Turnstone passed 279 Graphshell library tests with 6 ignored. The
full product Wasm check passed with CPU physics, WebGPU and WebGL presentation.
Both repositories passed their configured pre-commit checks. Turnstone's
root and browser resolved locks use 5206d4f.

This receipt is the retained evidence for M5. The existing reusable targets
in ~/Code/targets/graphshell and ~/Code/targets/turnstone still serve their
respective repositories. The Graphshell checkout in Cargo uses the existing
guest fixtures in ~/Code/targets/graphshell/guests.

## 2026-11-17 M6 compatibility and swatch axes

Graphshell 5409300 gathers the extent, placed relation forms and accepted
frames for the arrangement's current options. A registered solver uses the
same lookup and supplies positions for measured items. Frame validation and
relation routing consult that lookup. A typed swatch pick changes the selected
recipe slot as one history entry. A bounded-frame pick also clears the legacy
width and height, and undo restores both.

Turnstone d413ad7f reuses the comparison grid and its whole and selected
scopes for node marks, relation forms, field forms and frames. It checks each
choice, compiles supported recipes before offering them and lists the reasons
for incompatibilities. The
relationship preview reads the host's disclosed relationships. The field
preview reads the arrangement's regions. Viewport stores the preview
rectangle at the time of the pick.

The headed Chrome run on T-PC used the embedded product tree with `app=local`
and the BrowserWebGpu presenter. The fixture
`ports/graphshell/web/tests/projection-swatches.html` opens Partition on the
existing host relationship dataset. The `projection_swatches` scenario picks
Card, Viewport, Matrix cell and Contour. It checks the resolved slots and
their undo steps, redoes Card and Viewport, and verifies that the instantiated
viewport contains all four card footprints. The two captures show the four
cards and the complete outline of the viewport around them.

The scenario completed with zero page errors and zero gate failures. The
retained receipt is at
`~/Code/testing/turnstone/receipts/2026-11-17_m6_swatches-main` on T-PC. It
contains `partition_cards.png`, `partition_viewport.png`, the whole-frame
capture, the accessibility tree and the machine-readable results.
The scenario SHA-256 is
`fa65d31c89bb1e47568ccfe529cd58f56271fc6ac5e75a87299336400ee41a98`.
The Wasm bundle SHA-256 is
`d4e264757e6f9cd60ed4ff67bef3f9d2d6a008f16ee1c299fbe33e1d98767fba`.

Build from the `ports/graphshell/web` workspace so Cargo loads its Wasm
stack settings. This run set `CARGO_PROFILE_DEV_DEBUG=0` and used
`wasm-bindgen` 0.2.129.

```sh
cargo build --target wasm32-unknown-unknown --no-default-features \
  --features product,remote,image-decode,presentation-webgpu,presentation-webgl2,physics-engine \
  --offline --locked
```

Stage the web assets alongside the bindgen `pkg` directory. Pass the staged
site to `tests/run-headed-receipt.py` with `--scenario projection_swatches`,
`--page tests/projection-swatches.html`, `--query app=local`, `--unsafe-webgpu`,
`--chrome-arg=--enable-features=Vulkan` and `--chrome-arg=--use-angle=vulkan`.

Scenograph and Scenomise passed 355 tests with all features on the checked
Graphshell revision 7397ad9, which integrates the concurrent uxtree P2 update.
Turnstone passed 281 library tests with six ignored using
`cargo test -p graphshell --no-default-features --features web,physics-engine
--lib --offline --locked`.
Turnstone uses the checked Graphshell revision in both the root and web resolved locks,
on top of the dependency resolution recorded for its canvas envelope update
at 7406f9f3. The runs reused the T-PC targets in
`~/Code/targets/graphshell` and `~/Code/targets/turnstone` and the existing
guest targets under `~/Code/targets/graphshell/guests`. The headed site and
isolated browser profiles were removed after capture.

## 2026-11-18 Z Partition zoom

Graphshell f337e02 reads a focus group when it solves a zoom-enabled
Partition. Each other measured share gives half to the focused group.
Members keep their relative weights, and the unaccounted share retains
its measured size. The focus changes the view and stays outside the
recipe, score and saved snapshot. Placement reuse checks the focus as
well as the score. An absent group or disabled zoom yields the whole.

Turnstone 3e3d7c43 exposes the recipe's Partition zoom toggle in
Interaction and the group controls in the spatial preview. The preview reports
the shares from the realized area or angle. It draws the remainder as
a distinct region. Zoom out returns to the unfocused placement.

Both headed Chrome runs on T-PC used the BrowserWebGpu presenter with
`app=local`. The `projection_partition_zoom` scenario opens the fixture
`ports/graphshell/web/tests/projection-partition-zoom.html` with `form=tiles`
and `form=rings`. It enables zoom, focuses Alpha and then Beta, and
zooms out. The group and remainder shares move from 40/40/20 to 60/20/20
and 20/60/20, then return to 40/40/20. Each view uses the same
760 × 340 bounds. The run also checks that selecting a card retains the
focus and that the next recipe undo undoes enabling zoom.

Both receipts record zero page errors and zero gate failures. The
four inspected captures are `whole.png` and `focused.png` under `tiles`
and `rings` in
`~/Code/testing/turnstone/receipts/2026-11-18_partition_zoom` on T-PC.
The receipt directories also contain the whole-frame and accessibility
captures. The scenario SHA-256 is
`17e1523ffc5bb4de7f80d434e1289c687399ae153b8d9a82bd56c1ed5de935dc`.
The Wasm bundle SHA-256 is
`02ec6b29c55bbd2a669880b17dfde21e3134eb846e47c9ff4fb29d7283d32b02`.

The build used the same product Wasm features and web workspace settings
as the M6 receipt above. Run `tests/run-headed-receipt.py` with
`--scenario projection_partition_zoom` and
`--page tests/projection-partition-zoom.html`. Use
`--query 'app=local&form=tiles'` and `--query 'app=local&form=rings'`
with separate receipt directories.

Scenograph and Scenomise passed 357 tests with all features, both on
f337e02 and after 8c481b4 merged the concurrent uxtree P4 update.
Turnstone passed 281 library tests with six ignored using
`cargo test -p graphshell --no-default-features --features web,physics-engine
--lib --offline --locked`. Its root and web lock notes use f337e02. The runs reused the
Graphshell and Turnstone targets on T-PC. The staged browser site
and isolated browser profiles were removed after the headed check.

## 2026-11-18 N Crystal

Graphshell eacce17 adds the unbounded Crystal arrangement, its assigned
rectangular tiles, and adjacency as a placed relation form. The recipe's
`smallest_side` defaults to 32 px. A shared border counts at half that
length. The packer searches perimeter boundary coordinates at the minimum
size with a bounded beam, then grows placed tiles where the borders and
clearance allow. A pin only repacks its immediate unpinned links. Other
tiles, including previous pins and loose tiles, are obstacles. Unlinked
tiles keep 8 px apart.

The 30-leaf star at a 160 × 100 nucleus and 48 px minimum fits ten leaves.
Probe 2 fit nine for that control. The document replay reconstructed the
70 vertices incident to the 118 links in Graphshell a0469fc1. It parks
160 × 100 tiles 512 px apart in ten columns, visits each component in
breadth-first order from its highest degree, and checks clearance and tile
size after each pin. Its border counts were 17, 20, 27, 42, 61 and 74 after
1, 5, 10, 25, 50 and 70 pins respectively. The all-pinned share was 63%.
Probe 2 reported 65% in its grows-back run. Its free-tile and starting
positions differ from this replay. The graph and counts are in
`packer-graph.txt` and `packer-replay.json` in the receipt directory.

Turnstone e5feff14 wires the live edits into the spatial preview. Pinning
uses the scene's honored pin role. Moving an attached tile lets the packer
choose a feasible position on a border and repack the siblings. Scene
placements survive compiler refresh and unpinning. Each disclosed link
has a shared border or an unmet report, with a fallback stroke for
selected endpoints.

The headed Chrome run on T-PC used BrowserWebGpu. It loaded
`ports/graphshell/web/tests/projection-crystal.html` through the normal
product route with `app=local`. The `projection_crystal` scenario used
keyboard drop and slide input. The 96 px minimum fixture reports ten
links. The first pin fits six direct links and leaves three direct links
loose at their starting positions. The second-hop tile stays put. The
slide moves Branch from (82, 96) to (-130, 48) and its siblings repack
around the fixed nucleus. Unpinning preserves those coordinates.
Clearing selection removes the link overlays.

The final run passed with zero page errors and zero gate failures. The
three inspected captures are `crystal_pinned.png`, `crystal_slid.png`
and `crystal_unpinned.png` in
`~/Code/testing/turnstone/receipts/2026-11-18_crystal/headed-final` on T-PC.
This run used the M6 product Wasm features and web workspace configuration.
It used `tests/run-headed-receipt.py --scenario projection_crystal
--page tests/projection-crystal.html --query app=local`, with
`--force-device-scale-factor=0.8` passed to Chrome.
The scenario SHA-256 is
`2abc789328d34856810dbe9cb86d5f3510a6f49a2143566b57c7cb4fa4b914c6`.
The Wasm bundle SHA-256 is
`e435f98015ae07c2fc362b5b003e05deae20889d3bbb626e4d64ffd9e480194c`.

Scenograph and Scenomise passed 361 tests with all features. Turnstone
passed 281 library tests with six ignored using
`cargo test -p graphshell --no-default-features --features web,physics-engine
--lib --offline --locked`. Both repositories passed their configured
checks after integrating uxtree P5. Turnstone's resolved root and web
locks use the checked Graphshell merge ac91ed3 and Genet b75face for
the semantic lookup APIs. Stable per-repository and guest targets were
reused on T-PC. The generated site and browser profiles were removed
after capture.


## 2026-11-18 X context scenes

Graphshell 4c52c05 adds context declarations to recipe marks and a
presentation effect that can replace the declarations for a matching
occurrence. Each declaration selects the trigger, subject, input binding,
child recipe and placement. Scenomise binds each disclosed input to that
child recipe and uses the regular projection compiler. The command
dataset carries ordered identities, labels, shortcuts and availability.
Invocation stays with the authority that disclosed each command.

Turnstone f6790128 uses the same child renderer for linked rows, a
subject's resource, and an ordered menu. Its Interaction panel edits the
context recipes as TOML. Apply enters the recipe's undo history as one
step. Saving and reopening uses the normal executable projection record.
The editor flags subject kinds with hover-only triggers. The workspace
also reads committed, enabled presentation rules against disclosed facts.

Both final runs passed on T-PC in headed Chrome with zero page errors
and zero receipt gate failures. The first run used retained input to
open the same node's linked items beside it, address in a pane, and
commands in an overlay. It also showed a field summary in place on
hover, exercised secondary press and a stationary held primary press,
invoked a viewer command, and checked undo, save, and reopen. The
second run offered a real, verified capsule component's WIT
`actions.emit` action in the menu. Choosing it opened reading 2 through
the applet's existing scope and navigation grant. The command disappeared
after revocation. Remote advertised action and block claim bindings are
covered by the product build. The headed held press exercises the
retained gesture path. Physical touch remains unverified.

The receipts and inspected captures are in
`~/Code/testing/turnstone/receipts/2026-11-18_context_scenes` on T-PC.
The final directories are `headed-qualified` and `headed-guest-qualified`.
The signed component and derivation are in `native-capsule/site`. The
component BLAKE3 is
`300d7a2b58a8905d311aaf391978fa7878cc5e364eccd8078eae0507896f0a73`.
The component used Rust 1.97.1 for the native WASI runner's import version.

Both browser runs used the locked product build from
`ports/graphshell/web` with
`product,remote,image-decode,presentation-webgpu,presentation-webgl2,physics-engine`.
The receipt harness loaded `tests/projection-contexts.html` and the
scenarios `projection_contexts` and `projection_context_guest`.
The shared bundle SHA-256 is
`c43792327c260b3406f85ce99d8d1e4f8b30146acc78450fba081f99d434b48b`.

Scenograph and Scenomise passed 364 tests with all features. Turnstone
passed 281 library tests with six ignored under
`--no-default-features --features web,physics-engine`. The configured
repository checks passed. Turnstone's resolved root and web locks pin
the checked Graphshell commit 4c52c05. The staged web bundle, headed
profiles, and one-off fixture dependencies were removed after capture.
