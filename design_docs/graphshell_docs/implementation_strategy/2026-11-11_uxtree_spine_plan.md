# uxtree spine plan: a federated tree with one envelope

**Status:** 2026-11-18. P0 to P5 landed. P6 supplier landed on Graphshell main at `2a5a4f07`.
Turnstone is qualifying the landed supplier with the Crystal update. P7 is under way.
P8 to P11 remain open.
The slice 4 cutover moved uxtree, Cambium and E1a to Graphshell's `host/`.
P0 remains a gate for D1. If it fails, D1 returns to Mark.

**For:** carrying out `mere:engine-split D53`, "uxtree grows into the stack's accessibility and automation spine".
Two briefs read the producers and put the forks to Mark:
[the spine brief](../research/2026-11-11_uxtree_spine_brief.md) (forks 1 to 9) and
[the federated tree brief](../research/2026-11-11_uxtree_federated_tree_brief.md) with its
[evidence tables](../research/2026-11-11_uxtree_federated_tree_evidence.md) (forks A to F, cited here as E*n*). This
plan records his answers and lays out the work they ask for.

**Not here:** a node type uxtree owns (option 2 in the second brief); invoking actions over MCP (open question 5);
the engine split's own phases; the relevance review of the moved design documents (`DOC_POLICY.md`, local addendum);
human assistive-technology runs (`2026-10-28_platform_qualification_plan.md`).

Receipts go to `archive_docs/receipts/2026-11-11_uxtree_spine_plan_receipts.md` (DOC_POLICY §8), created when the
first receipt exists.

## Constraints

- **AccessKit stays the node type.** The envelope carries address, stability class, provenance, action ids, domain
  reference and authored key. It carries no role, name, state, bounds, children or relation: the diagnostics plan keeps
  "no second list of relation properties" and leaves typed traversal to AccessKit (D1).
- **The stale-target rule.** "A stale target cannot be retargeted because a node identifier was reused"
  ([diagnostics plan](2026-10-28_diagnostics_and_accessibility_plan.md), Constraints). A producer declares a node's
  class; a reader never infers it. A frame counter is never a life. An action is always rechecked by its owner (D4).
- **The 2026-07-22 refusal stands.** uxtree is not a wire contract
  (`mere/design_docs/archive_docs/2026-10-06_completed_plans/2026-07-22_graphshell_remote_projection_host_plan.md`,
  §4.3 and the promotion table). Nothing in this plan crosses a process or guest boundary except the read-only
  tooling snapshot of D6, which is not a contract.
- **Redaction before buffering.** Products project and redact before anything is buffered (diagnostics plan,
  Constraints). It binds the snapshot and the MCP resource (D6, P10).
- **Genet keeps no upward edge.** uxtree depends on `inker`, and `inker` on Genet's `document-session-api`, so Genet
  cannot name uxtree. The family cycle checker passes after every phase that touches a manifest (D2).
- **Data formats.** Text that a program writes is JSON, so the tooling snapshot is JSON; identity is never source text
  (`Code/CLAUDE.md`, "Data formats"; `mere/design_docs/2026-10-06_data_formats_brief.md` §1). The `.scn` scenario
  grammar is hand-authored custom text, which that rule would put in TOML; this plan does not rule on it, and a new
  verb adds to the files that would one day move (P7, P9).
- **Nothing here gates publication.** The lint reports; it does not refuse a tree (D7). The publisher's own refusals
  (`NativeIds::prepare`) are unchanged by this plan unless open question 8 is answered otherwise.
- **Producers keep their models.** Genet's `DocumentA11yProjection`, Cambium's `ProducerSemantics`, the canvas's
  `CanvasDescription` and graphshell-client's `AccessibilityTree` stay as they are (D1, D2).
- **Headed checks** are required for the promotion phase (P11).
- **Nothing a phase produces touches main until it is verified in its own worktree.** The family's build and
  concurrency rules apply and are not restated.
- **Estimates are estimates.** Line figures marked "estimate" come from the briefs' reading of neighbouring code and
  are not measurements.

**Path key** (current receiving homes, 2026-11-17). `uxtree/` is graphshell.git `host/crates/forme/uxtree/`.
`rootstock/` is graphshell.git `host/crates/cambium/cambium-rootstock/src/`. `winit-a11y/` is graphshell.git
`host/crates/cambium/cambium-winit-a11y/src/` (package `cambium-a11y`). `cartography/` is graphshell.git
`host/crates/canvas/cartography/src/`. `gs-port/` is turnstone.git `ports/graphshell/src/`;
`canvas_reader.rs` is present there after the reference app move.
`taproot/` is genet.git
`components/taproot/`. `dsa/` is genet.git `components/shared/document-session-api/src/`. `pelt/` is graphshell.git
`host/ports/pelt/desktop/workspace_viewer/`. `ux-events/` is graphshell.git `host/crates/system/ux-events/`.
`turnstone/` is turnstone.git `src/`.

## Phases

Dependency order. Sizes are the briefs' measured sizes of the code in the path, not of the edits. A phase that touches
another repository says so. Names the rulings do not fix (the envelope type, field types, verb and token names) are
named in the phase that first needs them.

### P0. The prototype (gate)

Extend the E1a bench, `host/crates/cambium/cambium-winit-a11y/tests/e1a_sessions.rs` (615 lines; it grafts two retained Cambium sessions
through `uxtree::graft` and reads them back through `accesskit_consumer`), with:

- a Cambium session carrying a texture producer that advertises a command (Life ids, an action id, a domain reference,
  as the canvas does);
- a Genet-backed session (Scoped ids by revision);
- the host tree built with uxtree paths, the Derived case;
- one consumer that finds a node's envelope by address and invokes by action id.

The envelope here is a sketch, kept or discarded at the gate. P1 names the real types. The lint here is one test-local
check for a node with no envelope; P7 builds the library.

- Done when, all in one run:
  - each node's envelope is filled from what its producer already knows, and the consumer reads all three producers
    through one code path;
  - a request to a retired guest is refused at its owner after a `NodeId` is reused (the stale-target control);
  - the check reports a planted node with no envelope, and reports none on the unplanted tree in the same run (the
    positive control and the clean control);
  - the consumer has no per-producer branch: no match, `if` or feature on producer kind in its source (a grep of the
    consumer for the three producer names returns zero lines).
- If the consumer needs per-producer branches, the federated form is not holding. D1 then goes back to Mark as a
  question, with the finding. This plan does not decide what replaces it.
- Needs: the slice 4 cutover and the E1a bench, both present under Graphshell's `host/` as of 2026-11-17.

The prototype lives in the E1a bench's `tests/e1a_spine/` modules. It reads a retained Cambium session
with a registered `TextureProducer` through Rootstock's `project_semantics` and publication bindings;
a real `LiverySessionEngine` session through `DocumentSession::accessibility_projection`; and a host
tree made with uxtree paths and grafts. All three are read through the same address consumer.
Its command id and domain reference come from the texture model; Genet and host nodes advertise no
command IDs. The envelope does not invent IDs from their action kinds. P1 still names production types.
The gate passes without changing D1 or adding a producer-kind branch to the consumer. The
[receipt](../../archive_docs/receipts/2026-11-11_uxtree_spine_plan_receipts.md) records its controls and limits.

