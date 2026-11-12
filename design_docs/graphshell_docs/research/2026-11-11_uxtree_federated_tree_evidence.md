# uxtree federated tree: evidence tables

**Date:** 2026-11-11
**Status (2026-11-11):** evidence companion to the [federated tree brief](2026-11-11_uxtree_federated_tree_brief.md).
Docs only. Nothing built or run; read from `origin/main` in detached scratch worktrees.
**Audit base:** mere `217dc2b16`, graphshell `d97a69a5`, turnstone `fd57dd3f`, genet `7c6ffc87b`.
**Paths.** Toolkit paths are mere.git's today and move to Graphshell's `host/` at the cutover being rehearsed
(engine split, `mere/design_docs/mere_docs/implementation_strategy/2026-11-07_engine_split_plan.md`). The key is the
brief's: `rootstock/` is mere.git `crates/cambium/cambium-rootstock/src/`, `winit-a11y/` is mere.git
`crates/cambium/cambium-winit-a11y/src/` (package `cambium-a11y`), `uxtree/` is mere.git `crates/forme/uxtree/src/`,
`cartography/` is mere.git `crates/canvas/cartography/src/`, `gs-port/` is mere.git `ports/graphshell/src/`,
`dsa/` is genet.git `components/shared/document-session-api/src/`, `genet-render/` is genet.git
`components/genet-render/src/`, `taproot/` is genet.git `components/taproot/`, `pelt/` is graphshell.git
`host/ports/pelt/desktop/workspace_viewer/`, `gsc/` is graphshell.git `crates/graphshell-client/src/`,
`turnstone/` is turnstone.git `src/`.

**Methods.** "Lines" are physical lines (`wc -l`). "Non-test" drops files named `tests.rs`, `*_tests.rs`, `*_cases.rs`
and anything under `tests/`. "Files naming AccessKit" is `git grep -l accesskit` over the crate's source, non-test;
it counts whole files, so it overstates the code that would change and is used only to bound it.

## E1. The producers, field by field

Six producers of "a UI thing a reader or a tool can name". The first brief listed four; this pass read two more that
already hold semantics above AccessKit (Genet's projection as its own row, and graphshell-client's tree).

