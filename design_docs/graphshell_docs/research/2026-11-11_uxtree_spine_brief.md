# uxtree as the accessibility, diagnostics and automation spine: brief

**Date:** 2026-11-11
**Status (2026-11-11):** research brief, complete. Docs only: no code changed, nothing built or run. The forks in
§8 wait for Mark. No decisions are taken here and none is written as one. Followed by the
[federated tree brief](2026-11-11_uxtree_federated_tree_brief.md) (same day), which reads the producers through, maps
these nine forks onto six (its §6.7) and corrects two readings here: `stitch` concatenates subtrees and does not
rehash them, and the canvas outputs Cambium producer semantics, not AccessKit.
**Ruling carried:** engine split D53 (`mere/design_docs/mere_docs/implementation_strategy/2026-11-07_engine_split_plan.md`,
Mark, 2026-11-09). Asked what to do with `uxtree`, Mark asked back: "Is there more that can be done with it to
support accessibility, diagnostics, and automation stack wise?" The ruling, "Grow into the spine (Recommended)",
keeps `uxtree` whole and asks for this brief, with no code now, covering four directions: graft joins the Cambium
and taproot subtrees into one ID space; taproot scenarios address native surfaces by uxtree path; a lint pass
(unlabelled, orphaned and unstable IDs) asserts through `mere-registry`'s diagnostics; Turnstone's generic a11y
pieces are audited for promotion.
**Audit base (all read from `origin/main` with `git show` and `git grep`):** mere `446750cf9`, graphshell `7e34fec00`,
turnstone `fd57dd3f2`, genet `7c6ffc87b`, knot-editor `424bf77ad`, woodshed `de16948ec`, cleromancy `2d02f6569`,
isocosm `a1acb8045`. No network was used (§6).
**Paths.** Code paths are Mere's as of the commit read. `uxtree` is still at `crates/forme/uxtree` in mere.git. It
moves to Graphshell's `host/` at the cutover now being rehearsed, and these paths will then change. Paths are cited
as `<repo>.git <path>` or through the key below.
**Path key.** `uxtree/` is mere.git `crates/forme/uxtree/`. `gs/ux-events/` is graphshell.git
`host/crates/system/ux-events/`. `gs/hest/` is graphshell.git `crates/hest/`. `gs/pelt/` is graphshell.git
`host/ports/pelt/desktop/`. `taproot/` is genet.git `components/taproot/`. `turnstone/` is turnstone.git `src/`.
Other Mere documents are cited as `mere/design_docs/...` (DOC_POLICY core §5).
**Related:** [app composition brief](../../cambium_docs/research/2026-10-06_app_composition_brief.md) (AC2, the graft
join), [diagnostics and accessibility plan](../implementation_strategy/2026-10-28_diagnostics_and_accessibility_plan.md),
`mere/design_docs/2026-10-06_data_formats_brief.md`.

*Reading, not ruled* marks this brief's own inference. Every other claim is a quotation, a file, a measured figure or
a count. Line counts are physical lines. Cost figures in §7 are estimates from the measured neighbours, not
measurements.

## 0. The result on one page