P0 receipt budget: 100 MB retained across repositories, under `Code/testing/graphshell/uxtree-p0/`.
Required receipts: the complete E1a run including the mixed-producer gate and deliberately broken controls,
the consumer's producer-name search, selected source/resolution identities and required hook results.
The prototype's envelope and missing-envelope check remain test-local; production envelope types are P1.

### P1. The envelope and the stability classes, in uxtree

A new envelope module in uxtree (200 to 400 lines with its tests; estimate, second brief §3.4). It holds the six
fields of D1 and the four classes of D4: Life, Scoped, Derived, Authored. The address is first the pair "producer
instance and local id" (D3); P6 adds the `(TreeId, NodeId)` form. Class is set by the producer's constructor, never
computed from a tree. `taproot` is not touched here (P4). Named in P1: the envelope type, the class type, the field
types, and where in the crate the module sits.

- Done when: the six fields and four classes exist with unit tests; a test lists the envelope's fields and finds none
  that AccessKit already carries (role, name, state, bounds, children, relations); uxtree's manifest gains no edge to
  Genet and the family cycle checker passes; the per-field table of which producers fill it (second brief §3.2) is in
  the module docs.
- Needs: P0 passed.

P1's production module is `uxtree::envelope`. Its names are `Address<I>`, `Stability` and
`Envelope<I, P, D>`: the instance, provenance and domain-reference types remain caller-owned, so
hosts can preserve their existing identities and weak admission witnesses without an upward edge.
The six fields are `address`, `stability`, `provenance`, `action_ids`, `domain_ref`, `authored_key`.
The producer explicitly chooses Life, Scoped with a revision/epoch, Derived with an optional owner
generation, or Authored with a key. An absent Derived generation is an observation only; this module
does not authorize actions or allocate lives, revisions or generations. P5 retains generation-map work.
The E1a bench now uses these public types and the Life/Scoped/Derived constructors in place of its
P0 sketch. The per-field producer table is in the module docs. Its manifest is unchanged.

P1 receipt budget: 100 MB across repositories, under `Code/testing/graphshell/uxtree-p1/`.
Required receipts: unit and adapted P0 supplier runs, deliberately broken constructor/schema controls,
native and wasm library checks, the unchanged uxtree manifest and family-cycle check, and normal hooks.

### P2. Cambium's native publisher and the canvas fill the envelope

Rootstock's `semantics/publication.rs` fills one uxtree envelope per semantic node: Life addresses,
existing weak provenance, actual producer command IDs, optional source reference and authored DOM key.
`SemanticPublication` holds these envelopes and typed delivery routes. The native publisher finalizes
addresses through its existing ID allocation, then advances envelopes with the tree and routes. DOM
action kinds stay on the AccessKit node; producer command IDs come from `ProducedAction`.

Turnstone's `ports/graphshell/src/canvas_reader.rs` forwards each member UUID through `ProducerNode`
as `sceno::SourceRef::new("mere.graph", member.to_string())`, including the item's action buttons after lowering.
`DescribedKeyRegistry` owns the key; the adapter forwards it.

- Done when: a test walks a published tree and finds an envelope on every node; a texture producer's command appears
  as an action id on its node; a canvas node carries its member UUID as domain reference; a request after the node's
  life ended is still refused at the owner (the existing provenance test passes through the envelope); the count of
  `PublicationBindings` in non-test source is zero, or what remains is named in this phase and lists only what the
  envelope cannot hold.
- Needs: P1.

P2 receipt budget: 100 MB total across repositories, under `Code/testing/graphshell/uxtree-p2/`.
Required receipts: the Rootstock/native publisher baseline and changed suites, a complete published
tree/envelope walk, actual producer command and canvas member-reference checks, unchanged stale-owner
refusals with broken controls, family cycles after manifest changes, receiving builds and normal hooks.
The domain field uses the existing `sceno::SourceRef` contract; it carries the canvas member UUID in
the source's identity scheme. Native envelopes are finalized only after existing ID remapping, and
requests keep their captured weak provenance. This phase changes neither publication refusal policy
nor `DescribedKeyRegistry`'s ownership of canvas keys. `PublicationBindings` is removed from Rust
source. Its replacement, `SemanticPublication`, holds envelopes plus typed `A11yRequest` input routes;
the routes retain their publication witness for delivery and are the only remainder the metadata
envelope cannot represent. DOM authored keys come from `data-key`, falling back to `id`. The
Rootstock fill lives in `semantics/publication.rs` so composition stays below the line ceiling.
The complete-tree gate covers semantic submissions. The existing root-only pre-layout window
bootstrap remains `PendingPresentation`, with no semantic routes or envelopes; it is not a submitted
application semantic tree. Browser runtime and human/platform acceptance remain later gates.

### P3. Pelt and graphshell-client fill the envelope

Pelt fills Life envelopes for the document nodes it attaches beneath each tile. The window,
tile and session identify the producer. Pelt keeps its existing local node allocator and uses
captured session identity, revision, placement generation, content rectangle and action kinds
for admission. Frisket retains a DOM node lookup for shell actions.

The Graphshell host reads the client tree and mounted scene together. Visible items carry
Scoped envelopes with the scene epoch, observed revision, `SourceRef` and advertised intent IDs.
Client tree and action routing stay in graphshell-client.

- Done when: a Pelt tile node carries an envelope; the existing stale-action test (session identity, content rect,
  placement generation, revision) passes unchanged through the envelope; a graphshell-client item reaches the envelope
  with `SourceRef` as domain reference and its observed epoch; the count of `action_map` in Pelt non-test source is
  zero, or what remains is named.
- Needs: P1.

P3 receipts are under `Code/testing/graphshell/uxtree-p3/`, with a 100 MB cap. Pelt's
primary and tearout use the same fill for all mounted document nodes. Addresses include the
window, tile and session, and keep Pelt’s current window node allocator. The envelope retains
the existing typed admission facts. Frisket keeps its DOM-only route lookup. The client
adapter lives in graphshell-host and derives scope and source from one mounted read.

### P4. The host fills the envelope for Genet-backed nodes (touches genet.git)

Taproot's `SemanticNode` adds `authored_key: Option<&str>` and `opaque_address: Option<&str>`.
Built-in lookups leave both empty and selector matching keeps its current behavior. Taproot exposes
its existing selector role spelling for document roles so the host uses the same vocabulary.

Pelt's retained-document lookup owns one `DocumentA11yProjection` observation. It walks the engine's
child order and inherits hidden state. Each node gets a Scoped envelope with the session as instance
and provenance, the engine-local node as local ID, and the observed revision as scope. The opaque
address is for observation. The host admits actions through the typed observation, verifies the same
session and forwards the captured revision for the engine to check. Genet's document projection has
no authored-key field, so these observations leave that metadata empty.

- Done when: `SemanticNode`'s new fields are `None` on Genet's own `AccessKitLookup` and `DocumentLookup` and taproot's
  tests pass unchanged; the host's lookup returns a Scoped envelope for a Genet-backed node; a request carrying a
  stale `revision` is refused by Genet's engine through the host path; no Genet manifest names uxtree (the family cycle
  checker, and `cargo metadata` over the Genet workspace, which the second brief could not finish).