| | Cambium native (rootstock + `cambium-a11y`) | Genet document projection | Pelt composite | Canvas reader | Turnstone app projection | graphshell-client tree |
|---|---|---|---|---|---|---|
| Source of truth | Genet DOM laid out by Livery, plus Sprigging leaves and texture producers' `ProducerSemantics` | The `ScriptedDom` and its layout | Frisket chrome DOM, plus each focused tile's `DocumentA11yProjection` | `kernel::Graph` members on screen | `App` state (chrome, panes), plus grafted foreign and contributed trees | A mounted `sceno` scene and the endpoint's presentation manifest |
| Model before AccessKit | `SemanticProjection` (AccessKit tree plus target maps) and `ProducerSemantics` (5 neutral roles) | `DocumentA11yProjection`: serde, 49 roles, state, 6 actions, `revision`, `support` | none of its own; consumes Genet's | `CanvasDescription` / `DescribedItem` | none: builds AccessKit `Node`s directly (49 `Node::new` sites in 14 files) | `AccessibilityTree` / `AccessibleItem` (3 roles, `AdvertisedAction`s) |
| Id minted how | Source id = `dom.opaque_id`, or a generated producer id; `NativeIds::prepare` remaps each to a process-wide counter, kept while the `SemanticNodeIdentity` is equal | `opaque_id` of the DOM node | Per tile, a counter from 2^63, kept while the tile's session identity holds | Process-wide counter per member UUID, kept while `SurfaceNodeLifetime` holds | `DefaultHasher` over a path string (`turnstone/chrome/omnibar`) | `sceno::InstanceId` from the scene |
| What an id promises | Session; never recycled ("no ended publication may recycle one", `winit-a11y/publication.rs:42-44`) | Object life within a live session; `revision` scopes each observation (`dsa/a11y.rs`, doc on `DocumentA11yProjection`) | Session; "never recycles an issued child ID, which makes a stale platform action inert rather than aliased" (`pelt/accessibility.rs:340-343`) | Member life; "reused slots never reuse a key", exhaustion fails closed (`cartography/canvas/reader.rs:28-60`) | Same path, same id within one toolchain; positional paths move (first brief F4) | Instance within a scene epoch; `observed_epoch` and `observed_revision` guard intents |
| Stale-action check | `A11yProvenance::is_current`: owner live, contributors live, re-captured identity equal (`rootstock/a11y_provenance.rs:160-174`) | Engine revalidates `revision`, target and advertised action (`DocumentA11yActionRequest` doc) | `document_action_is_current`: session identity, content rect, placement generation, revision, advertised action (`pelt/accessibility.rs:35-62`) | `member_of_described` compares the witness (`reader.rs`) | Bridge admission generation per `(TreeId, NodeId)`, bumped when the route or its source changes (`turnstone/shell/a11y_bridge.rs:58-140`); contributed routes carry a pane generation (`contributed_a11y.rs:27-50`) | Endpoint checks observed epoch and revision |
| Side table beside the tree | `PublicationBindings { identities, requests }`: "metadata, not another tree" (`rootstock/semantics.rs:42-46`) | none (the projection is the table) | `action_map: HashMap<AccessNodeId, WorkspaceA11yActionTarget>` | `DescribedKeyRegistry` (key to member and life) | `A11yRoutes = HashMap<NodeId, A11yRoute>` (`turnstone/a11y.rs`); bridge `published` map | none (items carry their own fields) |
| Domain reference | No (DOM node only) | No | Tile id and session identity | Yes: member UUID; dropped when lowered to `ProducerNode` (`gs-port/canvas_reader.rs:73-95`) | Yes for frozen instances (`SelectMember { pane, member }`); chrome nodes have none | Yes: `SourceRef { adapter, id }` (`sceno`) |
| Action identity | Producer actions: `ProducedAction { slot, key, id: String }`; DOM nodes: `A11yAction` kind only | `DocumentA11yAction` enum (6) | `DocumentA11yAction` bits | `AdvertisedAction.intent` (chirograph `IntentReference`) | Turnstone `Action` enum (about 190 variants; not `hest`) | `AdvertisedAction.intent` |
| Provenance | `A11yProvenance`: owner, document, mount, producer weak handle, child token, leaf lifetime | `DocumentA11ySupport`: capability and stated limitations | Tile, session identity, placement generation | Implicit (the canvas slot) | Implicit in the path root | Session, epoch, revision, presentation offer |
| Update model | Full snapshot each publication; candidate namespace advances only on install | Full projection per revision | Full tree per frame | Full description per read, capped at 200 items | Full tree per frame; foreign trees take supplier deltas (`turnstone/foreign_a11y.rs`) | Snapshot plus `scenotime` diffs |
| Focus and action routing owner | Host (`A11yHost` drains, app dispatches through its click path) | Engine for actions other than Click; host drives Click by point | Pelt workspace | Canvas (`canvas_act`) through the Cambium producer | Turnstone shell through the update spine | Endpoint (remote) or host `LocalActions` |
| Outputs today | AccessKit (`cambium-a11y`); ARIA DOM mirror (`cambium-genet-web-host`, 517 + 275 lines) | Lowered once to AccessKit (`genet-render/a11y.rs:507`, `accesskit_tree_from_projection`) | AccessKit | Cambium producer semantics | AccessKit; text lines for scenarios (`forest_lines`) | uxtree `frozen`, Cambium panes, the remote lens |
| Names uxtree | dev-dependency only (E1a test and example) | cannot (E3) | no | no | yes | no (uxtree names it, feature `frozen`) |

## E2. What each lowering drops

Read at the one place each model becomes AccessKit.

