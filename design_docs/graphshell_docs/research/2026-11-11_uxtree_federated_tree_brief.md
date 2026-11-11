# uxtree as a federated tree: second brief

**Date:** 2026-11-11
**Status (2026-11-11):** research brief, complete. Docs only: no code changed, nothing built or run. The forks in
§6 wait for Mark. No decision is taken here and none is written as one.
**Ruling carried:** engine split D53 (`mere/design_docs/mere_docs/implementation_strategy/2026-11-07_engine_split_plan.md`),
and Mark's answer to the first brief's shape question, quoted: "1 is acceptable but i am intrigued by 2 if that is
the correct boundary to draw, architecturally. i don't mind upheaval clearly". The orchestrator then proposed a third
form, a federated tree, as a proposal and not a finding; Mark said "Go ahead and start" this pass. This brief tests
that proposal against the code. It does not defend it.
**Follows:** [uxtree spine brief](2026-11-11_uxtree_spine_brief.md) (the first brief). Evidence tables:
[federated tree evidence](2026-11-11_uxtree_federated_tree_evidence.md) (E1 to E7, cited below as E*n*).
**Audit base:** mere `217dc2b16`, graphshell `d97a69a5`, turnstone `fd57dd3f`, genet `7c6ffc87b`, read from
`origin/main` in detached scratch worktrees (removed afterwards).
**Paths.** Toolkit paths are mere.git's today: `uxtree` is `crates/forme/uxtree`, the Cambium family is
`crates/cambium/*`, the canvas is `crates/canvas/cartography` (`mere-cartography`). They move to Graphshell's `host/`
at the cutover being rehearsed, and will then change. The path key is the evidence file's.

*Reading, not ruled* marks this brief's own inference. Everything else is a quotation, a file, a count or a measured
figure. Cost figures in §4 marked "estimate" are not measurements.

## 0. The result on one page

- **The forms, named.** *Option 1*: an address and a lint, no new tree (the first brief's shapes B and C).
  *Federated*: option 1 plus one shared **envelope** per node (address, stability class, provenance, action ids,
  domain reference), filled by each producer beside its own tree; uxtree owns the envelope and the composition;
  AccessKit stays the node type. *Option 2*: uxtree owns a richer node type that producers build, and AccessKit is
  lowered from it.
- **The semantic model above AccessKit already exists, four times.** Genet's `DocumentA11yProjection` (49 roles,
  serde, a `revision`), Cambium's `ProducerSemantics` (5 roles), the canvas's `CanvasDescription`, and
  graphshell-client's `AccessibilityTree` (3 roles, chirograph actions). Each is lowered to AccessKit in one place.
  Cambium already has two outputs from its model: AccessKit and an ARIA DOM mirror (E1, E2).
- **The envelope already exists, four times.** Cambium's `PublicationBindings` ("metadata, not another tree"),
  Turnstone's `A11yRoutes` plus its bridge's admission generations, Pelt's `action_map`, and the canvas's
  `DescribedKeyRegistry`. Each keeps, beside the AccessKit tree, the fields AccessKit drops (E1, E2).
- **Three kinds of id.** Minted and never recycled (Cambium, Pelt, canvas), revision-scoped (Genet,
  graphshell-client), path-hashed (uxtree, Turnstone). Every producer refuses a late action at its owner; the
  path-hashed one can do so only through a generation its OS bridge keeps beside the tree (E1).
- **Genet cannot name uxtree** (uxtree → inker → `document-session-api`, and Genet keeps no upward edge). After the
  cutover every other producer sits in Graphshell's host workspace beside uxtree (E3).