- Needs: P1 and Genet-side review. Land the Taproot change before the Graphshell consumer.

P4 receipts are under `Code/testing/graphshell/uxtree-p4/` with a 100 MB cap across both repositories.

### P5. Derived ids get an owner generation (touches turnstone.git)

Turnstone's path-hashed projection and uxtree's own projections (`project_document`, `frozen`, `gloss`, `platen`) are
the Derived class. uxtree takes the generation map that Turnstone's OS bridge keeps today (`turnstone/shell/a11y_bridge.rs:58-140`,
about 80 lines, estimate): a generation kept only while the route and its source compare equal, so a request captured
against an old admission is refused. `A11yRoutes` (`turnstone/a11y.rs`, 136 lines) becomes envelope action fills.
Turnstone's 69 `node_id_for_path` sites and 49 `Node::new` sites are in the path and are not all edited; chrome nodes
have no domain reference.

- Done when: a request on a Derived id captured before its route changed is refused by the uxtree-owned generation, the
  existing bridge test moved to uxtree and passing; Turnstone's `A11yRoutes` count in non-test source is zero, or what
  remains is named; uxtree's own projections emit Derived envelopes; no frame counter appears as a generation.
- Needs: P1 and D10, both settled.
- P5 uses a uxtree map per publishing window. Turnstone supplies route and source equality. Pure projections expose
  Derived observation envelopes with the producer instance and local ID. The window publisher issues the generation
  for each action admission. Frame fixtures observe and dispatch synchronously through their current owner.
- Turnstone's legacy graph-member selection fills use a different path from the Frozen Projection nodes.
  The platform publisher filters those fills out. Frozen Projection emits observations and has no published selection route.

### P6. The address converges on `(TreeId, NodeId)`

The address type gains the `(TreeId, NodeId)` form beside "producer instance and local id" (D3). A producer that is
grafted uses the pair, and one that is not is lowered to its minted id. Turnstone grafts today (`foreign_a11y`,
`contributed_a11y`); the Cambium publisher refuses grafts (`Refusal::ForeignTree`, `winit-a11y/publication.rs:63`) and
Pelt mints one tree. `GraftTable` already routes an `ActionRequest` by `target_tree` to the owning session.

- Done when: for Turnstone's grafted producers the address is the pair and a resolver test finds each node by it; a
  request naming a `TreeId` no longer in the table is refused; the plan's Findings list each producer with the address
  form it uses, updated here and again at P11.
- Needs: P1, P5, both landed.
- A federated reader declares either a producer instance or an installed tree as the address space.
  The shared graft resolver selects the owner from the tree ID. The owner resolves the local node
  and checks its live admission. Contributed route admission carries both IDs when the window
  publishes it.

### P7. The lint library, the sink trait, the registry sink, the scenario verb

In uxtree, a side-effect-free library over a tree and its envelopes (estimate: 300 to 500 lines and about as many of
tests; first brief, shape C). Checks: duplicate ids, cycles, dangling nodes, unreachable nodes, missing labels,
missing bounds, an action with no route, a node with no envelope. The structural half is the 26-line core of
`NativeIds::prepare` (`winit-a11y/publication.rs`), shared and not copied. Findings go to a sink trait. The first sink
is `mere-registry`'s existing `ux:` channels (`ux:structural_violation`, `ux:contract_warning`,
`ux:presentation_bounds_missing`, `ux:tree_build`, `ux:tree_snapshot_built`; zero to two new descriptors, estimate).
A scenario verb runs the lint in Turnstone's `.scn` runner (about 50 lines, estimate). P7 uses `lint-a11y` and uxtree's optional `registry-lint` feature. The structural findings retain Cambium's existing refusal mapping. Label, bounds, route and envelope findings are advisory.

- Not here: frame-to-frame stability checks. D7 puts them after stability classes exist in code; they are cut as their
  own phase when P1 and P5 have landed.
- Done when: a fixture per check produces its finding and a clean fixture produces none, in the same run; two subtrees
  stitched from one path report a duplicate (`stitch` checks nothing today, E4); a planted finding reaches the registry
  and the five `ux:` channels have an emitter where `git grep` finds none today; the scenario verb fails a scenario on a
  planted finding and passes on a clean one; the publisher still publishes a tree that has a finding (the lint does not
  gate); uxtree's default features add no registry edge.
- Needs: P1; P2 to P5 for the nodes the checks must find enveloped.

### P8. Events gain the address; two dead pieces retire (touches mere.git)

`UxEvent` (`ux-events/src/ux_observability.rs:84-101`) gets an optional node address taken from the envelope. Which
variants carry it is named in P8. Two deletions:

- `incipit::forme::ux` (458 lines, no caller in Mere, Graphshell or Turnstone; it moves to Graphshell's `host/` with
  `incipit` at the cutover, since incipit is not among the five crates that stay, D102). A Graphshell change.
- `descriptor_for` in `mere-kernel` (`mere/crates/graph/graph-kernel/src/accessibility.rs`, 376 lines in the file; no
  caller). `mere-kernel` stays in Mere, so this is a Mere change. It touches `mere:engine-split D59`'s "no kernel code
  moves" only as a deletion. The file also holds `SurfaceId` and the `AccessibilityDescriptor` type (Findings): the
  deletion takes `descriptor_for`, its tests and what only it uses; `SurfaceId` stays. Where `SurfaceId` then lives is
  named in P8.

`ux-events` and `SurfaceId` stay (`mere:engine-split D39`).

- Done when: a probe test shows two panes of one surface kind produce events with different addresses; `git grep` for
  `descriptor_for`, `emit_ux_tree` and `forme::ux` returns zero in Mere, Graphshell and Turnstone (run first on the
  pre-deletion tree to show the grep finds them); `ux-events` and Mere's workspace build with `SurfaceId` in place.
- Needs: P6 for the address; for the Mere half, nothing: it is independent and may land before the cutover.

### P9. Automation: authored keys, taproot reads the envelope, `act` by action id

Producers that can carry an authored key do (D5). The carrier is open question 2. Taproot's selector reads the key
through the optional fields of P4, with role and name as the fallback it uses today. The runner's `act` gains an
action-id form that invokes the id the envelope carries, and keeps the label form where no id exists. Pixel clicks stay
only where nothing semantic exists. Named in P9: the selector token, the `act` argument form, and whether mandrake's
restated grammar (`wheel::parse_selector`, diagnostics plan M9) is replaced.

The baseline, Turnstone's scenarios at the audit base: `act` by palette label 576 lines; `click-at` by pixel 54;
`assert a11y` by text line 37 (in 18 files). Class, role and `data-` filters over 176 `click` lines are 109, 67 and 71
(the groups overlap). The count is re-measured at the start with the same filters.

- Done when, counted with the baseline's method:
  - the `act` lines that name by label a command which carries an action id in the envelope reach zero;
  - the `click-at` lines on a node that has an authored key or a role and name reach zero, and each remaining line is
    listed with the reason nothing semantic exists;
  - the `assert a11y` lines that assert on one node which has a key or a role and name reach zero; whole-tree text
    assertions may stay and are listed;
  - an `act` by id against a retired producer is refused at its owner after the node id is reused;
  - new scenarios add none of the three forms where a semantic one exists.