| Lowering | Kept | Dropped |
|---|---|---|
| Genet `accesskit_tree_from_projection` | role, name, description, value, numeric range, state flags, bounds, children | `revision`, `support` and its limitations, `DocumentA11yAction` as a typed list (AccessKit actions only) |
| Canvas `canvas_semantics` to `ProducerNode` | key, name, rect, action intent as `ProducerAction.id` | member UUID, `focused`, the action's effect and payload schema |
| Cambium `project_semantics` to AccessKit | everything AccessKit models | producer action id, provenance, DOM node target: kept only in `PublicationBindings` beside the tree |
| uxtree `project_document` | role, label, level, children | the path (hash is one-way); link target and submit endpoint survive only by being written into `value`; list ordering and menu row kind only as `description` text (`uxtree/lib.rs:220-470`) |
| Turnstone `project_app` | AccessKit fields | the route: kept in `A11yRoutes` beside the tree |

## E3. Dependency positions (manifests and Cargo.lock)

| Fact | Evidence |
|---|---|
| uxtree depends on `accesskit`, `inker`, `tracing`, optional `graphshell-client` | `uxtree/Cargo.toml` |
| `inker` (its own repository) depends on `document-session-api` (Genet) | mere.git `Cargo.lock`, `inker 0.1.1` entry |
| Genet keeps no edge upward: "No member that stays depended on any of the eleven" (the Cambium family) | genet.git `Cargo.toml`, workspace comment |
| So Genet naming uxtree would close a loop genet → uxtree → inker → genet | the two rows above |
| Reverse dependencies of uxtree in mere.git's lock: `cambium-a11y` (dev), `mere-view`, `platen` | `Cargo.lock` scan |
| `mere-cartography` depends on no AccessKit crate | `Cargo.lock` |
| `taproot` depends on `accesskit` and `document-session-api` only (plus Genet DOM and Livery) | `Cargo.lock` |
| At the cutover `uxtree`, `platen`, `mere-cartography`, the Cambium family (`cambium-a11y` included), `mandrake`, `mere-view`, `ux-events` and `mere-registry` all move to Graphshell's host workspace | engine split plan, P3 list (lines 118-135 of the plan as read) |
| Pelt desktop is already in graphshell.git `host/ports/pelt/desktop` | tree |

`cargo metadata --locked --offline` was started on mere.git and stopped after three minutes waiting on the shared
package-cache lock held by another session; the lockfile scan above answers the same question for direct edges.

## E4. Composition: who joins how

| Join | Users |
|---|---|
| `uxtree::graft` (`Composition`, `Grafts`, `GraftTable`) | Turnstone `foreign_a11y` (nested foreign forests, supplier `TreeId`s intact) and `contributed_a11y`; the E1a test and E1b example in `cambium-a11y` |
| `uxtree::stitch` | Turnstone `project_app`. It concatenates node lists under a new root; it does not rehash (`uxtree/lib.rs:111-135`), so two subtrees built from the same path collide unchecked |
| Minted one-tree namespace | Cambium native publisher (refuses any graft: `Refusal::ForeignTree`, `winit-a11y/publication.rs:63`); Pelt (2^63 range) |

## E5. Consumers: what each reads and the least it needs