- **Verdict (reading, not ruled).** The federated form is a real third option, but a narrow one: it is option 1 plus
  one commitment, a shared envelope type that every producer fills. That commitment consolidates four existing side
  tables; it adds no node type. Option 2 buys one role and state vocabulary and one traversal outside AccessKit. The
  stack already has one traversal (AccessKit's, read by `taproot` and `forest_lines`), and a single vocabulary would
  have to absorb Genet's 49 roles, which only Genet can emit. §4 lays out the test.
- **Six forks** (§6), down from nine. The first brief's forks 6 to 9 carry over (§6.7).

## 1. What changed since the first brief, and what this pass read

**Changed.** Mark answered the shape question (option 1 acceptable, option 2 intriguing, upheaval acceptable). The
federated form was proposed. Graphshell's `origin/main` moved to `d97a69a5` (the first brief's own commit); Mere
moved from `446750cf9` to `217dc2b16`; `git diff --stat` between the two shows no change to uxtree, `cambium-a11y`,
rootstock's `semantics.rs` and `a11y_provenance.rs`, the canvas `reader.rs` or `canvas_reader.rs`.

**Read through this pass, sized only by the first:** Turnstone's `shell/a11y_bridge.rs` (323) and the
`foreign_a11y` family (799 non-test lines in ten files, plus `shell/foreign_a11y.rs`, 174); `rootstock`'s
`semantics.rs`, `a11y_provenance.rs`, `producer.rs`; `cambium-a11y`'s `publication.rs`; Genet's
`document-session-api/src/a11y.rs` and the AccessKit lowering in `genet-render`; Pelt's `accessibility.rs` namespace;
the canvas `reader.rs` and the reference app's `canvas_reader.rs`; graphshell-client's `AccessibilityTree`;
`taproot`'s `lookup.rs`; the two script hosts.

**Read for the first time:** graphshell-client's `AccessibilityTree` and chirograph's `AdvertisedAction` as a
semantic model. Neither appeared in the first brief's tables. They are the "neutral semantic payload" the 2026-07-22
remote projection plan asked for, in a scene form (§3.4, §5).

**Corrections to the first brief.**
- `stitch` does not hash subtrees into one id space. It concatenates their node lists under a new root
  (`uxtree/lib.rs:111-135`). The subtrees' ids are whatever their builders made; two built from one path collide, and
  nothing checks (E4).
- The canvas does not output AccessKit. `mere-cartography` has no AccessKit dependency. Its reader is
  target-neutral and is lowered to Cambium producer semantics in the reference app (`gs-port/canvas_reader.rs`).
- The first brief's F1 said "the shared tree is AccessKit's". True at the output. Above it, four producer models
  exist (§0), which the first brief did not count.

## 2. The producers and uxtree's consumers

### 2.1 Comparison

Condensed from E1. "Promise" is what an id is kept for. "Stale check" is where a late action is refused.

| Producer | Source of truth | Id | Promise | Stale check | Side table | Domain ref | Action id |
|---|---|---|---|---|---|---|---|
| Cambium native | Genet DOM + leaves + producers | minted per node, process-wide | session, never recycled | owner, re-captured provenance | `PublicationBindings` | no | producer actions yes; DOM nodes kind only |
| Genet document | DOM | `opaque_id` | object life; `revision` per read | engine, by revision | none (the projection) | no | 6-variant enum |
| Pelt composite | Frisket DOM + tile projections | minted from 2^63 per tile | session, never recycled | workspace, by session, generation, revision | `action_map` | tile | enum bits |
| Canvas reader | graph members | minted per member | member life, witnessed | canvas, by witness | `DescribedKeyRegistry` | member UUID (dropped on lowering) | chirograph intent |
| Turnstone app projection | `App` | path hash | same path, same toolchain | bridge, by admission generation | `A11yRoutes` | frozen members only | Turnstone `Action` |
| graphshell-client | mounted scene | `InstanceId` | scene epoch | endpoint, by epoch and revision | none | `SourceRef` | chirograph intent |

### 2.2 Each, measured

**Cambium native publisher.** `project_semantics` (`rootstock/semantics.rs`, 506 lines) lowers Genet's projection
to AccessKit, overlays producer nodes, and returns a `SemanticProjection` with target maps. `publication_bindings`
then freezes, per node, a `SemanticNodeIdentity`: owner, target (DOM node, or producer slot, key and action id),
document, mount, producer weak handle, child token, leaf lifetime (`a11y_provenance.rs`, 175 lines). Unknown,
duplicate or ended contributors refuse publication. `NativeIds::prepare` (`cambium-a11y/publication.rs`, 123 lines)
checks the tree (foreign tree, incomplete, duplicate, dangling, cycle, unreachable, missing identity) and remaps each
source id to a counter value, reused only while the identity compares equal. Full snapshots; the namespace advances
on install. It publishes the ROOT tree only and refuses any graft node (`Refusal::ForeignTree`). The ARIA path in
`cambium-genet-web-host` reads the same `project_semantics`.