- Needs: P1, P4; P2, P3 and P5 for the ids and keys to exist; open questions 2 and 3.

### P10. The JSON snapshot and the read-only MCP resource

A function dumps a tree with its envelopes as JSON, read-only, for diagnostics and the MCP surface (D6). It is not a
contract: no stability promise, nobody builds against it. Home of the function is named in P10. The MCP resource is in
Turnstone (`graphshell-mcp` moves there under `mere:engine-split D23` and D104), so that half is a Turnstone change. It
exposes the snapshot as a resource and adds no tool that invokes an action.

- Done when: a node a product marked for redaction appears redacted in the snapshot and in the MCP resource, and the
  snapshot code has no path that reads node text before the product's redaction (tested with a planted label); the
  MCP server's tool list has the same tools as before; a golden snapshot test exists and the file is JSON; the module
  docs say it is read-only tooling with no stability promise.
- Needs: P1, P6; the MCP home (Turnstone) after the cutover; open question 10.

### P11. Promotion: `foreign_a11y` and the generic bridge parts move into `cambium-winit-a11y` (touches turnstone.git)

The largest change in this work, done as its own phase with headed checks. The retained manager for nested foreign
forests, `foreign_a11y` (799 non-test lines in ten files in Turnstone), moves into `cambium-winit-a11y` as its
guest-tree support, delivering the guest half of the diagnostics plan's M4 ("the publisher is ROOT-only and refuses
them"). The OS bridge's generic parts follow (`turnstone/shell/a11y_bridge.rs`, 323 lines): the cross-thread action
queue and the generation map. Turnstone keeps only its `Admission` variants (pool document, contributed pane). There is
then one publication path and one id policy: the publisher's minted ids reach the OS, and path-hashed ids stop reaching
it. The Cambium publisher takes grafted guests, which moves its address to the `(TreeId, NodeId)` form (P6).

- Done when:
  - the publisher accepts a root tree with graft nodes whose guests are declared and refuses an undeclared one;
  - Turnstone publishes through `cambium-winit-a11y` and the count of its direct `AccessKitBridge` publication sites is
    zero;
  - a test asserts that no `NodeId` handed to the OS bridge in a Turnstone frame equals `node_id_for_path` of a known
    chrome path, and fails on the pre-move code first (the instrument is proved before the negative is believed);
  - the stale-admission test from the old bridge passes in its new home;
  - `foreign_a11y` is gone from Turnstone's source except the admission variants;
  - the headed receipts that engine-split P3's done-condition uses (Turnstone and Pelt) pass again, and a nested foreign
    surface is published and its action routed to the supplier, headed;
  - the P6 Findings list shows the Cambium publisher on the pair form.
- Needs: P5, P6; the slice 4 cutover and the headed pass of engine-split U6 (`mere:engine-split D113`), which give the
  baseline; open questions 6, 9, 11 and 12.

## Decisions

Each records the evidence put to Mark, what was offered, his answer in his words, what follows, and an outcome. Numbers
come from the briefs and are cited there. D1 to D9 are in answer order. D10 records the P5 ruling.

- **D1. The spine is federated (Mark, 2026-11-11):** Evidence (second brief §0, §3.4, §4, §6.1; E1, E2): four semantic
  models already sit above AccessKit (Genet's `DocumentA11yProjection` with 49 roles, Cambium's `ProducerSemantics`
  with 5, the canvas's `CanvasDescription`, graphshell-client's `AccessibilityTree` with 3), and four side tables
  already hold the fields AccessKit drops (`PublicationBindings`, `A11yRoutes` with the bridge's admission map, Pelt's
  `action_map`, `DescribedKeyRegistry`). Estimated existing lines in the path: option 2 about 6,000 to 9,000, federated
  about 1,000 to 1,500 (estimates). Offered, fork A: federated; option 1 (address and lint only); option 2 (uxtree owns
  a node type); option 2 through Genet's projection. Earlier, on the first brief's fork 1, he had said: "1 is
  acceptable but i am intrigued by 2 if that is the correct boundary to draw, architecturally. i don't mind upheaval
  clearly". After the second pass: "Federated (Recommended)". As put to him: AccessKit stays the node type; one shared
  envelope type that every producer fills (address, stability class, provenance, action ids, domain reference,
  authored key); it consolidates four existing side tables. What follows: P1 builds the envelope, P2 to P5 have each
  producer fill it, and P0 tests the form first. If the P0 consumer needs per-producer branches the form is not
  holding and this decision returns to Mark. Answers first-brief forks 1 and 3 and second-brief fork A. Outcome: new;
  refines `mere:engine-split D53` (the spine is an envelope and its composition, not one tree or one node type).

  Links: area The stack · draws on mere:engine-split D53 · plans uxtree-spine P0, uxtree-spine P1 · summary uxtree owns one envelope every producer fills beside its own AccessKit tree; AccessKit stays the node type; prototype first.

- **D2. The types live in uxtree; Genet stays out (Mark, 2026-11-11):** Evidence (E3; second brief §6.2): uxtree
  depends on `inker`, `inker` on `document-session-api`, and Genet keeps no upward edge, so Genet naming uxtree would
  close a loop; the lockfile scan answers it, a `cargo metadata` check was not finished. `taproot` reads through
  `SemanticLookup`, a trait it owns. Offered, fork B: in uxtree with Genet out; a new leaf crate; in
  `document-session-api`; in uxtree with `inker` behind a feature. Answer: "In uxtree; Genet stays out
  (Recommended)". What follows: the types live in uxtree; Genet keeps its projection as it is; the host fills the
  envelope for Genet-backed nodes; taproot's `SemanticNode` gains optional fields to read it (P4). Outcome: new;
  applies `mere:engine-split D53` ("stays whole") and Genet's no-upward-edge rule.

  Links: area The stack · draws on uxtree-spine D1, mere:engine-split D53 · plans uxtree-spine P1, uxtree-spine P4 · summary The envelope types live in uxtree; Genet's projection is unchanged and the host fills the envelope for Genet-backed nodes.