| Consumer | Reads today | Address it uses | Least it needs from an envelope |
|---|---|---|---|
| OS assistive tech | AccessKit only | AccessKit `(TreeId, NodeId)` | nothing beyond AccessKit |
| taproot | `SemanticLookup`: `SemanticNode { target: Dom(NodeId) or Contributed(u64), role, name, hidden }` (`taproot/lookup.rs:9-27`); `AccessKitLookup`, `DocumentLookup` | `Selector { surface, matcher (class or role), text, attr }` (`taproot/lib.rs:81-90`); `surface` "scopes attribution, not pane instance identity" | an authored key per node, and a producer instance to scope by |
| mandrake | the same grammar, restated in `wheel::parse_selector` (mere.git `crates/cambium/mandrake/src/wheel.rs:148`) | as taproot | nothing new; one parser |
| Turnstone scenarios | `act` (576 lines) by palette label; `click-at` (54) by pixel; `assert a11y` (37) by substring of `role: label = value` lines; 172 `.scn` files | label, pixel, text | an action id, so `act` can name the command rather than its copy |
| `ux-events` | `UxEvent::{SurfaceOpened, SurfaceDismissed, ActionDispatched { action_id, target: Option<NodeKey> }, OpenNodeDispatched}` (graphshell.git `host/crates/system/ux-events/src/ux_observability.rs:84-101`) | surface kind (20 variants, no instance), `hest::actions::ActionId` (an enum), graph `NodeKey` | a surface instance address; the action id and domain reference it already carries |
| registry lint channels | none: `ux:structural_violation`, `ux:contract_warning`, `ux:tree_build`, `ux:tree_snapshot_built` are referenced only by the descriptor catalog (`host/crates/system/registry/src/diagnostics/descriptor/catalog/phase3a.rs:377-392`) | channel id | producer provenance (who to blame) and a stability class (to judge "unstable id") |
| Turnstone swatch inspection | `apparatus_core::Inspection` lines projected with `node_id_for_path(&line.id)` | line id | nothing new |
| `graphshell-mcp` (868 source lines) | chirograph actions on a projection: `projection`, `instance`, `intent`, `observed_epoch`, `observed_revision` required (`src/tools.rs:48-61`) | `mere://<scope>/<projection>` | none: it addresses the scene, not the UI tree |

## E6. Boundaries

| Boundary | What crosses it today | Semantics carried |
|---|---|---|
| `app-host` guest | out: `actions.emit({name, payload})`; in: typed refusal (mere.git `crates/script/app-host/src/lib.rs` header) | none; a guest sees no tree |
| `block-host` guest | out: `view` returns scene and sheet bytes, capped at 1 MiB (`block-host/src/lib.rs:254-275`) | the guest produces; the host makes it accessible |
| Remote projection | chirograph: `AdvertisedAction { intent, label, explanation, payload_schema, input_form, effect }`, `SemanticRole` (3), `BoundsRelationship`; serde | label, role, actions, intent references |
| Resident | no accessibility code (`host/ports/mere-resident`, grep) | none |
| Data formats | program-written text is JSON; postcard inside the stack behind a version header; CBOR across a trust boundary; "identity is never the source text" (`mere/design_docs/2026-10-06_data_formats_brief.md` §1) | applies to any envelope written out |

## E7. Option 2 bound: files a richer node type would touch

Non-test files that name `accesskit`, as an upper bound of what a producer-built node type would touch.

| Crate | Files | Lines | Narrower read |
|---|---|---|---|
| uxtree | 3 | 1,236 | all of it; 34 `Node::new` sites |
| rootstock | 3 | 1,235 | `semantics.rs` 506, `document_projection.rs` 160 |
| `cambium-a11y` | 5 | 1,458 | `remap.rs` 230 maps AccessKit's 16 id-carrying fields by hand |
| `cambium-genet-web-host` | 1 | 275 | `a11y.rs` 517 lowers to ARIA |
| `cambium-genet-winit-host` | 1 | 900 | harness |
| platen / `mere-view` | 2 / 1 | 189 / 458 | `gloss.rs` |
| `genet-render` | 4 | 2,988 | `a11y.rs` 1,880 (only `accesskit_tree_from_projection` lowers; the projection itself is neutral) |
| `genet-winit-host` | 2 | 1,615 | `a11y.rs` 1,289, the OS bridge |
| taproot | 3 | 2,115 | `lookup.rs` 176 |
| Pelt | 5 | 11,116 | whole files; the projection is `accessibility.rs` 1,247 |
| Turnstone | 32 | 7,226 | whole files; the a11y family is 4,222 non-test lines (first brief) |
| cartography, `gs-port` | 0 | 0 | they lower to Cambium producer semantics |