**Genet document projection.** `DocumentA11yProjection` (`dsa/a11y.rs`, 472 lines) is renderer-neutral and serde:
nodes with parent and ordered children, 49 roles, a state record, numeric range, bounds, six actions, plus `revision`
and `DocumentA11ySupport` (capability and stated limitations). Its id is "document-local", "never an AccessKit node
id", and hosts "must namespace it". The engine "must revalidate `revision`, `target`, and that the requested action is
advertised". `accesskit_tree_from_projection` (`genet-render/a11y.rs:507`) lowers it once, and drops the revision and
the support statement. The host can ask for a projection, a click point by revision, and an action by revision. It
cannot ask for authored attributes through the projection; `taproot` reads `data-*` from the DOM directly.

**Pelt composite.** One AccessKit tree per window (`pelt/accessibility.rs`, 1,247 lines). Frisket chrome nodes are
authored there; each focused tile's Genet projection is mounted under its content aperture with ids drawn from a
counter starting at 2^63, one namespace per tile, reset when the tile's session identity changes. The
`WorkspaceA11yActionTarget` beside the tree carries tile, session identity, revision, placement generation, local
node, content rect and action bits, and every action is rechecked against all of them. No uxtree, no graft.

**Canvas reader.** "Minted keys with lifetime witnesses" means: `DescribedKeyRegistry` maps a member UUID to a u64
key taken from a process-wide counter, stored with the kernel's `SurfaceNodeLifetime` for that member. A read reuses
the key only while the lifetime is the same; a removed and recreated member gets a new key even if its UUID and slot
recur. Reads never mint (`reconcile` does). Activation resolves a key to a member only if the witness still matches.
At most 200 items are described, the focused one always among them. The reference app lowers this to
`ProducerSemantics` (`canvas_reader.rs`, 200 lines): key, name, rect, and each action's chirograph intent as the
producer action id. The member UUID is not carried.

**Turnstone app projection.** `project_app` builds AccessKit nodes directly (49 `Node::new` sites in 14 files, 69
`node_id_for_path` sites in 23 files) and stitches chrome, panes and the frozen projection. Routing is the side table
`A11yRoutes`, because "the projection hashes paths into `NodeId`s, which is one-way". A route is a frame control, an
app `Action`, a frozen member, or a contributed DOM node with its pane generation.

**Turnstone's direct OS bridge** (`shell/a11y_bridge.rs`, 323). Generic parts: a cross-thread action queue; an
admission map keyed by `(TreeId, NodeId)` whose generation is kept only while the route and its source compare equal
structurally, so a request captured against an old admission is refused; `safe_initial` (the host root alone, for
activation). App-specific parts: the `Admission` variants (pool document, contributed pane). It sits on the same
`genet_winit_host::AccessKitBridge` as `cambium-a11y`. *Reading, not ruled:* the generation map is Turnstone's
equivalent of Cambium's provenance check, built for path-hashed ids, which cannot carry a life of their own.

**Turnstone's `foreign_a11y`** (799 non-test lines). A retained manager for nested foreign forests: per surface a
root `TreeId`, the declared nested trees, supplier deltas folded into retained trees, a per-window `Grafts`
publication history, a `GraftTable` owner map; malformed known trees withdraw their surface and missing declared
subtrees wait. Supplier `TreeId`s and local `NodeId`s stay intact. The supplier is an inker surface that publishes its
own AccessKit trees (`shell/foreign_a11y.rs`). None of this is Turnstone-specific in its types: it names surfaces by
UUID and trees by `TreeId`. It is the guest-tree half that the diagnostics plan's M4 lists as missing from the
Cambium publisher ("the publisher is ROOT-only and refuses them").

## 3. The envelope, concretely