- **D3. The address ends as `(TreeId, NodeId)` (Mark, 2026-11-11):** Evidence (E4; second brief §6.3): only Turnstone
  grafts; the Cambium publisher refuses grafts (the guest half of the diagnostics plan's M4 is open) and Pelt mints one
  tree; app composition AC2 rules graft as the join for retained sessions. Offered, fork C: "producer instance, local
  id" lowered to the pair where grafted; the pair only, which commits Pelt and the publisher to grafting first; both,
  with the pair as the target. Answer: "Both, (TreeId, NodeId) as the end state (Recommended)". What follows: today an
  address is (producer instance, local id); it becomes `(TreeId, NodeId)` as each producer is grafted, and the address
  type holds both until then (P6, P11). Graft keeps each tree's ids apart, so whether D53's words "one ID space" are
  restated is open question 4. Outcome: refines `mere:engine-split D53` (the graft direction: the join is by address
  pair); applies app composition AC2
  ([brief](../../cambium_docs/research/2026-10-06_app_composition_brief.md)).

  Links: area The stack · draws on uxtree-spine D1, uxtree-spine D2, mere:engine-split D53 · plans uxtree-spine P6, uxtree-spine P11 · summary An address is (producer instance, local id) until its producer is grafted, then (TreeId, NodeId).

- **D4. Four declared classes of id (Mark, 2026-11-11):** Evidence (second brief §3.3; E1): three producers issue lives
  (Cambium, Pelt, canvas), two scope by revision or epoch (Genet, graphshell-client), one derives from a path and keeps
  its generation outside the projection (Turnstone's bridge); the diagnostics plan forbids a frame counter as a life.
  Offered, fork D: four declared classes with an owner generation for Derived ids; Life and Scoped only, retiring path
  hashes as published ids; no class field. Answer: "Four declared classes (Recommended)". What follows: Life (never
  recycled), Scoped (valid within a revision or epoch), Derived (path-hashed, with uxtree owning a generation),
  Authored (a key someone wrote). The producer declares the class; the owner rechecks every action; an Authored key
  is resolved to a current id at the moment of use and the id is never held (P1, P5). Outcome: new; applies the
  diagnostics plan's stale-target rule.

  Links: area The stack · draws on uxtree-spine D1 · plans uxtree-spine P1, uxtree-spine P5 · summary Ids carry a declared class: Life, Scoped, Derived with a uxtree-owned generation, or Authored; owners recheck every action.

- **D5. Automation addresses authored keys and role and name, and acts by action id (Mark, 2026-11-11):** Evidence
  (first brief F8; second brief §3.2, §6.5; E5): Turnstone's scenarios use `act` by palette label 576 times,
  `click-at` by pixel 54, and `assert a11y` by text line 37; of 176 `click` lines 109 use a class token, 67 a role and
  71 a `data-` filter; action ids exist only where a producer advertises commands; UI Automation and Playwright give
  precedent for an authored id beside a minted one. Offered, fork E: authored keys plus role and name with `act` by id;
  path tokens over Derived ids; keys only. Answer: "Authored keys plus role and name; act by action id
  (Recommended)". What follows: a node is addressed by an authored key where one exists, else by role and name as
  taproot does now; `act` invokes a `hest` action id where the envelope carries one; pixel clicks stay only where
  nothing semantic exists (P9). Answers first-brief fork 4. Outcome: amends `mere:engine-split D53`'s second
  direction, which said scenarios address native surfaces "by uxtree path".

  Links: area The stack · draws on uxtree-spine D1, uxtree-spine D4 · amends mere:engine-split D53 · plans uxtree-spine P9 · summary Scenarios address an authored key, else role and name, and act by action id where one exists; pixel clicks only where nothing semantic exists.

- **D6. A JSON snapshot for tooling, and no more (Mark, 2026-11-11):** Evidence (second brief §6.6; E5, E6; first brief
  fork 9): guests see no tree (`app-host` emits named actions, `block-host` returns a view); the remote path carries
  chirograph semantics; no consumer reads the envelope across a boundary today; program-written text is JSON. Offered,
  fork F: no, for now (recommended); a JSON snapshot for tooling; a wire contract. He chose, against the
  recommendation: "A JSON snapshot for tooling". As put to him: the tree with envelopes can be dumped as JSON for
  diagnostics and the MCP surface, read-only, not a contract anyone builds against. What follows: the 2026-07-22
  refusal of uxtree as a wire contract stands. The snapshot is not a contract because it is read-only, it is for
  tooling, and it carries no stability promise. The diagnostics plan's redaction rule applies to it: products redact
  before buffering (P10). This answers the first brief's fork 9 as far as a read-only resource; invoking actions over
  MCP is not ruled (open question 5). Outcome: new; applies the 2026-07-22 refusal and the diagnostics plan's redaction
  rule.

  Links: area The stack · draws on uxtree-spine D1, mere:engine-split D23 · plans uxtree-spine P10 · summary The tree with envelopes can be dumped as JSON for tooling and a read-only MCP resource, with redaction; it is not a wire contract and the 2026-07-22 refusal stands.

- **D7. The lint: structure plus labels, bounds and unrouted actions, as a library (Mark, 2026-11-11):** Evidence
  (first brief F5, F7, fork 6; E5): three checkers exist and share nothing (`GraftError`, `NativeIds::prepare` with a
  26-line structural core, a test-only `traverse`); five `ux:` channels are declared in `mere-registry` and
  `git grep` finds no emitter; the June 2026 plan asked for this audit and no record shows it landing. Offered, fork
  6: checks structural only, plus labels, bounds and unrouted actions, plus stability across a frame pair; place a
  library, also gating publication, or a scenario verb only; emission by a sink trait, by the registry directly, or by
  `UxEvent`. Answer: "Structure plus labels, bounds and unrouted actions; a library (Recommended)". What follows: the
  checks are duplicates, cycles, dangling nodes and unreachable nodes, plus missing labels, missing bounds, an action
  with no route, and a node with no envelope. A side-effect-free library that hosts and tests call, with a scenario
  verb; it reports through a sink trait, with the registry's existing `ux:` channels as the first sink. Frame-to-frame
  stability checks come later, once stability classes exist in code. It does not gate publication (P7). Outcome: new;
  applies `mere:engine-split D53`'s third direction and `mere:engine-split D26` (diagnostics stays a library).

  Links: area The stack · draws on uxtree-spine D1, uxtree-spine D4, mere:engine-split D26 · plans uxtree-spine P7 · summary A side-effect-free lint library checks structure, labels, bounds, unrouted actions and missing envelopes, reports through a sink trait with the registry first, and does not gate publication.

- **D8. Events gain the address; the two dead pieces retire (Mark, 2026-11-11):** Evidence (first brief F6, F7, fork 7;
  E5): a parked chain of about 3,000 lines with no consumer: `ux-events` (2,183 lines, kept by D39),
  `incipit::forme::ux` (458 lines, no caller in three repositories), and the kernel's `descriptor_for` (376 lines in its
  file, whose own doc says "Today no host calls this"); `UxEvent::SurfaceOpened` names a surface kind and never an
  instance. Offered: leave it parked; add an optional address to `UxEvent`; retire the two dead pieces; both. Answer:
  "Events gain the address; the two dead pieces retire (Recommended)". What follows: `UxEvent` gets an optional node
  address from the envelope; `incipit::forme::ux` and `descriptor_for` are deleted; `ux-events` and `SurfaceId` stay
  (`mere:engine-split D39` keeps a crate with no dependents). `descriptor_for` is in `mere-kernel`, which stays in
  Mere: that deletion is a Mere change and touches `mere:engine-split D59`'s "no kernel code moves" only as a deletion
  (P8). Outcome: new; applies `mere:engine-split D39`; amends `mere:engine-split D59` by one deletion.

  Links: area The stack · draws on uxtree-spine D3, mere:engine-split D39 · amends mere:engine-split D59 · plans uxtree-spine P8 · summary UxEvent gets an optional node address; incipit::forme::ux and the kernel's descriptor_for are deleted; ux-events and SurfaceId stay.

- **D9. The generic a11y machinery is promoted into Cambium (Mark, 2026-11-11):** Evidence (first brief fork 8, F2;
  second brief §2.2, §6.7): two OS publication paths with two id policies, Cambium's minted ids and Turnstone's path
  hashes; `foreign_a11y` (799 non-test lines in ten files) names surfaces by UUID and trees by `TreeId`, and is the
  guest half that the diagnostics plan's M4 says the publisher lacks; Turnstone's bridge (`a11y_bridge.rs`, 323 lines)
  has generic parts (an action queue, an admission map keyed by `(TreeId, NodeId)`, `safe_initial`) and app-specific
  `Admission` variants. Offered: promote only what has a second user; promote the readers into uxtree; fold the bridge
  into `cambium-winit-a11y` for one publication path; audit first. Answer: "foreign_a11y and the generic bridge parts
  into Cambium (Recommended)". What follows: `foreign_a11y` moves into `cambium-winit-a11y` as its guest-tree support,
  delivering the diagnostics plan's M4; the OS bridge's action queue and generation map follow; Turnstone keeps only
  its admission variants. One publication path and one id policy; path-hashed ids stop reaching the OS. It is the
  largest change in this work, done as its own phase with headed checks (P11). Outcome: new; refines the diagnostics
  plan's M4 (the guest half is delivered here) and settles `mere:engine-split D53`'s fourth direction, the promotion
  audit.

  Links: area The stack · draws on uxtree-spine D3, uxtree-spine D4, mere:engine-split D102 · plans uxtree-spine P11 · summary foreign_a11y and the bridge's generic parts move into cambium-winit-a11y as guest-tree support; one publication path and one id policy; headed checks.

- **D10. One shared generation map in uxtree (Mark, 2026-11-18).** D4 and D9 use the same map implementation.
  Turnstone owns an instance for each window in P5. In P11, Cambium owns the instance for the moved native
  publisher and uses uxtree's map. Each publisher supplies its admission equality and rechecks its own source.

## Open questions

Each is a question for a later round, with its evidence. None is answered here.

1. **Can one envelope serve a minted id and a path-hashed id without a consumer knowing which?** Read, not run
   (second brief §7). P0 is the test. If it fails, D1 returns to Mark.
2. **What carries an authored key?** `data-key` and `data-tabkey` are in use in Turnstone's DOM (first brief F8). The
   brief marks Turnstone chrome as "no": it is built without a DOM (second brief §3.2). The alternatives named were an
   AccessKit node's `author_id` or a defined property in the tree (first brief fork 4). Not tested. Decides P9.
3. **Do `hest` action ids name Turnstone's `Action` variants?** `hest` has 44 typed ids (1,359 lines). Turnstone's
   `Action` has about 190 variants by a line count and does not use `hest`. No mapping exists anywhere to measure. D5
   says `act` invokes a `hest` id where the envelope carries one, so the mapping's owner and scope decide how many
   scenario lines can move.
4. **Does D53's "one ID space" stay?** Graft keeps each tree's ids apart under a `TreeId`, by design (`graft.rs:7-25`);
   D3's end state is the pair; `taproot` builds no tree. The first brief's fork 2 proposed "one address space" as the
   restatement. Whether D53's text is restated, and by whom, is not decided.
5. **Should an MCP tool invoke a tree node's action?** D6 rules a read-only resource only. The first brief's fork 9
   option (c) would add a tool through the same route table and stale checks; the June 2026 plan imagined a typed agent
   harness. `graphshell-mcp` lives in Turnstone after D23.
6. **Where does the generation map live?** Settled in D10.
7. **Is the Derived hash pinned?** `DefaultHasher` is unspecified across Rust releases; the toolchain is pinned today
   (first brief F4). D4 calls Derived an address and not a life; no ruling says whether the hash is specified.
8. **Does NativeIds::prepare call the shared structural core?** P7 shares the structural checks and
   preserves the existing refusal mapping. The extra advisory findings never enter native admission.
9. **Does Pelt graft?** D3 says the address becomes the pair "as each producer is grafted". The publisher grafts in P11.
   Pelt mints one tree with a counter from 2^63 per tile; no ruling asks it to graft (app composition AC2 names Pelt's
   combined tree as a user of the helper).
10. **What does the snapshot's schema version promise?** The second brief's option committed to "a versioned schema";
    D6 makes no stability promise. Whether the snapshot carries a version field, and what it then means, is open.
11. **Can inker surfaces fill an envelope?** The suppliers behind `foreign_a11y` publish AccessKit trees only, and the
    second brief did not read inker's surface side. Whether a supplier tree's nodes are Scoped, Life or unenveloped
    (and so reported by the lint) is not known. Also open: the 174-line `turnstone/shell/foreign_a11y.rs`, the supplier
    side, which D9's 799 lines do not include.
12. **Which platforms count as headed for P11?** Windows is the primary machine; macOS and Linux are validatable on
    the other machines. The headed set is not ruled.
13. **Does P11 land before or after the Cambium publish?** The [cambium publish plan](2026-11-05_cambium_publish_plan.md)
    (D7) publishes the Cambium chain once, from Graphshell, after engine-split P3. P11 changes `cambium-a11y`'s public
    surface.

## Findings

### P6 address forms by producer

| Producer | Address at this phase |
|---|---|
| Cambium native publication | Producer owner and minted `NodeId`, assigned by `NativeIds`. The publisher refuses grafts until P11. |
| Retained Genet document read through Pelt core | Producer session and the document projection's local `NodeId`. Scope comes from the engine revision. |
| Pelt desktop document | Producer window, tile and session with the minted node ID in the window's tree. |
| Canvas through native publication | Producer owner and minted node ID. The graph member UUID travels separately as the domain reference. |
| Graphshell mounted scene | Producer session and scene-instance node ID, scoped to the scene epoch. |
| Turnstone host chrome and shared frame | Producer window root and local projected ID. Lens remapping runs before publication. |
| Turnstone's retained documents in the host tree | Installed `(TreeId, NodeId)`. Each placement graft keeps the document's local IDs. Its captured route checks source and placement. |
| Turnstone foreign surfaces, including nested guests | Installed `(TreeId, NodeId)`. The current owner table selects the surface, then the retained tree resolves the local node. |
| Turnstone contributed panes | Installed `(TreeId, NodeId)`. The pane fills the address from the subtree it publishes. The source and placement still govern admission. |
| Document, frozen, Gloss and Platen observation views | Caller-supplied producer instance and path-derived node ID. |


- 2026-11-17: P1 defines the six metadata fields and all four classes without importing any host
  types or changing uxtree's manifest. Generic instance/provenance/domain payloads preserve a
  reader's existing common identity and weak witnesses. The mixed-source P0 gate still reads all
  13 nodes and passes its three controls using the production types. Unit tests read the actual
  Envelope declaration to guard the six-field boundary and catch planted AccessKit fields.
- 2026-11-17: actual source mutations verify the P1 tests: making Scoped return Life, dropping an
  Authored key and adding a Role field each fail at the intended assertion. The control runner
  restores the exact source bytes after each experiment. Metadata constructors neither infer
  classes from nodes nor grant action admission; later owners retain their checks.
- 2026-11-17: the P1 source-manifest cycle check covers 29 actual family checkouts with this
  worktree substituted for Graphshell, finding no strict/dev cycle or stale allowlist entry.
  Existing resolved-lock warnings at 14 roots remain distinct from this manifest result.
- 2026-11-17: P0 reads 13 joined nodes through `tests/e1a_spine/consumer.rs`, which names no producer
  kind. Life provenance comes from Rootstock's `SemanticNodeIdentity` and bound `A11yRequest`;
  Scoped revisions come from the actual Genet document session; Derived ids come from uxtree paths
  with a retained route-owner life. Missing metadata is checked against the AccessKit consumer's
  actual traversal, including both grafted trees, rather than against the metadata map alone.
- 2026-11-17: retiring and replacing a texture guest under the same DOM slot really reuses the
  generated command `NodeId`. The owner refuses the old publication even when its retired model
  remains alive. Recapturing today's provenance for that old address is a deliberately broken
  control and trips the stale-target assertion. Freezing Genet's scope and disabling the missing
  metadata check also trip their own assertions. These controls are test-local.
- 2026-11-17: Genet's session produces a semantic projection only after retained layout exists.
  P0 obtains a real frame before reading it; a value edit advances its revision and Genet itself
  rejects the old action request. Its neutral projection exports action kinds, but neither command
  IDs nor authored DOM keys; those optional envelope fields remain empty in this adapter.
- 2026-11-17: a Rootstock `TextureProducer` contributes semantics through a `custom-leaf` slot.
  The raw `external-texture` element is not in `owned_layout::custom_leaf_key` and does not take
  this semantic path. P0 uses the supported slot; it adds no production API or rendering fallback.
- 2026-11-17: the independent Mere half of P8 is a pushed candidate, Mere `13a084dde`, removing
  `descriptor_for` and its unused AccessKit support while retaining `SurfaceId`. It is not yet on
  Mere main; the rest of P8 remains open.
- 2026-11-11: the semantic model above AccessKit exists four times and the envelope's fields are kept four times
  (second brief §0, E1, E2). The canvas outputs Cambium producer semantics, not AccessKit, and `stitch` concatenates
  subtrees and does not rehash them (second brief §1; it corrects the first). Two subtrees built from one path collide
  unchecked (E4).
- 2026-11-11: Genet cannot name uxtree (E3). The check was a lockfile scan; the `cargo metadata` run was stopped on
  another session's package-cache lock. P4's done-condition repeats it.
- 2026-11-11: `descriptor_for` and `SurfaceId` share one file. Read from mere `0113c39d1`,
  `crates/graph/graph-kernel/src/accessibility.rs` (376 lines): `SurfaceId` at line 49, `AccessibilityDescriptor` at
  104, `descriptor_for` at 138, tests from 305. The 376 in D8 is the file, not the function; P8 deletes the function
  and what only it uses.
- 2026-11-11: the briefs disagree on two counts. Turnstone's scenario files are 181 (first brief §3) and 172 (second
  brief §6.5 and E5); the verb counts (576, 54, 37) are the same in both and are the baseline. `foreign_a11y` is "about
  1,000 with its parts" (first brief fork 8) and 799 non-test lines in ten files plus 174 in `shell/foreign_a11y.rs`
  (second brief §1); D9 uses 799.
