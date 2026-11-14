# Primitive Representations Plan

**Status:** 2026-11-14: plan. Cut from `2026-06-23_node_body_face_model_plan.md`, archived
whole in `archive_docs/2026-11-14/` with B0 to B2 landed. Nothing here is started. Phases M1
to M6 wait on fork F4 of the scene composition brief (F1 to F3 were answered 2026-11-14,
D5 to D12); B3 and B4 are carried open.

**For:** how the three graph primitives (node, relation, field) are drawn becomes a set of
independent choices on a scene recipe, so a person can assemble a scene in Graphshell's
projection editor instead of picking a finished family. The frame and the reasons are in the
[scene composition brief](../research/2026-11-14_scene_composition_brief.md); this plan
carries its cost items 1 to 6 and the open work of the body and face plan, where a node's
body and face are now two properties of one mark kind, the physical tile.
Not here: the missing projections themselves (outline, calendar, chart, document, record,
table's wider encoding), which wait on the brief's forks F5 to F8 and get their own plan;
props, backdrops, linked swatches and primitive editor writes (scenograph editor plan); the
recipe's dynamics slot and field regions (dynamics grammar plan); derived marks, scales and
guides (projection grammar plan, A5); theme and typography (appearance plan).

Receipts from this plan's runs go to `archive_docs/receipts/`, never into this plan
(DOC_POLICY §8).

## Constraints

- A scene never carries rendered content; a representation is a slot the host fills
  (scene contract note, decision 6).
- Per-node overrides key by member Uuid and are pruned when the member leaves
  (`node-body-face-model D1`, in the archived plan).
- Options and capabilities are declared as data, and an undeclared key is refused at compile
  before solving (scenograph editor plan, SE7 and SE8).
- A pin a solver cannot honour is reported unmet, never dropped. Placed relation forms take
  the same report.
- Mosaic keeps its steady behaviour on a pannable infinite canvas; Partition is the
  viewport-bounded subdivision (Mark, 2026-10-16; kept by D5). They stay two arrangements,
  and the frame is part of what each is.
- Each phase is verified in its own worktree before it touches main: its tests pass and the
  existing scene pins hold, or move under a ruling.

## Phases

### M1. One mark vocabulary
A node's look is described once: a mark has a detail rung (`sceno::Representation`), a paint
(today's `Face`: Favicon, Derived, Sprite, Bare), an optional body (collider and material),
and an extent rule: measured, assigned or encoded.
- Done when: the representation registry and its docs name detail, paint, body and extent as
  parts of a mark; every existing family states which extent rule it uses; no document in
  `design_docs/` presents `Representation` and `Face` as alternatives.
- Also done when (D11): no mark is the default for every node; the registry picks a mark
  from what the node's data discloses, and a node whose data is not a document is not drawn
  as a card by default.
- Also done when (D12): a card can be chosen as a node's own mark, or as a companion to a
  node drawn another way, placed beside it or in an overlay; the placement is a choice on the
  recipe, and a document graph with card nodes is expressible.

### M2. Mark slots on the recipe
The recipe names a node mark, a relation form and a field form.
- Done when: a recipe that names a node mark compiles to a scene whose items ask for it; a
  presentation rule can still override it per occurrence, with the trace showing which won; a
  recipe written before the slots existed compiles to the same scene as before.
- Needs: M1; fork F4 (default on the recipe, or rules only).

### M3. Relation form is read
`RelationForm` has no reader outside `presentation.rs` today.
- Done when: a drawn form (stroke, matrix cell) changes what the renderer draws for the same
  relations; a placed form (containment, order) reaches the arrangement as a constraint; each
  placed relation is reported honoured or unmet in the scene; a form an arrangement did not
  declare is refused at compile.
- Needs: M2; fork F3 for adjacency, with a probe of relation-driven adjacency at size before
  the fork is put.

### M4. Frame as a slot
Bounded, unbounded, nested and flow frames are chosen on the recipe and passed to the
arrangement.
- Done when: the recipe names the frame and `Partition`, `Setscape` and `Mosaic` take their
  rectangle or origin from it; each arrangement declares the frames it accepts (Partition
  bounded only, Mosaic unbounded only, D5) and a mismatch is refused at compile; existing
  scenes of those three families produce the same placements as before.
- Needs: M2.

### M5. Mosaic and Partition take any assigned mark
- Done when: each accepts any node mark with an assigned extent, a card included, with its
  current placements unchanged for the tile mark.
- Needs: M2, M4.

### Z. Partition zoom
Zooming a Partition changes how much of the viewport each cell gets along its containment
relations; it does not scale the picture (D5).
- Done when: zooming in on a group gives it and its members a larger share of the same
  bounds and the rest a smaller one; zooming out restores the shares; the remainder region
  stays honest at every level; a headed check shows it on Tiles and on Rings.
- Needs: the viewer's zoom input routed to the family as a reading parameter.

### N. Crystal
Tiles on the infinite canvas that touch when linked, started by pinning (D6, D7; brief
section 3.7).
- Done when: with nothing pinned, tiles stay where dropped and no link is drawn until a tile
  is selected; pinning a tile packs its linked tiles against it, resized to fit; a tile
  against a nucleus can be slid along its border and the rest repack; every link is reported
  as a border or as unmet; unpinning leaves the tiles where they are.
- Needs: a probe first, on a real graph, of how many links become borders and what a crowded
  nucleus does; M3 for the report; the physics profile `crystal` renamed first (D10), which
  reaches Turnstone's `src/arrange_pane/tests.rs` (five uses). Packing goes one step (D8) and overflow
  stays loose (D9), so the done-condition adds: pinning a tile moves only its own linked
  tiles, and linked tiles that do not fit on its border stay where they are.

### M6. Compatibility and the swatch axes
- Done when: every arrangement declares the extents it assigns, the placed forms it honours
  and the frames it accepts; the swatch grid has axes for node mark, relation form, field form
  and frame beside Scope, Arrangement and Dynamics; a combination that cannot work is not
  offered and the editor says why; a pick along a new axis is one undo step; a headed check
  shows cards under a subdivision and a subdivision in a viewport frame.
- Needs: M2 to M5. The second half of the headed check (a subdivision in a viewport frame)
  is Partition with a card mark.

### X. Context scenes
What appears for a subject on hover, focus, selection or pin is a scene with its own recipe
(D13; brief section 3.9). Nothing about its content is fixed: linked nodes, a resource, or
commands.
- Done when: a recipe declares contexts, each with a trigger, a subject kind, a recipe and a
  placement; the same subject shows three different contexts under three recipes (its linked
  nodes, its resource, its commands) with no code specific to any of them; a context's
  placement can be beside the subject, in an overlay, in place of it or in a pane; a context
  is picked in the editor like any other slot and is one undo step; the existing summoned
  card and context menu are expressible as contexts.
- Also done when (D14): the commands available on a subject are a dataset a recipe reads; a
  person's edits to what a context shows (which commands, order, labels, shortcuts) are data
  that round-trips through save and reopen; an action emitted by a Wasm guest through WIT
  appears and runs from a context like a host command.
- Needs: M2; fork F4, since a context is a second mark choice on a subject; the trigger
  vocabulary put as a question before code; the format proposal in brief section 3.9
  confirmed; a check of whether a guest can advertise commands, which the WIT read on
  2026-11-14 did not show.

### B3. The body designer
Carried from the archived plan. A hull can be set only through `set_node_sprite_hull` today,
so an authored body still enters by the sprite path.
- Done when: a person authors a node's hull from scratch, picks a primitive, and sets its
  material in an editor, with collider and physics updating live. The stored hull is the
  edited polygon while the collider is reconvexified, so a concave edit must either be
  constrained to convex or lowered as a compound collider.
- Needs: an owner for the editor surface (open question below).

### B4. Form parity
Carried from the archived plan, unverified against the current tree: its citations name
crates that no longer exist. Re-read the code at phase entry.
- Done when: sprites and custom bodies render on secondary panes as on the focused one; a
  live surface is selectable as a node's top detail rung; a decorative scripted face is wired;
  each blocked case names its owner.
- Needs: re-verification; M1.

### Tails. Follow-ons of the archived plan
One styling lens over node shape, edge style and field style; label density (off, terse,
full) per scene; sprite and hull blobs out of the sidecar into a store keyed by hash; whether
sprite, hull and material travel with a node across federation sync; scene-wide defaults by
content type; which per-node facets the deep editor exposes; routed-path edges trimmed to the
face.
- Done when: each is done, moved to a named owner, or ruled out.
- Needs: M2 makes scene-wide defaults and the styling lens recipe material; the rest wait on
  the open question below.

## Open questions

- Forks F2 and F4 of the scene composition brief gate M1 and M2: F2 is answered (D11, D12), F4 is open.
- Which triggers does a context take? Hover, focus, selection and pin are named in D13 and
  section 3.9; the full set and its touch and keyboard equivalents are not ruled.
- Does the format proposal for commands (TOML authored, JSON interchange, WIT across
  components) stand? Mark asked "You tell me"; it is proposed, not ruled.
- Which plan owns the `node:<id>` facet pane, the per-object widget surface and label
  density's settings page? Both former owners are archived (raised 2026-10-06, still open).
  Options: this plan; a Turnstone plan; a canvas plan.
- Does the body designer (B3) belong in the projection editor beside the swatch axes, or in
  the node's own facet pane?

## Decisions

- **D1. The brief's scope (Mark, 2026-11-13):** "Slots plus ten projections". One brief
  covers the slot model and the ten projections; no implementation plan for the projections
  is cut from it yet.

  Links: summary The scene composition brief covers the slot model and the ten projections together.

  *New.*

- **D2. Body and face plan (Mark, 2026-11-13):** "Archive and re-cut now". The plan is
  archived whole and its open work is carried here, with body and face as two properties of
  the physical tile mark.

  Links: summary The body and face plan is archived; its open work continues in the primitive representations plan.

  *New.*

- **D3. The promotion gate does not hold this scope back (Mark, 2026-11-13):** "Everything
  in the stack runs on linked data, yes? So we have to build stack capabilities to adopt
  them. All are potential and intended consumers, which is plenty reason to implement a plan.
  That rule is poorly motivated for this scope". Reading, mine: for the scene slots and the
  ten projections, the family's applications are the consumers, and neither a forcing proof
  nor a second heterogeneous consumer is required before work is planned or built. The gate
  is not ruled on outside this scope.

  Links: summary The forcing-proof and second-consumer gate does not apply to the scene slots or the ten projections; the stack's applications are the consumers.

  *Refines the projection grammar plan's constraints on promotion and on a speculative spec stack.*

- **D4. The first scene designer (Mark, 2026-11-13):** "Person in the editor". A person picks
  each slot in Graphshell's projection editor; the saved recipe is data an application can
  also ship.

  Links: summary Scene design is built first for a person in the projection editor; the shipped-recipe path follows from the saved format.

  *New.*

- **D5. Mosaic and Partition stay distinct (Mark, 2026-11-14):** asked which arrangement
  carries the name Mosaic: "3, crystalline, or solid? Idk. A mosaic is an actual ui
  presentation, right? If that applies to mosaic as it exists, it should keep that name. But
  we should differentiate between mosaic and partition; a partition is not on an infinite
  canvas, but it's a subdivision of the viewport. So zooming a partition expands or shrinks
  cells relative to their containment relations." Reading, mine: a mosaic is an established
  presentation (tiles in a tiling) and the existing behaviour fits it, so the name stays; the
  edge-adjacent layout is a third arrangement, to be named.

  Links: summary Mosaic keeps its name and steady behaviour; Partition is the viewport subdivision whose zoom reallocates cells along containment; the edge-adjacent layout is a separate arrangement.

  *Applies the 2026-10-16 Mosaic and Partition ruling; new for Partition zoom.*

- **D6. Pinning nucleates adjacency; other links show on selection (Mark, 2026-11-14):**
  asked what happens to a link that cannot be a shared border: "2, but pinning a node is what
  reflows the layout to collapse the links into adjacency. It's like pinning creates a
  nucleation point for the restructuring of the layout." Option 2 was: the link is reported
  unmet and stays invisible until one of its tiles is selected or hovered.

  Links: summary In Crystal a pinned tile is the point its linked tiles pack against; links that are not borders are drawn on selection only.

  *New.*

- **D7. Dragging in Crystal (Mark, 2026-11-14):** "If no pin, dropped tile
  stays. If pin, reflow, unless you adjust it around the adjacent node (for which any
  position adjacent is valid, but it will pack nodes into adjacency essentially)".

  Links: summary Unpinned tiles stay where dropped; a tile against a nucleus may take any position on its border and the rest repack.

  *New.*

- **D8. Packing goes one step (Mark, 2026-11-14):** asked how far packing spreads from a
  pinned tile: "One step, pin to grow". Only the pinned tile's own linked tiles pack; to grow
  further, pin one of them.

  Links: summary In Crystal a pin packs only its own linked tiles; the structure grows by pinning more.

  *New.*

- **D9. A crowded nucleus leaves the overflow loose (Mark, 2026-11-14):** asked what happens
  to linked tiles that cannot fit on a pinned tile's border: "I pictured 3, but building a
  crystal by pinning similar nodes together (e.g. pin a node, pin some adjacent nodes, then
  some of their adjacent nodes, etc.) is nifty, potentially. 1". Option 1 was: the tiles that
  fit pack in and the rest stay where they are, their links shown on selection. Option 3 was:
  the pinned tile enlarges until its border holds every linked tile.

  Links: summary Linked tiles that do not fit on a nucleus's border stay loose; the nucleus does not grow to hold them.

  *New.*

- **D10. The arrangement is Crystal (Mark, 2026-11-14):** offered Accretion; Crystal with
  the physics profile renamed; or Crystal for both: "2". The arrangement of D6 to D9 is named
  Crystal and the physics profile with id `crystal` takes another name.

  Links: summary The pin-nucleated adjacency arrangement is named Crystal; the physics profile that held the name is renamed.

  *New.*

- **D11. Why "a card is not a node" (Mark, 2026-11-14):** "for the a card is not a node
  idea, it was to correct a particular tendency to turn all nodes into cards in a document
  graph, when not all nodes should be that, because not all data is documents." Reading,
  mine: the rule is that no mark is the look of every node and a card is for document data;
  Card remains a detail rung a node may be drawn at.

  Links: summary A card is not the default look of a node: marks follow what the data is, and not all data is documents.

  *Refines the body and face plan's "a card is not a node".*

- **D12. Nodes can be cards, but not every node is a card (Mark, 2026-11-14):** "I
  distinguished between the node and the card. The card being something that exists either
  in the overlay or as a contextual thing, like a contextual card to the side of the node, or
  context menu could be considered a type of card that is, you know, configurable and
  changeable in size, or snapshot cards. These things typically existed next to the node. I
  don't think it has to exist next to the node. I think it's maybe a cleaner presentation,
  but not every presentation requires that. And I do want to be able to represent document
  graphs using the system. So, you know, nodes can be cards, but not every node is a card."

  Links: summary A card is a form that may be a node's own mark or a companion beside or over a node; which is a choice of the presentation, and document graphs draw nodes as cards.

  *Refines D11.*

- **D13. Context is a scene, and is not prefigured (Mark, 2026-11-14):** "the question of
  what shows up contextually when a node is hovered or brought into focus is also a question
  of a scene, I would imagine, right? In some cases, you would want, you know, the thing
  shown to be other nodes, right? Other entities, data that is like pure data. But in other
  cases, you want resources to be shown or controls and commands to be shown, right? So that
  contextual question should not be prefigured. We should build to allow for the complexity
  of that."

  Links: summary What shows for a subject on hover or focus is a scene with its own recipe; its content (nodes, resources, commands) is a choice, never built in.

  *Refines D12: the companion is a context scene, of which a card is one recipe.*

- **D14. Commands are a dataset (Mark, 2026-11-14):** "I'm down for commands as a data set.
  Absolutely. That matches the stack's inclination, right? And it also allows for
  modification ... I hope it works with Wasm. And I hope it works with Wit. And I hope it
  works with WASI ... define it as data. I mean, we could even use structured data. I don't
  know, TOML. We usually use TOML for other stuff. I don't know. You tell me."

  Links: summary The commands available on a subject are a dataset a recipe reads; a menu is a list projection, and a person's changes to it are data.

  *New.*

- **D15. The physics profile is Lattice for now (Mark, 2026-11-14):** "you want to call
  this one Lattice? I think that's fine if you want to, at least a stand-in name for right
  now." The profile with id `crystal` (the Stress law with the grid-snap overlay) becomes
  Lattice as a stand-in. Mark also said of the physics profiles that "a couple of those are
  ... kind of bunk, the kind of rough, or in need of tweaking"; that review is the physics
  catalog plan's and is not taken up here.

  Links: summary The physics profile that held the name Crystal is renamed Lattice, as a stand-in.

  *Refines D10.*

## Findings

- 2026-11-14 (at `b957d819`): `RelationForm` is defined at
  `host/crates/cambium/scenes/scenograph/src/presentation.rs:188` and no other file in the
  repository names it.
- 2026-11-14: `Mosaic`, `Partition` and `Setscape` are `Arrangement` variants
  (`crates/sceno/src/score/arrangements.rs:21`) whose structs carry their own frame:
  `Partition.bounds`, `Setscape.bounds`, `Mosaic.origin` (`crates/sceno/src/subdivision.rs`).
- 2026-11-14: the recipe (`ProjectionDefinition`, `scenograph/src/lib.rs:354`) has no field
  for a node's look; `Encoding` carries x, y, color and label only.
- 2026-11-14: the body and face setters live in cartography: `set_node_face`,
  `set_node_sprite`, `set_node_sprite_hull` (`canvas/cartography.rs:261,290,321`) and
  `set_node_material`, `clear_node_material` (`canvas/cartography/appearance.rs:72,81`). There
  is no setter for a hull that is not a sprite hull. `FACE_INSET` is 0.72 at `canvas.rs:371`.
- 2026-11-14: the swatch grid's axes are `Scope`, `Arrangement` and `Dynamics`
  (`scenograph/src/swatch.rs:75`).

- 2026-11-14: `scenomise/src/families/partition.rs` has the Tiles and Rings forms and no zoom
  or drill-in input.
- 2026-11-14: `crystal` is a physics profile id
  (`cartography/src/canvas/physics_catalog.rs:187`) and "crystallize" names freezing a
  selection into a subgraph (`mere-view/src/subgraph.rs:228`).

- 2026-11-14: the id `crystal` is used outside this repository in Turnstone
  (`src/arrange_pane/tests.rs`, five times); Mere, Knot, Woodshed and Isocosm do not name
  the profile (tracked files, `git grep`; Isocosm's one hit is an unrelated item key).

- 2026-11-14: commands are data in three places: `hest::menu::Command` and `CommandChoices`
  (`crates/hest/src/menu/mod.rs:26,83`), `chirograph::AdvertisedAction`
  (`crates/chirograph/src/lib.rs:141`), and the WIT `actions` interface's `action-envelope`
  (`crates/graphshell-endpoint/wit/world.wit:146`).
- 2026-11-14: in this repository "lattice" is an ordinary word in one test and one comment,
  not a name. Other repositories and registries were not checked.

## Progress

- 2026-11-14: cut from `2026-06-23_node_body_face_model_plan.md` with the scene composition
  brief; nothing landed.
- 2026-11-14: forks F1 and F3 answered (D5 to D7). M5 rewritten, phases Z and N added; nothing
  landed.
- 2026-11-14: forks F10 and F11 answered (D8, D9); phase N's done-condition extended.
- 2026-11-14: the arrangement named Crystal (D10); F2 answered (D11, D12).
- 2026-11-14: context scenes ruled (D13); phase X added.
- 2026-11-14: commands as a dataset (D14); the physics profile is Lattice for now (D15).