### 3.1 Fields the consumers need, and nothing more

From E5. Each row names the consumer that needs the field; a field no consumer needs is left out.

| Field | Needed by | Why |
|---|---|---|
| **address**: producer instance and producer-local id | every non-OS consumer; the lint | one way to say "this node" across producers |
| **stability**: a declared class (§3.3) | the lint ("unstable id"); automation (what may be held) | the stale-target rule |
| **provenance**: producer kind, instance, generation | the lint (who to blame); `ux-events` (which pane, not which kind) | `SurfaceOpened` names a kind, never an instance |
| **actions**: per advertised action, a stable id | scenarios (`act` by id, not by copy); `ux-events` (`ActionDispatched { action_id }`) | 576 `act` lines name a label |
| **domain**: an optional `(adapter, id)` reference | `ux-events` (`target: Option<NodeKey>`); diagnostics joins; MCP joins | the graph node a UI node stands for |
| **key**: an optional authored key | automation | the durable address; `data-key` and `data-tabkey` are already used this way |

Not in the envelope: role, name, state, bounds, children, relations. AccessKit carries those, and the diagnostics plan
rules that "AccessKit owns typed node-reference traversal; Mere keeps no second list of relation properties". Not
`revision` either: it is part of the address for a revision-scoped producer (§3.3).

### 3.2 Who can fill each field today

Yes / small change (named) / no (why).

| Field | Cambium | Genet | Pelt | Canvas | Turnstone | graphshell-client |
|---|---|---|---|---|---|---|
| address | yes (source id + owner) | yes (`DocumentA11yNodeId`; host adds the session) | yes (tile + local id) | yes (described key) | yes (path hash) | yes (`InstanceId`) |
| stability | yes: life | yes: revision-scoped life | yes: life | yes: life | small: derived plus the bridge generation (the generation lives in the shell, not the projection) | yes: epoch-scoped |
| provenance | yes (`A11yProvenance`) | partial: `support` says what is covered, not who made it; the host knows the session | yes | small: from the Cambium slot | small: the path root names the part; no instance for panes beyond `PaneId` | yes |
| actions | producer nodes yes; DOM nodes no (kind only) | no: six kinds, no ids | no: kind bits | yes (intent) | small: `A11yRoute::Action(Action)` holds the app action; mapping to `hest` ids is not done anywhere | yes (intent) |
| domain | no: a DOM node knows no graph member | no | partial: tile | small: carry the member UUID through `ProducerNode` (one field, `canvas_reader.rs:73-95`) | partial: frozen members yes, chrome no | yes (`SourceRef`) |
| key | small: read a `data-key` attribute where the projection is built | small: same, in `projection_walk` | small: from the tile's DOM | yes (member UUID) | no: chrome nodes are not DOM | yes (`InstanceId` or source id) |

*Reading, not ruled:* action ids are the weakest column. DOM-derived producers know an action's kind, not which
command it runs; that is decided in the app's click handler. Only producers that advertise commands (canvas,
graphshell-client, Cambium texture producers) have ids. An envelope can carry an id where one exists; it cannot make
one up.

### 3.3 What "stability class" has to mean under the stale-target rule

The rule (diagnostics plan, Constraints): target lifetimes "are never allocated from observation reads, and a frame
epoch, an observed-description counter or a hand-rewritten subset of AccessKit node references is never substituted
for owner-issued lives; a stale target cannot be retargeted because a node identifier was reused".

So a class is declared by the producer, never inferred by a reader; and an action is always rechecked by the
producer. Four classes cover what exists:

- **Life.** The owner issued the id for one object's life and never reuses it. A request on an ended life is refused
  by the owner. Cambium, Pelt, canvas today.
- **Scoped.** The id is the object's for as long as it lives, and each observation carries the owner's revision or
  epoch; a request carries the revision it saw and the owner refuses a stale one. Genet, graphshell-client today.
- **Derived.** The id is a deterministic function of a path. It is an address, not a life. It may be held for
  *finding* a node; an action through it needs an owner-issued generation beside it. Turnstone today, with its bridge
  generation; uxtree's own projections have no generation.