- 2026-11-11: the five `ux:` channels are declared and referenced only by the descriptor catalog
  (`host/crates/system/registry/src/diagnostics/descriptor/catalog/phase3a.rs:377-392`); `git grep` found no emitter in
  Mere, Graphshell or Turnstone (E5). P7 repeats the grep at its start.
- 2026-11-11: action ids are the weakest envelope field. DOM-derived producers know an action's kind and not the
  command it runs; only the canvas, graphshell-client and Cambium texture producers have ids (second brief §3.2). The
  envelope carries an id where one exists and cannot make one up.
- 2026-11-11: prior documents and what the rulings do to them.
  - 2026-07-22 remote projection plan, §4.3 and the promotion table: applies, untouched (D6, D1, D2). Nothing crosses a
    wire; the types live in uxtree, which the plan kept "for now".
  - Diagnostics plan, stale-target rule: applies (D4). "No second list of relation properties": applies (D1).
    Redaction: applies (D6). M4: refined (D9 delivers the guest half). M5: untouched; its one-model goal stays inside
    Cambium (D1).
  - App composition AC2: applies (D3). The briefs note Pelt and the publisher do not graft; open question 9.
  - `mere:engine-split D53`: refined (D1), refined (D3), amended (D5), applied (D7), settled (D9). D39: applied (D8).
    D59: amended by one deletion (D8).