- **What uxtree is.** A 1,236-line crate (2,191 with tests) that builds AccessKit nodes with path-hashed ids, joins
  trees two ways (`stitch` hashes them into one id space; `graft` keeps each tree's own ids behind a `TreeId`), and
  projects a document and a frozen scene. Four crates name it (platen, mere-view's `gloss`, and two as dev
  dependencies) plus Turnstone. Cambium, Genet, Pelt, the canvas, Knot, Woodshed, Cleromancy and Isocosm do not.
- **The common tree is AccessKit's, not uxtree's.** Every accessibility producer in the stack ends in an AccessKit
  `TreeUpdate`. uxtree sits under three of the six producers that build one (§3, F1).
- **Four identity schemes are live** for "which UI thing is this": path-hashed ids (uxtree), minted ids bound to a
  producer identity (Cambium's native publisher), engine-local u64s (Genet), and minted keys with lifetime witnesses
  (the canvas). None derives from another (F2).
- **Automation does not use uxtree at all.** 468 scenario files across six repositories drive apps through taproot
  selectors (role, name, class, data attribute) and palette labels. The grammar has no path or id selector. The tree
  keeps no path, because the hash is one-way (F3, F8).
- **Diagnostics has no per-node identity.** `ux-events` reasons over a 20-variant surface enum, `mere-registry` over
  channel ids. The registry declares `ux:structural_violation` and `ux:contract_warning`; nothing emits them (F5, F7).
- **Three of D53's premises do not match the tree** (§1): `frozen` does not catch unlabelled nodes, taproot builds no
  tree, and graft keeps separate id spaces by design. They shape forks 1, 2 and 6.
- **Nine forks** (§8). The first asks which candidate shape (§7) the work takes; the rest are narrower.

## 1. What D53 asks, and what the tree shows

D53's four directions, each beside what was measured. This is not a criticism of the ruling: the facts were read for
this brief and not before it.

| D53 premise | Measured today | Where |
|---|---|---|
| uxtree projects "documents and workbenches" with stable path-derived ids | Documents yes. The workbench projection (`project_tile_layout`, 139 lines) lives in platen, not uxtree. Ids are path hashes. Stable within a run; across Rust releases only by accident (F4). | `uxtree/src/lib.rs:150-200`; mere.git `crates/platen/platen/src/accessibility.rs` |
| `graft` "joins independent trees" | Yes, and "independent" is the point: each guest keeps its own `NodeId`s under a `TreeId`. The one hashed id space is `stitch`, which graft was built to replace for composed sessions. | `uxtree/src/graft.rs:7-25` |
| `frozen` "catches unlabelled nodes" | `frozen` (151 lines) projects a `FrozenScene` and labels every node it emits. The catching is a test-local `traverse` helper in `graphshell-client-tests`. No library check exists. | `uxtree/src/frozen.rs`; graphshell.git `host/crates/graphshell/graphshell-client-tests/tests/frozen.rs:471` |
| Turnstone's a11y layer is "25 files, 1.3k lines" | 26 files name the word `uxtree`. The a11y family is 55 files: 4,222 non-test and 2,906 test lines. The 1.3k figure does not reproduce. | turnstone.git `src`, `git grep -w uxtree` and the `a11y` name filter |
| "Cambium (sprigging, rootstock) and Genet's taproot build their own AccessKit trees" | Rootstock and `cambium-winit-a11y` publish and remap trees; `genet-render` builds them from the DOM (1,880 lines). Sprigging only fills a node through a `Leaf::accessibility` hook. Taproot builds none: it reads a tree (`AccessKitLookup`, `DocumentLookup`). | `taproot/lookup.rs`; genet.git `components/genet-render/src/a11y.rs` |
| Taproot "formerly named genet-probe" | Correct. The manifest says "Named genet-probe until 2026-09-14". | `taproot/Cargo.toml` |
| A lint "asserts through `mere-registry`'s diagnostics" | The registry declares the channels. Nothing in Mere, Graphshell or Turnstone emits them (F5). | mere.git `crates/system/registry/src/diagnostics/channels.rs:300-311` |
| Scenarios "address native surfaces by uxtree path" | None does today, and the tree cannot answer a path (F3). | §3, F3 |

## 2. What uxtree is today (measured)

**Size.** `lib.rs` 535, `graft.rs` 550, `frozen.rs` 151: 1,236 source lines. Tests: `src/tests.rs` 435 (13 tests),
`tests/e1a_subtrees.rs` 520 (9 tests), 7 more inline in `graft.rs`: 29 tests, 2,191 Rust lines in all. README 89 lines.
Version 0.0.2, stage "pre-alpha".

**Features and dependencies.** Default features none. `frozen` (optional) pulls `graphshell-client`. Dependencies:
`accesskit` 0.25.1 (workspace pin), `inker`, `tracing`. Dev: `nematic` (Micron probe fixtures, since copied into the
crate), `accesskit_consumer` `=0.39.1`. The README still says AccessKit 0.24 and names consumers 0.35, 0.36 and 0.38:
stale against the manifest.

**What it models.**

- `UxTree { root, nodes: Vec<(NodeId, Node)> }`: a flat list, descendants first, root last. No parent links, no
  bounds, no path. `to_tree_update(focus)` wraps it as a root-tree `TreeUpdate`.
- `node_id_for_path(&str) -> NodeId`: `DefaultHasher` over the string, wrapped as a u64. This is the whole identity
  scheme. Nothing records which paths exist or checks two paths for a collision.
- `stitch(root_path, root, subtrees)`: hashes every subtree into one id space under a new root.
- `graft` (AC2, ruled 2026-10-07): `tree_id_for_path`, `graft_node`, `as_subtree`, `Composition::new` (seven checks
  that stop the AccessKit consumer panicking, as `GraftError`), `Grafts` (orders one frame's updates so focus into a
  new guest is one move), `GraftTable<K>` (routes an `ActionRequest` by `target_tree` to the owning session).
- `project_document` and `project_document_with_folds`: an `inker::EngineDocument` to a `Role::Document` tree. Paths
  are `engine:{address}#blocks/{i}`, then `/quote/{i}`, `/list/{i}/item/{j}`, `/paragraph-link/{n}`, `/menu/{i}`.
  Block indices survive folds, so ids do not move when a fold toggles.
- `frozen::FrozenUxTree`: a `FrozenScene` to a tree, ids `{path}/instance/{n}/{adapter}/{id}`.

**What it does not own** (its own header): bounds, event synthesis, host coupling. It does not publish to the OS. It
has no lint, no snapshot form, no query or lookup API, and no reverse map from id to path.

**Users, by repository** (`git grep -i -E "uxtree|ux_tree"` on `*.rs` and `*.toml`, then read):

| Repository | Users | Use |
|---|---|---|
| mere | `platen` (`project_tile_layout`), `mere-view` feature `gloss` (`project_outline`, `gloss.rs:255`), `cambium-winit-a11y` (dev dependency: `examples/e1b_two_sessions.rs`, `tests/e1a_sessions.rs`) | build, graft |
| graphshell | `graphshell-client-tests` (dev dependency, `tests/frozen.rs`, 11 mentions) | frozen |
| turnstone | 35 files, 93 mentions, 69 `node_id_for_path` sites, 28 `UxTree { .. }` constructions, `stitch` in the app projection, graft in `contributed_a11y.rs` and `foreign_a11y.rs` | build, stitch, graft |
| genet | one comment, `components/shared/document-session-api/src/a11y.rs` | none |
| knot-editor, woodshed, cleromancy, isocosm | none | none |

The Mere count of 67 mentions outside uxtree includes `incipit/src/forme/ux.rs` (32), which is a parallel type
and not a user (F6), and manifests and ledgers. Genet cannot name uxtree: uxtree depends on `inker`, and `inker`
takes the engine-facing half that lives in Genet's `document-session-api` (read from the manifest comment; the edge list was not walked). *Reading, not
ruled:* anything Genet's taproot or `genet-render` needs from a shared spine has to be a type Genet owns, or the
dependency edge has to move.

## 3. The three neighbours today

One table. "Relation" is to uxtree: **built on** (calls it), **parallel** (does a similar job separately) or
**unrelated**. "Stable" says what the identity survives: *frame* (rebuilt each frame, same id), *session* (process
lifetime), *edit* (a document edit), *run* (another process).

### Accessibility

| Producer | Tree and address | Stable | Relation | Size (lines) |
|---|---|---|---|---|
| Turnstone app projection | AccessKit forest: chrome, panes, frozen via `stitch`; contributed and foreign DOMs via `graft`. Ids are path hashes (`turnstone/chrome/omnibar`, `turnstone/doc/{node}`, `frisket/{n}`). A route table `HashMap<NodeId, A11yRoute>` sits beside the tree. Pushed to the OS through `genet_winit_host::AccessKitBridge`. | frame, session; run if the hash holds | built on | 55 files: 4,222 + 2,906 test |
| `gloss` outline (mere-view) | `gloss/outline/{address}/heading/{block index}` | frame; edit: no (index) | built on | `gloss.rs` 458 |
| platen tile layout | `tile-layout/slot/{i}/tab/{member}`; the root path is a constant | frame; slot reorder moves ids | built on | 139 |
| Cambium native (rootstock, `cambium-winit-a11y`) | Tree from the Genet DOM, leaf hooks and producers. `NativeIds::prepare` remaps every node to a process-wide minted id bound to a producer identity, after checking duplicates, cycles, dangling and unreachable nodes. | session; ids never recycled | parallel (uxtree is a dev dependency only) | 1,022 + 1,593 src; 1,926 + 585 + 615 tests |
| Genet DOM projection | `DocumentA11yProjection` from `dom.opaque_id(node)`; `DocumentA11yNodeId` is "document-local" and "never an AccessKit node id", yet `genet-render` wraps it as one | session (engine-live) | unrelated | `genet-render` 1,880; `genet-winit-host` 1,289; `document-session-api` 472 |
| Pelt composite tree | One workspace tree, "namespaced child subtree" per document lane | session | unrelated (0 mentions) | `gs/pelt/workspace_viewer/accessibility.rs` 1,247 + `receipts/a11y.rs` 1,149 |
| Canvas reader | `DescribedKeyRegistry`: minted u64 keys with kernel lifetime witnesses, at most 200 items | session; reused slots never reuse a key | unrelated | `reader.rs` (cartography) |
| Chrome descriptors | `graph-kernel` `accessibility::descriptor_for(SurfaceId)`: 20 surfaces, static role and label | static | parallel; no caller | 376 |
| Forme UX nodes | `incipit::forme::ux` `UxNodeDescriptor`, ids `tree-item-{member:?}`; the donor `ux_tree_and_probe_spec` shape | none kept | parallel; no caller | 458 |

### Diagnostics

| Producer | Tree and address | Stable | Relation | Size (lines) |
|---|---|---|---|---|
| `ux-events` | Events, not a tree. `SurfaceId` (20-variant enum), `hest::ActionId`, `NodeKey`. Channel ids `ux.<surface>.opened`. Four probes assert over event order. | by type | parallel; no dependents (kept by D39) | 2,183 in 13 files |
| `mere-registry` diagnostics and observations | Channel ids `ux:*` (39 of 254 constants), run, source and operation correlation, bounded history. No per-node identity. | by channel id | unrelated | `diagnostics` 4,346 (D26) + `observations.rs` 182 |
| Publication observations | `A11yPublicationOutcome`: fixed-code, serialisable, "certify neither reader delivery nor actor work" | per publication | unrelated | `a11y_publication.rs` 111 |
| Turnstone swatch inspection | Projects `apparatus_core::Inspection` lines into a uxtree (`apparatus/inspection`, id from `line.id`). The one diagnostics-to-uxtree bridge in the family. | line id | built on | `swatch_pane/inspection.rs` |
| App inspections | Field, link and access inspection (228, 773, 281): domain ids (`Uuid`, `FieldId`, statement handles). They inspect the graph, not the UI. | domain | unrelated | in `ports/graphshell/src` |
| Content report | Genet `ContentReport`, `OutlineEntry { depth, role, name }`: no id | none | unrelated | `inspect.rs` 162 |

### Automation

| Producer | Tree and address | Stable | Relation | Size (lines) |
|---|---|---|---|---|
| taproot selectors | `Selector`: class token or role, plus a name substring, plus a `data-*` attribute filter, plus a surface scope. Resolves to a window point from DOM geometry or from an AccessKit tree or projection (`AccessKitLookup`). | by copy and class names; not by id | reads the same trees; no uxtree | 2,820 (4 files) |
| `act <label>` | Palette label matched against `App::available_actions()`, exact then prefix | by UI copy | unrelated | `shell_adapter_snapshot.rs:391` |
| `assert a11y <text>` | Substring of a rendered line `role: label = value` from `forest_lines`, which walks the grafted forest as the OS sees it. No id or path in any line. | by text | reads the tree | `forest_lines.rs` 53 |
| mandrake | Ticks taproot scenarios per frame. Restates the selector grammar in `wheel::parse_selector` (an open defect, M9 of the diagnostics plan). | per taproot | unrelated | 6,154 |
| `hest` | `namespace:name` ids (`node:new`, `nav:back`): 44 typed `ActionId`s, 1,359 lines in six files. Turnstone's own `Action` enum (about 190 variants by a line count) does not use them. | by id | unrelated | 1,359 |
| `graphshell-mcp` | `mere://<route>/<projection>`, an instance, an intent, an observed epoch and revision. Domain data, not UI. Moves to Turnstone under D23. | per route revision | unrelated | 948 src, 1,673 in all |

Scenario files, counted: Turnstone 181, Woodshed 149, Cleromancy 76, Knot 56, Pelt 5, Isocosm 1 (468). Turnstone's
verbs: `assert` 1,654, `settle` 1,329, `act` 576, `capture` 474, `click` 209 (176 outside the tabard port), `click-at` (raw pixel coordinates) 54,
`click-row` 34. 37 lines are `assert a11y`, in 18 files.

## 4. Findings

Each finding names its files. Numbers are from the audit base.

**F1. The shared tree is AccessKit's.** Six producers (§3: Turnstone, `gloss`, platen, Cambium, Genet, Pelt) end in an
AccessKit `TreeUpdate`; three build it through uxtree, three do not. uxtree adds a node-list carrier, a hash, two joins and three projections. The two reads that matter for
automation already read the AccessKit form: `AccessKitLookup` in taproot, `forest_lines` in Turnstone. *Reading, not
ruled:* "the single semantic tree" already exists and is not uxtree's to define; what uxtree could add is what AccessKit
lacks (an address, provenance, a check).

**F2. Four identity schemes.** Path-hash (`node_id_for_path`, `uxtree/src/lib.rs:98-102`); minted and bound
(`cambium-winit-a11y/src/publication.rs:41-123`, a process-wide counter, an entry kept only while its producer identity
matches); engine-local (`DocumentA11yNodeId`, genet.git `document-session-api/src/a11y.rs:50-60`, with the doc comment
that hosts "must namespace it"); minted with witnesses (the canvas). The Cambium publisher remaps whatever id it is
handed, so path-hash ids from uxtree never reach the OS on that path. Turnstone publishes path hashes straight through
its own bridge. Two publication paths, two policies, in the same stack.

**F3. A path cannot be recovered from a tree.** `Node` carries no path and the hash is one-way. Turnstone's `a11y.rs`
says so and builds a side table: "routing is a table built beside the tree rather than parsed back out of it". A
scenario could still hash a path it writes and find that id in the tree, but only for trees whose ids are path hashes:
not Cambium's, not Genet's, not Pelt's. A path vocabulary also does not exist: no list of path roots is kept. The ones
in Turnstone alone: `turnstone/chrome`, `turnstone/doc`, `turnstone/gloss/{n}`, `turnstone/frozen-projection`,
`turnstone/arrange`, `frisket/{n}`, `apparatus/inspection`.

**F4. Stability is weaker than the README says.** (a) `DefaultHasher` is documented by std as unspecified across
releases. The repo pins Rust 1.97.1, so today it is stable in practice; a toolchain bump could move every id. (b) Paths
are positional: a block index, a `slot/{i}`, `paragraph-link/{n}`. Inserting a block moves every later id; reordering
slots moves tab ids. Member-keyed paths (`tab/{member}`) do not. (c) `project_tile_layout` roots at the constant
`tile-layout`, so two layouts in one window would collide, and nothing checks. (d) `stitch` has no duplicate check at
all; `Composition::new` checks graft rules, not node uniqueness. (e) Nothing tests stability across a toolchain or
across sessions: `ids_are_deterministic` (`tests.rs:35`) compares two runs in one process.

**F5. Three checkers, none shared, none reporting.** `GraftError` (seven graft-rule variants). `NativeIds::prepare`
(duplicate id, repeated child or cycle, dangling reference, unreachable node, missing identity; a typed `Refusal` and
the serialisable `A11yPublicationRefusal` codes). The test-only `traverse` in `graphshell-client-tests`. Not checked
anywhere at runtime: an unlabelled node, a node with an action and no route, missing bounds, an id that moved between
frames. The June 2026 plan asked for exactly this audit ("missing label, missing bounds, duplicate id, invisible
focused node, action without command route", D5); no record shows it landing before the Meerkat host retired. The registry has
the channels (`ux:structural_violation` Error, `ux:contract_warning` Warn, `ux:presentation_bounds_missing`,
`ux:tree_build`, `ux:tree_snapshot_built`), and `git grep` over Mere, Graphshell and Turnstone finds no emitter.

**F6. A parked chain of about 3,000 lines.** `graph-kernel` `accessibility.rs` (376: `SurfaceId`, `descriptor_for`;
its doc says "Today no host calls this"), `ux-events` (2,183, no dependents), `incipit::forme::ux` (458,
`emit_ux_tree`, no caller in any of three repositories). `SurfaceId` is used only by `ux-events`. D39 keeps
`ux-events` as a host leaf because a missing dependent is not evidence of dead code. `UxEvent::SurfaceOpened` carries a
surface kind and no instance; the `SurfaceId` doc comment says per-pane identity is "queued".

**F7. Two channel vocabularies.** `ux-events` emits `ux.omnibar.opened` and its kin (dots, `event_channel`); the
registry's 39 `ux:` ids use a colon and share none of them. The README says a host "pre-registers" the ids through a
sink, so this is by design. It also means no registry invariant (`started` then `succeeded`) covers a UI event today.

**F8. Automation is coupled to copy and class names, not to the tree's ids.** Of 176 `click` lines in Turnstone's
`scenarios/`: 109 use a class token, 67 a role, 71 a `data-` filter (the groups overlap), 126 a surface scope. 54
`click-at` lines use pixel coordinates. 576 `act` lines name a palette label, so renaming a label breaks them. The
one durable-identity device in use is the app putting a `data-key` or `data-tabkey` in the DOM (`click surface:frame
.frisket-tab @data-tabkey=0`): an authored key, not a derived one. Mandrake restates the grammar rather than sharing
it.

**F9. Three action vocabularies, no node-to-action link in the tree.** `hest` (44 ids), Turnstone's `Action`, and the
MCP route intents (`mere.session.apply`, `mere.notes.append`). A tree node's action is reached through Turnstone's own
`A11yRoute` table. `ux-events` takes `hest::ActionId`; Turnstone does not.

**F10. Hard edges already ruled.** The remote projection plan (2026-07-22, §4.3 and the promotion table) refused
uxtree as a wire contract ("it depends on Inker and projects browser documents. A neutral semantic payload must be
proven by two applications before extraction into a shared crate") and named Meristem as a possible home for a neutral
core. The diagnostics plan forbids reusing a node id to retarget a stale action (§5). The data-formats brief says
program-written files are JSON and identity is never source text (§5).

## 5. Prior art in the family's documents

Searched: Mere's `design_docs` (active and archive) and Graphshell's, for uxtree, UX tree, accessibility tree, probe,
semantics tree, automation. Dates are the document's own. "Historical" marks the retired Meerkat host.

| Document | Date | What it decided or proposed |
|---|---|---|
| `mere/design_docs/archive_docs/2026-06-09_completed_plans/2026-05-27_adoption_roadmap.md` | 2026-05-27 | Harvested `ux_tree_and_probe_spec` (a three-layer `UxNode`: semantic, presentation, trace; per-frame snapshot rebuilt, not incremental) as doc-mapped, not code. Source of `incipit::forme::ux`. |
| `mere/design_docs/archive_docs/2026-06-09_pivot_superseded/2026-05-15_spatial_chrome_ir_brief.md` | 2026-05-15 | Exploratory. Offered a wrapper that recovers structure from path-keyed ids, or a second projection per crate. Decided nothing. |
| `mere/design_docs/archive_docs/2026-10-28/2026-06-08_system_diagnostics_and_accessibility_plan.md` (historical tail, lines 1915-2230) | 2026-06-08 | One observability spine: ux-events, registry diagnostics, uxtree, probes, an agent harness. D5 (tree audit) and D7 (typed `AgentObservation` and `AgentAction`) proposed for Meerkat. D1 to D3 landed there and D4 as a seed; nothing shows D5 to D7 landing. |
| `mere/design_docs/archive_docs/2026-10-06_completed_plans/2026-07-07_headed_automation_plan.md` | 2026-07-07 | Two automation layers (in-process harness; OS synthetic input) and one scenario vocabulary. Landed in Meerkat, retired with it; scenario work continues in Turnstone's `.scn` runner, taproot and mandrake. |
| `mere/design_docs/archive_docs/2026-10-06_completed_plans/2026-07-22_graphshell_remote_projection_host_plan.md` | 2026-07-22 | uxtree not a wire contract; keep it in Mere "for now"; prove a neutral payload in two applications first. Superseded in place by the cutover. |
| `mere/design_docs/archive_docs/2026-10-28/2026-09-15_micron_navigation_plan.md` | 2026-09-16 | P1 carries accessibility facts in uxtree only; no live session projection until an app consumes one. Fold state changes expanded and actions, not ids. |
| `cambium_docs/research/2026-10-06_app_composition_brief.md` | 2026-10-07 | AC2 ruled: retained sessions join as AccessKit subtrees through one Mere helper (`uxtree::graft`). E1a (tree half) and E1b (through winit) built; focus ordering found and fixed. |
| `mere/design_docs/archive_docs/2026-10-28/2026-10-16_full_source_audit_plan.md` | 2026-10-16 | Notes uxtree is separate from parked Forme descriptors and that visual and semantic projections "remain distinct implementations". |
| `graphshell_docs/implementation_strategy/2026-10-28_diagnostics_and_accessibility_plan.md` | 2026-10-28 to 30 | Live plan. Owners: Genet computes DOM accessibility semantics; hosts own surface mappings; taproot selects and drives; the scenario lane schedules; registry owns channel descriptors. M1, M4, M5 (target lives, publication, supplier model), M9 (selector grammar duplicated), M10 (Inspector controls need semantic identity). |
| `mere/design_docs/mere_docs/implementation_strategy/2026-11-07_engine_split_plan.md` | 2026-11-07 to 11 | D10 (squish), D23 (host layer as a library; MCP moves to Turnstone), D26 (diagnostics stays a library), D37 (`host/`), D39 (keep dependent-free crates), D51, D53. |
| `mere/design_docs/2026-10-06_data_formats_brief.md` | 2026-10-06 | Person-authored text is TOML; program-written is JSON; postcard inside the stack behind a version header; "identity is never the source text". |

**Rulings that constrain this brief:**

1. *D53 itself:* uxtree "stays whole". The options in §7 are shapes inside one crate, with features, not a split.
2. *Diagnostics plan, Constraints:* "a stale target cannot be retargeted because a node identifier was reused";
   target lifetimes are "never allocated from observation reads"; a frame epoch or an observed-description counter is
   never substituted for owner-issued lives; AccessKit owns typed node-reference traversal and Mere "keeps no second
   list of relation properties". Any id scheme in §7 has to meet these.
3. *Diagnostics plan, Constraints:* the observation store "neither dispatches actions nor recomputes their truth";
   products redact before buffering; `act = true` and `busy = false` prove neither success nor presentation.
4. *D51:* overmap and view stay separate. `gloss` (a `mere-view` feature) is the only uxtree user in the view; the
   shapes do not need that boundary to move.
5. *The `mere:script` and guest boundaries:* a guest's tree reaches the host only as data it is handed. Graft's
   per-tree id space is the property that keeps a guest from naming a host node. A one-hashed-space join (`stitch`)
   gives that up. This is the strongest reason fork 2 exists.
6. *D26:* the registry stays a library; the host workspace already names `mere-registry` (default features off), so a
   uxtree lint may emit through it, or through a sink trait as `ux-events` does.
7. *Data formats:* a tree snapshot written for golden tests or export is JSON. The `.scn` scenario grammar is a custom
   text format authored by people, which the rule would put in TOML. Not a fork here, but any new scenario verb adds to
   the 468 files that would one day migrate.
8. *D39 and D23:* `ux-events` is kept; the MCP server leaves Mere for Turnstone. A shape that reads the tree from MCP
   is a Turnstone-side change.

## 6. Outside reference points

No web access was used. These paragraphs are from memory and from what the family's documents already cite (the
composition brief reads AccessKit's source). Treat each as a prompt to verify before a ruling cites it.

**AccessKit's tree model.** A `TreeUpdate` carries nodes keyed by a u64 `NodeId`, an optional `TreeInfo` and a focus.
Ids are opaque and unique within one tree; the consumer keeps whatever it was last given and panics on a malformed
frame (the reason `Composition::new` exists). A `TreeId` names a tree; a graft node with a `tree_id` hosts a subtree;
an `ActionRequest` names `target_tree`. There is no traversal API for the relation properties (`remap.rs` lists the 16
node-id-carrying fields by hand and has a schema canary). AccessKit gives no promise that an id means the same thing
in the next session.

**The browser accessibility tree against the DOM.** A browser computes a separate accessibility tree from DOM, style
and ARIA, drops nodes that carry no meaning, and keeps its own node ids apart from DOM identity. Genet's projection
has the same split (`DocumentA11yNodeId` from `opaque_id`). Playwright's role-and-name locators and its text snapshot
of the tree are the same shape as taproot's `role:name` selector and `forest_lines`: the market answer to "what do
you click" is role plus accessible name, with an authored test id when the name is not unique.

**UI Automation and AT-SPI providers.** UI Automation separates a `RuntimeId` (unique for the session, not persistent)
from an `AutomationId` (assigned by the developer, meant to be stable). AT-SPI objects are addressed by a bus path that
lasts as long as the application. The split is the nearest precedent for fork 5: a minted id the platform can rely on
for a session, and an authored key that automation can rely on across runs.

**WebDriver and Playwright locators.** WebDriver element references are opaque per-session handles; using one after
the node left the document is a "stale element reference" error and never a retarget. That is the same rule as the
diagnostics plan's "a stale target cannot be retargeted". Playwright adds lazy locators that re-resolve on every use,
which is what a path or selector gives that a held id does not.

## 7. What "a spine" could mean

Five candidate shapes. Each is stated with what it commits the stack to. Costs are readings from §2 and §3: rough
lines touched, in crates named, not measured by building anything. None is ruled.

**Shape A. One semantic tree that accessibility, probes and MCP all read.** The AccessKit forest (F1), with uxtree as
its carrier and a side table of paths and provenance. `ux-events` stops being event-only and gains a tree reader;
taproot, Turnstone's `assert a11y` and the MCP read the same forest.
- *Commits to:* uxtree (or a type below it) becoming something Genet and Cambium can name. Either uxtree's tree and
  address types drop into a crate Genet owns, or uxtree's `inker` edge moves. Both revisit F10's two-application proof.
- *Touches:* `genet-render` a11y (1,880), `genet-winit-host` a11y (1,289), rootstock and `cambium-winit-a11y` (2,600
  src), Turnstone's a11y layer (4,200 src), `ux-events` (2,200), MCP (950). Order of 12,000 lines of existing code are
  in the path; the new code is a few hundred lines but the migration is wide.
- *Boundaries moved:* Genet to host direction; D23's MCP home; the 2026-07-22 refusal of uxtree as wire.
- *Cost:* highest. *Gain:* one view of the UI for every consumer. *Reading, not ruled:* it is wider than D53 asks.

**Shape B. uxtree as an identity and addressing scheme only.** Trees stay where they are. uxtree defines the address
(a tree id plus a node id plus an authored or derived key), a specified hash, and a locator that resolves an address
in any AccessKit forest.
- *Commits to:* one address grammar shared by taproot (Genet), mandrake, Turnstone and any later tool, and a stability
  promise (fork 5).
- *Touches:* uxtree, +150 to 300 lines (a path index beside `UxTree`, a pinned hash, a resolver); taproot, +100 to 200
  (a `path:` selector token and a lookup impl; it already has `SemanticLookup`); mandrake, +30 (and the M9 duplicate
  parser could go); Turnstone, +100 to 150 (a path index next to `A11yRoutes`). Scenario files: none must change.
- *Boundaries moved:* the grammar is a contract across Genet and the host layer. The path root list (F3) becomes a
  governed vocabulary.
- *Cost:* low to moderate. *Gain:* direction 2 of D53, and a place to pin ids.

**Shape C. uxtree as the diagnostics and inspection model only.** A lint over an AccessKit forest (and over a pair of
frames for stability), a stable text and JSON snapshot of the forest, and emission through the registry's existing
channels.
- *Commits to:* one findings vocabulary (unlabelled, orphaned, duplicate, unreachable, unstable, action without route,
  missing bounds) and an emission path, a sink trait or `mere-registry` directly.
- *Touches:* uxtree, +300 to 500 lines and about the same in tests (the structural half is the 26-line structural
  core of `NativeIds::prepare`, which would be shared, not copied); registry, 0 to 2 descriptors (five `ux:` channels already
  declared); Turnstone, +50 (a call site in its projection and a scenario verb); `cambium-winit-a11y`, −60 if its
  checks call the shared one. Probes in `ux-events` stay as they are.
- *Boundaries moved:* none new if emission is a sink. A direct registry edge is allowed (D26, host workspace).
- *Cost:* moderate. *Gain:* direction 3 of D53, and the audit the June plan wanted.

**Shape D. Status quo with adapters.** Nothing grows. Thin adapters join what exists: `ux-events` carries an optional
tree address; `forest_lines` becomes shared; a doc maps the four identity schemes.
- *Commits to:* the four schemes continuing side by side.
- *Touches:* under 200 lines and a document. *Boundaries moved:* none.
- *Cost:* lowest. *Gain:* the least. It keeps D53's wording and does not act on it.

**Shape E. D53 as worded.** Its four directions map to the shapes: graft (done for Turnstone's two uses; the Cambium
publisher and Pelt use other joins), path-addressed scenarios (B), lint (C), promotion audit (a reading task, F-count
in fork 8). *Reading, not ruled:* the ruling is B plus C plus an audit, which is a subset of A. The question is whether
A is wanted at all, which is fork 1.

## 8. Forks for Mark

Numbered forks, not decisions. Each has its evidence, then two to four options stated neutrally, then a recommendation
marked as one where the evidence supports it. Forks 1 to 4 are the ones the rest depend on; a first round could take
them, with 5 to 9 following.

**Fork 1. Which shape does the spine take?**
*Evidence:* §7. D53 names B and C and an audit (§1). Shape A is a superset and moves a Genet edge (§2, F10). The tree
every consumer already shares is AccessKit's (F1).
- (a) B and C together, no new tree model (D53 as worded).
- (b) B alone first; C when the address is ruled.
- (c) C alone first (it needs no address and the channels exist).
- (d) A: one semantic tree for accessibility, probes and MCP.
- (e) D: status quo with adapters.
*Recommendation (reading, not ruled):* (a), sequenced B then C. Evidence for it: both are additive to 468 scenario
files, neither moves an edge, and the June plan's audit step has a ready core in `NativeIds::prepare` (26 lines).

**Fork 2. What does "one ID space" mean for graft?**
*Evidence:* D53 says graft joins subtrees "into one ID space". Graft keeps each tree's own `NodeId`s under a `TreeId`,
by design, so a guest cannot name a host node and each guest's ids stay stable (`graft.rs:7-25`, AC2). `stitch` is the
one hashed space.
- (a) One address space: the pair (`TreeId`, `NodeId`) is the stack's address; ids stay separate.
- (b) One hashed id space: every tree through `stitch`.
- (c) Both: graft by default, `stitch` only for trees uxtree itself builds.
*Recommendation (reading, not ruled):* (a). It matches AC2 and the `mere:script` and guest boundary (§5.5). The words in
D53 can then be restated as "one address space".

**Fork 3. Is the common model AccessKit's, or a richer uxtree one?**
*Evidence:* F1 (six producers end in `TreeUpdate`); `incipit::forme::ux` is a richer model with no caller (F6); the
donor spec proposed three layers (§5); the diagnostics plan keeps AccessKit as the owner of node references.
- (a) AccessKit's tree is the model. uxtree adds side tables (path, provenance) and checks, never a second node type.
- (b) uxtree owns a richer node type and lowers it to AccessKit.
- (c) Decide when a second consumer needs a field AccessKit lacks.
*Recommendation (reading, not ruled):* (a). Reason: (b) repeats the 2026-07-22 refusal at a smaller scale, and
(c) is a way of reaching (a) or (b) with a wasted measure in between.

**Fork 4. What do scenarios use as the address?**
*Evidence:* F3 (path not recoverable), F8 (role and name 67 of 176 clicks, class 109, data attribute 71, 54 coordinate
clicks, 576 palette labels), the stale-target rule (§5.2), UI Automation's two ids and WebDriver's stale-element rule
(§6).
- (a) Keep role, name and class; add a `path:` token that resolves over uxtree-built trees only.
- (b) Authored keys: a `key:` token resolved from a stable key the app sets (`data-key` today; the AccessKit node's
  `author_id` or a defined property in the tree).
- (c) Action ids as the address for commands (hest), and keep selectors for pointing.
- (d) Held ids: a scenario captures a node id and reuses it (needs owner-issued lives, §5.2).
*Recommendation (reading, not ruled):* (a) plus (b) at once, (d) not at all. Evidence: path covers only uxtree-built
trees (F3), key covers Cambium and Genet trees through the DOM attribute already in use (F8), and (d) is ruled out by
the stale-target constraint.

**Fork 5. What does an id promise, and how is it made?**
*Evidence:* F2 (four schemes), F4 (unspecified hasher, positional paths, constant root, no duplicate check), the
Cambium publisher mints ids bound to identity, UI Automation's runtime and automation ids.
- (a) Keep path hash; pin a specified hash in uxtree and promise stability only for the same code and the same
  document; positional paths stay.
- (b) Path hash with member-keyed paths (`tab/{member}`, `list/{key}`), for edit stability; document which producers
  can give a key and which cannot.
- (c) Two ids: a minted session id for the platform (as now in Cambium) and an authored key for automation (fork 4b).
- (d) Promise nothing beyond one run; keep the hasher.
*Recommendation (reading, not ruled):* (c), with (a) as the minimum if fork 4 settles on a path token. Evidence:
(c) matches how the stack's one OS-facing publisher already works and how UI Automation divides the job.

**Fork 6. Lint: which checks, where, and what does it emit?**
*Evidence:* F5 (three checkers, five declared channels, no emitter), F7 (two channel vocabularies), the diagnostics
plan (products redact before buffering; a store observes and never dispatches).
- Checks: (a) structural only (duplicate, cycle, dangling, unreachable; today's core); (b) plus labels, bounds, and
  action without route; (c) plus stability across a frame pair (needs a retained previous frame).
- Where: (i) a library with no side effects, called by hosts and tests; (ii) also gating publication (refuse, as
  `NativeIds::prepare` does now); (iii) a scenario verb only.
- Emission: (x) a sink trait; (y) `mere-registry` directly on the existing `ux:` channels; (z) `ux-events` `UxEvent`.
*Recommendation (reading, not ruled):* checks (b), library (i) with the scenario verb (iii), emission (x) by default
with (y) as the first sink. Stability across frames (c) after fork 5. Evidence: (i) is the cheapest place to share the
26-line core, and (x) keeps the registry edge optional.

**Fork 7. Does diagnostics learn tree identity, and what happens to the parked chain?**
*Evidence:* F6 (`SurfaceId`, `descriptor_for`, `forme::ux`, about 3,000 lines with no consumer; D39 keeps `ux-events`),
F7. `UxEvent` names a surface kind but never an instance.
- (a) Leave the chain parked; add nothing to events.
- (b) Add an optional tree address to `UxEvent` (the fork 2 pair), so a probe can say which pane.
- (c) Retire `incipit::forme::ux` (no caller in three repositories) and `descriptor_for` (no caller); keep `SurfaceId`
  and `ux-events` per D39.
- (d) (b) and (c).
*Recommendation (reading, not ruled):* (d), after fork 2. This is the one place the evidence is thin on need: nothing
consumes the chain, so (a) is a fair answer if no diagnostics consumer is planned.

**Fork 8. Promotion audit (D53 direction 4): scope and home.**
*Evidence:* Turnstone's a11y layer is 55 files, 4,222 non-test lines. Likely generic pieces, read from file names and
headers and not yet audited: the route table (`a11y.rs`, `route_table.rs`), `forest_lines` (53), `matches_lines` and
`assert a11y`, `contributed_a11y.rs` (233) and `foreign_a11y.rs` (about 1,000 with its parts: the grafted foreign-lens
publication), `bridge_integrity` tests (149), `a11y_bridge.rs` (323, the direct `AccessKitBridge` publication that
`cambium-winit-a11y` also does in its own way).
- (a) Promote only what already has a second user (today: none, by grep of Knot, Woodshed, Cleromancy and Isocosm).
- (b) Promote the tree readers (`forest_lines`, the matcher) into uxtree, and leave publication in Turnstone.
- (c) Fold Turnstone's direct bridge into `cambium-winit-a11y`'s `A11yHost`, so there is one publication path and one
  id policy (F2).
- (d) Audit first as a read-only pass and bring back a list.
*Recommendation (reading, not ruled):* (d), then decide (b) and (c) from the list. (c) is the largest change in this
brief: it ends the path-hash ids reaching the OS and is not an uxtree question alone.

**Fork 9. Should the MCP expose the UI tree?**
*Evidence:* `graphshell-mcp` addresses routes, projections, instances and intents with an observed epoch and revision
(948 src lines); it "adds no authority"; D23 moves it to Turnstone. The tree holds labels of whatever the user is
looking at. The June plan imagined a typed agent harness (D7) and a model that "should not be asked to operate the app
through raw pixels".
- (a) No: MCP stays at the data and intent layer. UI automation stays with taproot scenarios.
- (b) A read-only resource, the forest as text lines or JSON, with the redaction the diagnostics plan requires.
- (c) (b) plus a tool that invokes a tree node's action, through the same route table and stale checks.
*Recommendation (reading, not ruled):* (a) for now. The evidence for a need is thin: no consumer asks, the identity
and redaction questions (forks 4, 5) come first, and D23 puts the server in a repository this brief does not govern.

**Suggested order (not a decision):** round one, forks 1 to 4; round two, 5 and 6; round three, 7 to 9.

## 9. What could not be determined

- Whether the AccessKit `NodeId` or the DOM `data-` key is the better carrier for an authored key (fork 4b): not
  tested; no code was run.
- Whether any consumer outside the six scenario repositories pins a hashed id: none found by grep; a binary or script
  outside git is not covered.
- The 1.3k-line figure in D53: it does not reproduce under any filter tried (26 files name uxtree; 55 files carry
  a11y; 35 files, 7,523 lines mention it case-insensitively).
- How much of `foreign_a11y` and `a11y_bridge` is generic: sized by lines, not read through.
- The line costs in §7: estimates.
- The outside reference points (§6): from memory; no network.
- Whether `ux:*` registry channels are emitted by a binary not in the audited repositories (Djinn and the resident were
  not searched beyond the Mere tree).