- **Authored.** The app set a key meant to survive runs. Automation resolves it to a current Life or Scoped id at the
  moment of use and never holds the resolved id (the WebDriver stale-element rule the first brief cites).

*Reading, not ruled:* "Derived" meets the rule only paired with a generation, which today lives in Turnstone's shell
and nowhere in uxtree. A federated uxtree would either own that generation (a small owner-side map like the bridge's)
or refuse actions on Derived ids. A frame counter would not do: the rule names it as a forbidden substitute.

### 3.4 The same exercise for option 2

A node type uxtree owns, that producers build, lowered to AccessKit.

**What it would contain.** Everything in §3.1, plus what each producer's model carries today: role, name,
description, value, numeric range, state (Genet's record has 14 fields), bounds, ordered children, actions with
kinds and ids, live-region and popup facts, and the support statement. Its role set must cover Genet's 49, the
producers' 5 and chirograph's 3, and still lower to AccessKit. It must carry no relation list of its own (the rule
above), so relations would still be AccessKit's, on the lowered tree.

**What each producer would change.** Method: E7 lists non-test files naming `accesskit` as an upper bound; the
narrower figure is the file that builds the model. Line counts are of existing code in the path, not of edits.
Estimates are marked.

| Producer | Change | Code in the path |
|---|---|---|
| Genet | Cannot build a uxtree type (E3). Either Genet keeps `DocumentA11yProjection` and the host translates it (then Genet is federated in effect), or the type moves into `document-session-api` and Genet's vocabulary becomes the stack's | `dsa/a11y.rs` 472; `genet-render/a11y.rs` 1,880, of which the lowering is one function |
| Cambium | `project_semantics` builds the new type; `NativeIds` and `remap.rs` (the 16 hand-listed id fields) move to the lowering; the ARIA mirror reads the new type | `semantics.rs` 506, `document_projection.rs` 160, `cambium-a11y` 1,458, web host `a11y.rs` 517 |
| Pelt | Builds the new type for chrome; mounts tile projections through it | `accessibility.rs` 1,247 |
| Canvas | None in cartography; the reference app's lowering changes target | `canvas_reader.rs` 200 |
| Turnstone | Replace direct `Node` construction (49 sites, 14 files) and the path ids (69 sites, 23 files); `foreign_a11y` keeps AccessKit, since inker surfaces publish AccessKit | a11y family 4,222 non-test lines; 32 files name AccessKit |
| uxtree | Its projections and `frozen` build the new type | 1,236 |
| graphshell-client | None required: it is already a semantic model; the host translates | — |

Estimate: some 6,000 to 9,000 existing lines in the path across six crates, and the decision of whose vocabulary
the type is. The federated form, read the same way, touches the side tables only (estimates, from the spans that
define them): `publication_bindings` in `semantics.rs` (about 100 lines), `A11yRoutes` (`a11y.rs`, 136) and the
bridge's admission map (about 80), Pelt's `DocumentA11yActionTarget` (about 60), the canvas lowering (about 25), plus
a new envelope module in uxtree (200 to 400 lines with its tests).

## 4. The three forms tested against each other

| Test | Option 1 (address + lint) | Federated (option 1 + envelope) | Option 2 (one node type) |
|---|---|---|---|
| Source of truth | each producer | each producer | each producer computes; the node type becomes the published truth |
| Id stability, stale rule | address only; each consumer still learns each producer's stale check | the class is declared per node; checks stay at owners | the type would have to carry a life or revision per node; checks still at owners |
| Wire and guest boundaries | nothing new crosses | nothing has to cross; a JSON snapshot for receipts would be new | a serialised tree would invite being a wire format, which the 2026-07-22 plan refused for uxtree |
| Genet | untouched; `taproot` gains a key token in its own grammar | untouched; `taproot`'s `SemanticLookup` gains optional key and address fields of its own, filled by the host | cannot emit it; translated by the host, or its vocabulary becomes the stack's |
| Existing code changed | uxtree, taproot, mandrake, Turnstone: estimate under 1,000 lines | the above plus four side tables: estimate 1,000 to 1,500 lines | estimate 6,000 to 9,000 lines in the path (§3.4) |
| Deleted | mandrake's parser; `stitch` if unused | the above; Turnstone's bridge generation map once uxtree owns Derived generations; four side-table types become one | the above; `PublicationBindings`, `A11yRoutes`, `action_map` as separate types |
| Migration | additive at each step | additive at each step: an envelope can be filled for one producer while the others still use their own table | each producer flips at once at its output, since the OS needs one tree per frame; a producer half-migrated needs both lowerings |
| Hard to undo | a path or key grammar in 172 `.scn` files | the stability classes, once scenarios rely on Authored keys; the envelope's fields, once written to receipts | the node type and its role vocabulary, once four producers build it |

**Is the federated form distinguishable from option 1?** Yes, by one commitment: a shared envelope type that every
producer fills and that `ux-events`, the lint, the scenario runner and `taproot`'s host read. Without it, option 1
leaves each consumer to learn four side tables, which is what the stack does now. With it, no new node type exists.
If the envelope were cut to the address alone, the two would be the same.

**Does option 2 buy anything the federated form does not?** Two things. One role and state vocabulary that every
consumer reads without AccessKit; and a tree that could be serialised whole. Against the first: automation reads
roles as tokens already (`taproot` matches "owner-computed" role strings), and the OS reads AccessKit's. Against the
second: the remote path already carries chirograph semantics, and the 2026-07-22 plan refused uxtree as a wire
contract. *Reading, not ruled:* the browser analogy (an accessibility tree as a projection of one DOM) favours option
2 only where one DOM exists. Here, four models exist above AccessKit, one of which (Genet's) is a DOM projection
already and sits below uxtree. Option 2's cleanest variant would make Genet's `DocumentA11yProjection` the stack's
node type, which is a different proposal from "uxtree owns it".

## 5. Prior rulings each form would touch

1. **Remote projection plan (2026-07-22), §4.3:** "The existing Mere `uxtree` is not adopted as the wire contract: it
   depends on Inker and projects browser documents. A neutral semantic payload must be proven by two applications
   before extraction into a shared crate." Promotion table: "Extract a renderer-neutral semantic core into Meristem
   only after Graphshell and another app prove the same structure." *Option 1, federated:* untouched if the envelope
   stays in-process. *Option 2:* contradicts it if the node type crosses a wire; amends it ("extract into uxtree, not
   Meristem") if not. Note: chirograph's semantics are read by two applications today (the reference app's
   `dramatis-pane` and `graphshell-host`'s canary through `AccessibilityTree`; Turnstone through `FrozenScene`). Whether that meets the
   two-application bar is Mark's reading.
2. **Diagnostics plan, Constraints:** the stale-target sentence (§3.3) and "AccessKit owns typed node-reference
   traversal; Mere keeps no second list of relation properties". *Federated:* applies, if Derived ids carry an owner
   generation. *Option 2:* applies only if the node type keeps no relations, which leaves AccessKit as a second
   structure anyway.
3. **Diagnostics plan, M5:** "contributions live in one supplier-qualified model that preserves geometry, clipping,
   disabled and read-only behaviour". This is a one-model goal for Cambium's contributors, not the stack. *Option 2*
   would widen it; *federated* leaves it inside Cambium.
4. **Diagnostics plan, M4:** guest "grafts and trees (the publisher is ROOT-only and refuses them)". An address of
   `(TreeId, NodeId)` needs this open item done for Cambium; Turnstone's `foreign_a11y` is an existing
   implementation of the missing half (fork 3).
5. **App composition AC2** (2026-10-07): retained sessions join as AccessKit subtrees through `uxtree::graft`. Pelt
   and the Cambium publisher do not graft today. Applies to all three forms; fork 3 asks whether the address depends
   on it.
6. **D53's wording:** "graft joins the Cambium and taproot subtrees into one ID space". `taproot` builds no tree and
   graft keeps id spaces apart. All three forms need it restated, as the first brief's fork 2 proposed ("one address
   space").
7. **D51:** "mere-overmap and mere-view stay separate". None of the forms moves it; `gloss` is the only uxtree user in
   the view.
8. **D23, D39:** the MCP server goes to Turnstone; `ux-events` is kept. Neither is touched; the MCP needs no
   envelope (E5).

## 6. Forks for Mark

Each fork has its evidence, then options stated with what each commits the stack to, then a recommendation marked as
one. The recommendation is a reading and is kept apart from the evidence.

### 6.1 Fork A. Which form does the spine take?

*Evidence.* Four models above AccessKit exist (E1, E2). Four side tables carry the envelope's fields in four types
(§2, E1). Genet cannot name uxtree (E3). Option 2's path holds an estimated 6,000 to 9,000 existing lines; the
federated form's, an estimated 1,000 to 1,500 (§3.4, §4).
- (a) **Federated:** uxtree owns the address, the envelope and the composition; producers keep their models and fill
  the envelope beside their AccessKit output. Commits to one envelope type in the host workspace.
- (b) **Option 1:** address and lint only. Commits to nothing new per node; consumers keep reading producer tables.
- (c) **Option 2:** uxtree owns a node type producers build. Commits to a stack vocabulary and a migration of six
  producers, and to a Genet answer (fork B).
- (d) **Option 2 via Genet:** Genet's `DocumentA11yProjection` becomes the stack's node type, extended with the
  envelope. Commits Genet's contract to host needs.

*Recommendation:* (a). The envelope's fields are already kept four times, so consolidating them removes parallel
runs rather than adding one, and the change stays in side tables. Option 2 can follow later if a consumer needs a
field AccessKit cannot carry; nothing in (a) closes it.

### 6.2 Fork B. Where do the address and envelope types live, given Genet?

*Evidence.* uxtree → inker → `document-session-api`; Genet keeps no upward edge (E3). `taproot` reads through
`SemanticLookup`, a Genet trait with `SemanticTarget::{Dom, Contributed(u64)}` (`taproot/lookup.rs:9-27`). At the
cutover the other producers join uxtree in Graphshell's host workspace.
- (a) **In uxtree; Genet stays out.** `taproot`'s `SemanticNode` gains optional fields of Genet's own (a key, an
  opaque address), filled by the host's `SemanticLookup` impl. Commits to no new crate and keeps D53's "stays whole".
- (b) **A new leaf crate** depending on `accesskit` and `serde` only, named by uxtree and `taproot`. Commits to a
  new published contract below the engine.
- (c) **In `document-session-api`.** Commits Genet's contract to host provenance and action ids.
- (d) **In uxtree with `inker` behind a feature.** Does not reach Genet: the edge would still point from Genet up to a
  Graphshell host crate.

*Recommendation:* (a). `taproot` only reads, and it already reads through a trait it owns; two optional fields on
`SemanticNode` cost Genet least, and they carry nothing host-specific.

### 6.3 Fork C. What is the address?

*Evidence.* Only Turnstone grafts. Cambium's publisher refuses grafts (M4 open); Pelt mints one tree (E4). AC2 rules
graft as the join for retained sessions.
- (a) **`(producer instance, local id)`**, lowered to `(TreeId, NodeId)` where the producer is grafted and to the
  minted id where it is not. Commits uxtree to a producer registry per window.
- (b) **`(TreeId, NodeId)` only.** Commits Pelt and the Cambium publisher to grafting first (M4).
- (c) **Both**, with (b) as the target and (a) during migration.

*Recommendation:* (c). The address should not wait on M4, and AC2 already names graft as the end state; (a) is the
step that leaves every producer working meanwhile.

### 6.4 Fork D. Stability classes

*Evidence.* §3.3. Three producers issue lives; two scope by revision; one derives and keeps a generation outside
the projection. The stale-target rule forbids a frame counter as a life.
- (a) **Four declared classes** (Life, Scoped, Derived, Authored); Derived ids take an owner generation in uxtree;
  actions are always rechecked by the owner. Commits uxtree to a generation map like Turnstone's bridge.
- (b) **Life and Scoped only.** Derived producers must mint (Turnstone and uxtree's projections switch to minted ids
  plus a path index). Commits to retiring path hashes as published ids.
- (c) **No class field.** Each consumer keeps per-producer knowledge. Commits to today's arrangement.

*Recommendation:* (a). It names what exists, and the one new piece (an owner generation for Derived ids) is a move
of 80-odd lines from Turnstone's bridge, not new design.

### 6.5 Fork E. What automation addresses

*Evidence.* 576 `act` by label, 54 `click-at` by pixel, 37 `assert a11y` by text, 172 `.scn` files; class 109,
role 67, `data-` 71 of 176 clicks (first brief F8). Action ids exist only where a producer advertises commands
(§3.2).
- (a) **Authored keys plus role and name, and `act` by action id where one exists.** Commits producers to carry keys
  and the runner to an id form of `act`.
- (b) **Path tokens** over Derived ids. Commits to a path vocabulary as a contract; covers Turnstone and uxtree only.
- (c) **Keys only.** Commits nothing about commands.

*Recommendation:* (a). Keys reach every producer with at most a small change (§3.2); paths reach two.

### 6.6 Fork F. Does the envelope cross a boundary?

*Evidence.* Guests see no tree (`app-host` emits named actions; `block-host` returns a view) (E6). The remote path
carries chirograph semantics. Program-written text is JSON; postcard inside the stack behind a version header; CBOR
across a trust boundary.
- (a) **No.** In-process only; receipts print it as text lines as `forest_lines` does now.
- (b) **As a JSON snapshot** for receipts and an MCP read (the first brief's fork 9 option (b)). Commits to a
  versioned schema.
- (c) **As a wire contract.** Commits to reopening the 2026-07-22 ruling.

*Recommendation:* (a) now; (b) when a consumer asks. No consumer reads it across a boundary today (E5, E6).

### 6.7 The first brief's forks, mapped

| First brief | Here |
|---|---|
| 1 (shape) | A |
| 2 (one id space for graft) | C, and §5.6 |
| 3 (AccessKit model or richer) | A |
| 4 (scenario address) | E |
| 5 (id promise) | D |
| 6 (lint checks, place, emission) | unchanged; the envelope gives it provenance and a class to judge |
| 7 (diagnostics identity, parked chain) | unchanged; `UxEvent` would take the fork C address |
| 8 (promotion audit) | partly answered (§2.2): `foreign_a11y` and the bridge's generation map are generic; the `Admission` variants are not |
| 9 (MCP and the UI tree) | F; the MCP needs no envelope |

**Suggested order (not a decision):** A and B together; then C and D; then E and F with the first brief's 6 to 8.

## 7. What could not be determined, and the prototype

**Not determined.**
- Whether one envelope can carry a Life producer and a Derived producer side by side without a consumer having to
  know which is which. Read, not run.
- The edit counts in §3.4 and §4. Estimates from the sizes of the code in the path.
- Whether a `data-key` attribute is the right authored-key carrier for chrome built without a DOM (Turnstone's).
- Whether `hest` action ids can name Turnstone's `Action` variants: no mapping exists anywhere to measure.
- Whether `cargo metadata` shows edges the lockfile scan missed: the run waited on another session's package-cache
  lock and was stopped (E3).
- Whether inker surfaces (the foreign producer behind `foreign_a11y`) could fill an envelope: they publish AccessKit
  only, and this pass did not read inker's surface side.

**The smallest prototype that settles fork A and fork D.** Extend the E1a bench, which already grafts two retained
Cambium sessions through `uxtree::graft` and reads them back through `accesskit_consumer`
(`cambium-a11y/tests/e1a_sessions.rs`, 615 lines). Two producers: a Cambium session with a texture producer that
advertises a command (Life ids, an action id, a domain reference, as the canvas does), and a Genet-backed session
(Scoped ids by revision). The host tree built with uxtree paths gives the Derived class for free. One consumer: a
lookup that resolves an envelope by address and invokes by action id. It would have to show: each node's envelope
filled from what its producer already knows; a request on a retired guest refused at its owner after a `NodeId` is
reused (the stale-target control); a planted envelope-less node reported by the lint (the positive control). If that
holds, fork A (a) is evidenced; if the envelope needs producer-specific branches in the consumer, option 1 is the
honest answer.