## Progress

- 2026-11-11: plan opened; D1 to D9 recorded from Mark's answers to the two briefs; P0 to P11 laid out; nothing built.
- 2026-11-17: P0 locally qualified in a review worktree. The mixed-source gate and three deliberately
  broken controls pass alongside the six existing E1a tests; the accessibility and uxtree supplier
  suites pass together. The source-manifest cycle check passes with this candidate substituted for
  the primary Graphshell checkout. No production envelope or platform acceptance is claimed.
- 2026-11-17: P0 integrated with Graphshell main `18744fff`; the 67 supplier tests pass again on the
  combined tree. Mark authorized the merge and continuation to P1. The primary checkout's parallel
  source work remains untouched; the integration is delivered through the verified worktree.
- 2026-11-17: P1 locally qualified: public envelope types, declared-class constructors, per-field
  documentation and the adapted E1a gate. The supplier suites pass 74 tests including the new
  unit and documentation tests; source mutation controls and native/wasm all-feature library checks
  pass. Production publishers, generation maps, lint and human AT remain later phases.
- 2026-11-17: Mark authorized the P1 merge and continuation to P2. The P1 candidate was combined
  with main `f49826ce` without conflicts; all 74 supplier tests pass again on the combined tree.

- 2026-11-17: P2 baseline: 240 Rootstock tests pass (18 existing ignores). The native
  publisher freezes metadata with routes, remaps envelope addresses to native IDs and retires
  both together. The complete native walk passes; the source-manifest cycle check finds no strict
  or development cycles across 29 repositories. Receiving canvas checks and broken controls remain
  in progress. The raw packet is `Code/testing/graphshell/uxtree-p2/`, capped at 100 MB total.
- 2026-11-17: P2 controls: removing native envelopes, leaving source addresses, dropping a
  producer command or domain reference, and rebinding an old request each fail their runtime
  assertions. The envelope route rejects a replaced child even while its old marker is retained.
  An exploratory provenance-only bypass was still refused by the independent producer token check;
  its log is retained alongside the stronger caught rebind control. Final supplier/receiving gates
  and the Turnstone fresh resolution remain in progress.
- 2026-11-17: P2 restored supplier qualification passes 340 tests (19 existing ignores),
  followed by 10 desktop accessibility tests and the Rootstock/browser host wasm library check.
  Turnstone receiving qualification and normal candidate hooks remain in progress.
- 2026-11-17: P2 normal hooks passed all three Clippy checks; dependency bans found
  `peer-facts` 0.0.1 from the older Dramatis holder alongside Armillary Git 0.0.3. Current
  Dramatis main already uses the Armillary source. Refreshing that Git holder through Cargo,
  retaining the before/after resolution receipts, then re-running the affected gates. No skip
  or manual lock edit is introduced. Turnstone is materializing its fresh Genet source checkout.
- 2026-11-17: Cargo refreshed the existing Dramatis Git source from `2c779ded` to
  current main `9185d150`. The 12 family packages moved together and registry `peer-facts`
  0.0.1 disappeared. Before/after package IDs and locks are retained; manifests, source
  patches and dependency-ban policy are unchanged. Normal hooks are being retried.
- 2026-11-17: P2 supplier candidate `5c390762` is pushed with normal hooks passing after
  the holder refresh. Turnstone passes 13 canvas reader tests, a broken-reference source control,
  the restored 13-test suite and its browser-target library check. Six receiving contract packages
  resolve to the candidate Graphshell checkout. The final source cycle gate passes across 29 repos.
- 2026-11-17: Review removed an unnecessary assumption that every native ID must numerically
  differ from its source ID. The native test checks the final tree address and retained provenance;
  all 29 tests pass again, and omitting address finalization still fails the revised test. Receiving
  hooks have passed both Clippy checks and the root bans; the web workspace is fetching its own
  newer Genet revision before its bans run. These source cohorts are retained separately.
- 2026-11-17: P2 Turnstone candidate `219c99b1` is pushed. Both Clippy hooks, root/web/relay
  dependency bans, house patch checks, resolved-version recording and push hooks pass. Native tests
  and the wasm check use Genet `0cd7e98`; the separate web dependency-ban resolution uses `fe42f35`.
  Exact locks and source IDs are retained for each claim. The root receiving package IDs did not
  change through the hook run. Land Graphshell first, then refresh Turnstone's Graphshell Git source
  through Cargo and integrate the receiving change; all three standalone roots retain their own locks.
- 2026-11-17: P2 cleanup: the P1 worktree and feature branches were removed after integration.
  The two P2 worktrees remain for review. Automatic approval review rejected removal of the named
  temporary Cargo home and a narrower literal junction unlink, returning only "blocked by policy".
  `C:/t/cargo-homes/turnstone-uxtree-p2` remains owned by P2 qualification (325,055 bytes of local files,
  plus junctions to shared caches). Four gate-created Git cache origins were normalized to their
  persistent cache paths. Shared targets and source caches are preserved. Cleanup details and the
  complete qualification packet are under `Code/testing/graphshell/uxtree-p2/` (100 MB cap).
- 2026-11-17: Final integration snapshot: Graphshell main advanced to `a9c12694` and
  Turnstone main to `66716cd5` during dependency qualification. Both merge cleanly into the
  review candidates. The combined trees pass the 340 supplier tests, 10 desktop tests,
  13 canvas tests and the Turnstone wasm check again. Normal merge hooks are being run.
  The original report/hash inventory is frozen; combined-tree receipts use separate filenames.

- 2026-11-17: Graphshell P2 landed at `7c40580e`. Turnstone resolved the four envelope
  dependencies to that exact published source. Native and wasm receiving checks are in flight.
  P3 is replacing Pelt document routes with envelope provenance and filling client item envelopes
  in graphshell-host.

- 2026-11-17: P3 native qualification passed 85 Pelt and 26 graphshell-host tests. Three
  source controls caught a missing Pelt tile, a missing client source and revision substituted for
  scene epoch. The source-cycle check passed across 29 repositories. The Pelt test also distinguishes
  the same tile and session observed through two windows.
- 2026-11-17: The no-default-feature graphshell-host library builds for wasm. Browser
  execution and platform assistive-technology acceptance remain later gates.
- 2026-11-17: P2 landed on Turnstone main at `7406f9f3`. The 13 canvas tests and wasm
  library check pass with Graphshell `7c40580e` resolved from Git. The three dependency-ban
  hooks and the resolved-version push passed. P3 candidate `85cdc0af` passed all normal
  code commit hooks. The P3 branch also records this P2 landing.

- 2026-11-17: P3 integrated with Graphshell main `2e041d0b`. The combined tree passed
  85 Pelt and 26 Graphshell host tests. Landing receipts are in
  `Code/testing/graphshell/uxtree-p3/landing-tests.log` and `landing-hooks.log`.

- 2026-11-17: P4 Genet candidate `b75face72c9` is pushed. Taproot passed 47 tests and
  its normal commit gates. A planted opaque address in a built-in lookup fails the new test.
  The full Genet workspace metadata resolves 59 members and finds no uxtree dependency.
  The host has passed an initial retained-document stale-revision test and Pelt core suite with
  a local source override. The published Genet source is being qualified in the ordinary Git
  dependency resolution before the receiving candidate is pushed.
- 2026-11-17: The P4 receiving lock resolves all 37 Genet packages from published
  candidate `b75face72c9`. All 116 Pelt core tests pass in that Git cohort. Replacing
  the captured revision with the live revision makes the stale-write test fail at runtime.
  Restoring the source passes the suite. The family checker finds no strict or development
  cycles across 29 repositories. The review order is Genet first, then Graphshell.

- 2026-11-18: P4 landed in supplier order. Genet main received the exact tested source
  `b75face72c9`. The Graphshell lock matched the final qualification records, and the
  retained-document stale-action test passed again. Both landings use the normal hooks.
- 2026-11-18: D10 settled the P5 boundary. uxtree owns the map implementation and
  each publishing window owns its instance. Turnstone preserves its document and pane
  comparators and captures its host action envelopes at publication. The publishers
  preserve equal admissions and retire changed or removed ones.
- 2026-11-18: Graphshell `a0595668` passed uxtree's 38 native tests and its wasm
  check. Turnstone passed 71 accessibility tests and a palette test against that Git
  revision. Keeping an old generation alive caused each deliberately broken
  stale-admission test to fail. The family checker found zero cycles across 29
  repositories. Turnstone `6f45ffc4` passed the regular commit and push hooks.
  The root and web dependency locks still match the selected supplier after the
  hooks. Land Graphshell before Turnstone. The raw packet is under
  `Code/testing/graphshell/uxtree-p5/`.

- 2026-11-18: P5 landed in supplier order. Graphshell advanced to `5a8d2d1e` after
  38 uxtree tests passed again. Turnstone advanced to `6f45ffc4` after the 71
  accessibility tests, the palette Keep/Drop test and all three dependency-ban
  checks passed with Graphshell main. Both pushes ran the regular hooks.
  The landing receipts are under `Code/testing/graphshell/uxtree-p5/landing-*`.

- 2026-11-18: P6 pairs installed tree IDs with unchanged local node IDs in
  Turnstone's foreign and contributed grafts. The P6 Findings table records
  the producer-instance and installed-tree forms in use.
- 2026-11-18: Graphshell `78c9f4ca` passed 39 uxtree tests, 10 real-session tests
  and the wasm check. Turnstone `563ed234` passed 74 accessibility tests against
  that exact Git supplier. The local-ID-only control failed at runtime and the
  restored pair passed. Both candidates passed their ordinary hooks. The family
  check found zero cycles across 29 repositories. The receipt packet is under
  `Code/testing/graphshell/uxtree-p6/`. Land Graphshell before Turnstone.
