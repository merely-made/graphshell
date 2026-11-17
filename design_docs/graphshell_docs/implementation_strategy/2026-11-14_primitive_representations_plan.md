# Primitive Representations Plan

**Status:** 2026-11-14: plan. Cut from `2026-06-23_node_body_face_model_plan.md`, archived
whole in `archive_docs/2026-11-14/` with B0 to B2 landed. Phases M1
to M6 no longer wait on a fork of the scene composition brief (F1 to F3 were answered 2026-11-14,
D5 to D16); B3 and B4 are carried open.

**For:** how the three graph primitives (node, relation, field) are drawn becomes a set of
independent choices on a scene recipe, so a person can assemble a scene in Graphshell's
projection editor instead of picking a finished family. The frame and the reasons are in the
[scene composition brief](../research/2026-11-14_scene_composition_brief.md); this plan
carries its cost items 1 to 6 and the open work of the body and face plan, where a node's
body and face are now two properties of one mark kind, the physical tile.
Not here: the missing projections themselves (outline, calendar, chart, document, record,
table's wider encoding), which get their own plan, table first (D18), with the document
left to Knot (D19), fields as an ordered list (D20) and feed as a reading (D21);
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
- Also done when (D12, D31): a card can be chosen as a node's own mark, and a document graph
  with card nodes is expressible. A card as a companion to a node drawn another way is a
  context scene, placed beside or over the node in phase X.
- Also done when (D28 to D30, D32 to D35): the mark type is in scenograph and holds detail,
  a paint (Favicon, Derived, Sprite, Bare and an open tail), a body (none, a silhouette or
  an authored hull, with material) and the extents it accepts; each arrangement declares the
  extent it imposes and a mismatch is refused at compile with both named; an occurrence
  discloses its classes and the registry resolves the mark from them; the Sprite rung is
  retired to the open tail; `set_node_rule_faces` is renamed in the same change, with
  callers across the family checked.

### M2. Mark slots on the recipe
The recipe names a node mark, a relation form and a field form.
- Done when: a recipe that names a node mark compiles to a scene whose items ask for it; a
  presentation rule can still override it per occurrence, with the trace showing which won; a
  recipe written before the slots existed compiles to the same scene as before.
- Needs: M1. The recipe names the default and rules override it (D16).

### M3. Relation form is read
`RelationForm` has no reader outside `presentation.rs` today.
- Done when: a drawn form (stroke, matrix cell) changes what the renderer draws for the same
  relations; a placed form (containment, order) reaches the arrangement as a constraint; each
  placed relation is reported honoured or unmet in the scene; a form an arrangement did not
  declare is refused at compile.
- Needs: M2. Adjacency driven by relations is Crystal's (D6); Mosaic keeps group and order.

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
- Needs: M3 for the report. The smallest tile side is a recipe setting, 32 px by default
  (D24), and a border counts at half that side or longer (D25), so the done-condition adds:
  a recipe can set the smallest side and the packing honours it. Unlinked tiles
  never touch (D23), so the done-condition adds: no two tiles share a border unless they are
  linked. The physics profile `crystal` is renamed `lattice` in this repository (D26);
  Turnstone's `src/arrange_pane/tests.rs` (five uses) follows through its integration lane. Packing goes one step (D8) and overflow
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
What appears for a subject on hover, focus, selection or a context request is a scene with its own recipe
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
- Also done when (D16, D17): a context is declared on the recipe and a rule can override it;
  each of hover, focus, selection and context request (right-click, long-press, menu key)
  can trigger one; a recipe whose only trigger is hover is flagged in the editor, since hover
  has no touch equivalent.
- Needs: M2; one command dataset drawn from the four shapes that exist (Findings), guest
  commands included.

### B3. The body designer
Carried from the archived plan. A hull can be set only through `set_node_sprite_hull` today,
so an authored body still enters by the sprite path.
- Done when (D43): a person picks a silhouette and adjusts it by a few numbers (sides,
  corner radius, aspect), can trace an image to suggest the nearest silhouette or a
  polygon, and sets the material, with collider and physics updating live; dragging
  vertices is an escape hatch, not the way in; picking and edge attachment follow the
  outline. The stored hull is the
  edited polygon while the collider is reconvexified, so a concave edit must either be
  constrained to convex or lowered as a compound collider.
- Needs: the surface is the scenograph editor's (D27); the scenograph editor plan owns it
  and this phase supplies the hull, primitive and material it edits.

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

- Forks F2 and F4 of the scene composition brief gate M1 and M2 and are answered (D11, D12, D16).
- The fields list takes SHACL's terms with one added (`table-projection D2`); the
  [table projection plan](2026-11-14_table_projection_plan.md) carries it.
- Which plan owns the `node:<id>` facet pane, the per-object widget surface and label
  density's settings page? Both former owners are archived (raised 2026-10-06, still open).
  Options: this plan; a Turnstone plan; a canvas plan.

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

- **D16. Mark choices live on the recipe, and rules override (Mark, 2026-11-14):** "Recipe
  default, rules override". The recipe names a default node mark, relation form, field form
  and contexts; presentation rules change them for particular occurrences or conditions.

  Links: summary A recipe names default marks and contexts; presentation rules override them per occurrence.

  *New.*

- **D17. Context triggers (Mark, 2026-11-14):** all four offered: "Hover, Focus, Selection,
  Context request". A context request is a right-click, a long-press or the menu key.

  Links: summary A context scene may be triggered by hover, focus, selection or a context request.

  *New.*

- **D18. Table is the first missing projection (Mark, 2026-11-14):** "Table", over outline,
  calendar and chart.

  Links: summary Of the projections with nothing in code, table is planned first.

  *New.*

- **D19. The document projection is Knot's (Mark, 2026-11-14):** "Knot's, embedding
  scenes". Narrative text in reading order is Knot's surface, with scenes inside it as
  blocks; the scene stack gains no flow layout. Document graphs, nodes drawn as cards, stay
  a scene recipe (D12).

  Links: summary Narrative documents are Knot's surface with scenes embedded; the scene stack has no flow layout of its own.

  *New.*

- **D20. Fields are an ordered list, with the standards considered (Mark, 2026-11-14):**
  asked how a recipe carries many fields per item: "Hmm. If the standards have anything to
  say about this, might be useful to consider. 1 otherwise." Option 1 was a separate slot on
  the recipe: an ordered list of fields, each with a label and how it is shown. The
  standards (SHACL property shapes, CSV on the Web table schemas, Fresnel lenses) point the
  same way; brief section 3.10.

  Links: summary A recipe carries an ordered list of fields, separate from the encoding, shaped with the linked-data display standards in view.

  *New.*

- **D21. A feed is a reading (Mark, 2026-11-14):** "A feed... should be able to be
  represented as most of these. I don't think these two options are the only way to
  represent a feed, but 1 is valid. A feed is an updating log of n length where the entities
  are ordered and a bounded range of them are shown." Reading, mine: feed is not a
  projection; it is an ordered, updating reading with a bounded window, which table,
  timeline, graph and the others can each show. A one-column table of cards is one preset.

  Links: summary A feed is an ordered, updating reading with a bounded window, shown by any projection; it is not a projection of its own.

  *New.*

- **D22. Command format (Mark, 2026-11-14):** "TOML authored, JSON carried, WIT across".
  What a person or app author writes is TOML; the live list a recipe reads is typed in
  memory and JSON where exchanged; across a Wasm component boundary it is a WIT record.

  Links: summary Commands as data are TOML where authored, JSON where exchanged and WIT records across components.

  *Applies the workspace data-format rule.*

- **D23. In Crystal, unlinked tiles never touch (Mark, 2026-11-14):** "Never touch". A
  shared border always means a link. The second probe kept unlinked tiles 8 px apart and a
  full-size tile then held 9 linked tiles against a ceiling of 10.

  Links: summary In Crystal a shared border always means a link; tiles that are not linked keep apart.

  *New.*

- **D24. Crystal's smallest tile side is a recipe setting, 32 px by default (Mark,
  2026-11-14):** "4, default 32px", to the option "A recipe setting, default 48". The
  scene's author chooses how far a linked tile may shrink; a recipe that says nothing gets
  32 px, which gives a 160 by 100 tile 16 places. Probe 1's ceiling at 32 px with everything
  pinned: Graphshell 100%, Turnstone 100%, Mere 86%. Probe 2 did not run at 32 px.

  Links: summary Crystal's smallest tile side is set on the recipe and defaults to 32 px.

  *New.*

- **D25. A border counts at half the smallest side (Mark, 2026-11-14):** "Half the smallest
  side". 16 px at the default; a tile may overhang a corner. This is the rule probe 2 used
  (24 px at 48), under which a star held 9 of 10 places against 6 with a full-side rule.

  Links: summary In Crystal a shared border shows a link when it is at least half the smallest tile side long.

  *New.*

- **D26. The profile is renamed now (Mark, 2026-11-14):** "Now but message the graphshell
  -> turnstone integration agent". The physics profile's id and label become `lattice` and
  Lattice in this repository at once; Turnstone's five test uses are the integration
  session's to change, and it is told.

  Links: summary The physics profile crystal is renamed lattice in Graphshell now; Turnstone follows through its integration lane.

  *Applies D10 and D15.*

- **D27. The body designer lives in the scenograph editor (Mark, 2026-11-14):** "In the
  scenograph editor". A mark is designed where scenes are designed; the node's facet pane
  shows and picks marks and does not design them. The scenograph editor plan owns the
  surface.

  Links: summary A node's body is designed in the scenograph editor, not in the node's facet pane.

  *Applies D4.*

- **D28. The mark type lives in scenograph (Mark, 2026-11-14):** "Scenograph". Beside
  `PrimitiveKind` and `RelationForm`, so the recipe, the families and the canvas can name
  it; the scene wire format does not change.

  Links: summary The node mark type is defined in scenograph, where the recipe and the families can name it.

  *New.*

- **D29. A mark accepts extents, an arrangement imposes one (Mark, 2026-11-14):** "Mark
  accepts, arrangement imposes". A mark declares which of measured, assigned and encoded it
  can take; an arrangement declares the one it imposes; compile refuses a mismatch. A card
  is one mark in Spiral and in Mosaic.

  Links: summary A mark declares the extents it accepts and an arrangement the one it imposes; a mismatch is refused at compile.

  *New.*

- **D30. Each row discloses its classes (Mark, 2026-11-14):** "Each row discloses its
  classes". An occurrence carries a list of classes, resolved through the registry the
  canvas already uses. Documents keep the card; other data gets the ladder without the Card
  rung. Three card-class test pins move under this ruling.

  Links: summary A dataset occurrence discloses its classes, and the mark is resolved from them.

  *Applies D11.*

- **D31. The companion card is a context (Mark, 2026-11-14):** "M1 does card-as-mark;
  companion is a context". M1 delivers the card as a node's own mark. A card beside or over
  a node drawn another way is a context scene, placed in phase X.

  Links: summary A card shown beside or over a node is a context scene; M1 only makes the card a node's own mark.

  *Refines D12; applies D13.*

- **D32. Paint is a type in scenograph (Mark, 2026-11-14):** "A paint type in scenograph".
  Favicon, Derived, Sprite and Bare with an open tail for paints a host adds; cartography's
  `Face` becomes that type and its saved codes stay.

  Links: summary Paint is a scenograph type with an open tail; cartography's Face becomes it.

  *New.*

- **D33. What a body is (Mark, 2026-11-14):** "None, a silhouette or an authored hull, with
  material". `NodeShape` and `PrimitiveBody` merge into one list of silhouettes; hull
  overrides stay keyed by node; the body designer (B3) edits the hull.

  Links: summary A mark's body is none, a silhouette or an authored hull, with a material.

  *New.*

- **D34. Sprite is a paint; the Sprite rung is retired (Mark, 2026-11-14):** "2 or 1
  depending on which has better resonance/sets up with isocosm's sense of sprite too. Gotta
  reconcile that one as well; be nice if 1 or 2 had a common thread. If not, meh, isocosm
  can keep sprite and we'll do 2". Reading, mine: Isocosm's sprite is a held image of a
  subject, placed at the subject each draw in place of rendering it live
  (`isocosm/shared/isometer/src/hybrid/sprites.rs:36-56`). That is what is drawn, not how
  much detail is shown, so the thread is with the paint: a sprite is an image drawn for a
  subject. Option 2 follows: `Representation::Sprite` moves to the open tail and only the
  paint is called Sprite. Isocosm's baked stand-in for a live body is nearer the Snapshot
  rung, which is noted and not acted on. The rung has four uses in this repository and none
  in Turnstone's `src` or Knot.

  Links: summary Sprite names a paint, an image drawn for a subject, as in Isocosm; the Sprite detail rung is retired to the open tail.

  *New.*

- **D35. `set_node_rule_faces` is renamed with the mark change (Mark, 2026-11-14):**
  "Rename with the mark change". It takes detail rungs and calls them faces; it is renamed
  in the change that brings the mark type to the canvas, with callers across the family
  checked then.

  Links: summary The canvas call that sets detail rungs stops being called faces when the mark type lands.

  *Applies D28.*

- **D36. The class registry moves to scenograph (Mark, 2026-11-15):** "Move the registry to
  scenograph". The recipe compiler is in scenomise and cannot call cartography's registry
  without a cycle; the registry goes beside the mark type and cartography re-exports it.

  Links: summary The representation registry lives in scenograph, where the compiler and the canvas both read it.

  *Applies D28 and D30.*

- **D37. An old "Sprite" reads as the open rung (Mark, 2026-11-15):** "Old \"Sprite\" reads
  as the open rung". A decode alias reads a saved `Sprite` as `Open { kind: "sprite" }`, so
  the fixture `scenomise/fixtures/coastal_map.json` keeps reading; Isocosm, which builds
  `Representation::Sprite` (`isocosm-views/src/projection.rs:41`), moves off it in its own
  lane.

  Links: summary A saved Sprite rung decodes as the open rung named sprite; old data keeps reading.

  *Refines D34.*

- **D38. The canvas draws all five silhouettes (Mark, 2026-11-15):** "Canvas draws all
  five". `NodeShape` merges into the one silhouette list now, and the canvas draws diamond
  and hexagon as hull colliders with their own style classes, as Mer3ly does. A visible
  change, so it takes a headed check.

  Links: summary The canvas draws square, rounded square, circle, diamond and hexagon from one silhouette list.

  *Applies D33.*

- **D39. M1 lands, and Isocosm's line is fixed with it (Mark, 2026-11-15):** "You can fix
  it and land it". Isocosm's tile board asks for the open rung named sprite first, which
  builds against Graphshell before and after, and then M1 goes to main.

  Links: summary M1 lands on main once Isocosm's one use of the Sprite rung is changed.

  *Applies D34 and D37.*

- **D40. A constructor before a new row field (Mark, 2026-11-15):** "Constructor first,
  then the field". `ProjectionOccurrence::new` lands now and breaks no one; Cleromancy,
  Knot, Mer3ly and Woodshed move to it in their own lanes; `classes` is added after.

  Links: summary A dataset row gets a constructor first, so adding its classes field breaks no repository.

  *Applies D30.*

- **D41. The four further pin moves are accepted (Mark, 2026-11-15):** "Accept the moves".
  A node with no class draws as a glyph. The spiral fixtures are tagged `document` so they
  still exercise the Card rung, the registry test reads the document profile, and four
  Spiral score hashes in the arrangement goldens change; every position hash is unchanged.

  Links: summary Unclassed nodes draw as glyphs; the spiral score pins move and positions do not.

  *Applies D11 and D30.*

- **D42. The mark type is `NodeMark` (Mark, 2026-11-15):** "NodeMark".

  Links: summary The node mark type is named NodeMark.

  *Applies D28.*

- **D43. A body is authored from a silhouette (Mark, 2026-11-15):** "the previous editor
  for the hulls of nodes was not necessarily the most efficient way to do that. It was just
  like a bunch of contours essentially around whatever the shape of the thing was. There's
  probably an easier, better way to do that", and then: "Pick a silhouette and adjust it;
  trace as a starting point".

  Links: summary A node's body is authored by picking and adjusting a silhouette, with image tracing as a starting point.

  *Refines D33.*

- **D44. Material is three numbers on the mark (Mark, 2026-11-15):** "Three numbers in
  scenograph". Restitution, friction and density; the canvas maps them to its engine.

  Links: summary A mark's material is restitution, friction and density, held in scenograph.

  *Applies D33.*

- **D45. An open paint survives a save (Mark, 2026-11-15):** "Unknown codes read back as
  the open paint". A saved code the canvas does not know is kept and drawn bare.

  Links: summary A saved paint code the canvas does not know reads back as the open paint.

  *Refines D32.*

- **D46. Registry marks accept measured and assigned (Mark, 2026-11-15):** "All accept
  measured and assigned".

  Links: summary Every mark resolved from the registry accepts a measured or an assigned extent.

  *Applies D29.*

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

- 2026-11-14, correcting the entry below: a Wasm guest does advertise commands. A block
  guest's `claim` carries `commands: list<command>` (id, label, kind, argument field) at
  mount (`crates/graphshell-endpoint/wit/world.wit:284,315`), and a projection provider
  returns snapshots carrying `AdvertisedAction`s and receives `intent` calls
  (`world.wit:174`; `crates/chirograph/src/lib.rs:197`). That makes four command shapes.
- 2026-11-14, Crystal probe 2 (a greedy packer on real rectangles; controls on a star, a
  path and a triangle passed in the same run; no overlaps and no unlinked tiles touching in
  any run). Same graphs and pin order as probe 1, smallest side 48 px, a border at least 24
  px long, unlinked tiles 8 px apart, a pinned tile allowed to grow back. Links shown as
  borders with everything pinned, against probe 1's ceiling: Graphshell 65% of 100%,
  Turnstone 51% of 94%, Mere 31% of 73%. After ten pins: 19% of 31%, 23% of 46%, 2% of 6%.
  Letting a pinned tile grow back is worth 0 to 4 points. The packer is greedy, so these
  are what one simple rule reaches, not what Crystal can reach. Receipts as below.
- 2026-11-14, Crystal probe 1 (a ceiling; no geometry solved; controls on a star and a path
  passed in the same run). Document link graphs without their index files, 10 places per
  nucleus (160 by 100 tile, 48 px smallest side), pins taken breadth-first from the
  best-connected document. Graphshell docs, 70 nodes and 118 links: 0% of nodes overflow, 10
  pins make 31% of links borders, all pins 100%. Turnstone docs, 74 and 134: 2.7% overflow,
  46%, 94%. Mere docs, 437 and 1468, one node with 116 links: 16% overflow, 6%, 73%, and
  the planar limit alone is 89%. An index file that links to everything held 80 of
  Graphshell's 198 links. Receipts: `archive_docs/receipts/2026-11-14_primitive_representations_plan_receipts.md`.
- 2026-11-14: commands are data in three places: `hest::menu::Command` and `CommandChoices`
  (`crates/hest/src/menu/mod.rs:26,83`), `chirograph::AdvertisedAction`
  (`crates/chirograph/src/lib.rs:141`), and the WIT `actions` interface's `action-envelope`
  (`crates/graphshell-endpoint/wit/world.wit:146`).
- 2026-11-14: in this repository "lattice" is an ordinary word in one test and one comment,
  not a name. Other repositories and registries were not checked.

- 2026-11-14 (M1 entry, at `96eb1b8c`): the extent rule each of the fourteen catalog families
  uses today, read from the code. Only Partition and Mosaic write an item's footprint
  (`scenomise/src/families/mod.rs:57-64`, `solve.rs:119-122`); every other family keeps the
  host's measured footprint and takes its pitch from the largest item
  (`scenomise/src/catalog/families.rs:378-511`). No family encodes an extent from a channel.

  | Family | Extent | Where |
  | --- | --- | --- |
  | Spiral | Measured; spacing at least 1.6 times the largest side | `solve.rs:52-60` |
  | Grid | Measured; cell is the largest item | `catalog/families.rs:389-396` |
  | Geographic | Measured; footprint not read, coordinates scaled by spacing | `catalog/families.rs:398-402` |
  | Hulls | Measured; cells are regions, not item footprints | `solve.rs:141-147,240` |
  | Stack | Measured; gaps from the largest item | `catalog/families.rs:413-417` |
  | Penrose | Measured; tile scale from the item pitch | `catalog/families.rs:418-435` |
  | L-system | Measured; path size from the item pitch | `catalog/families.rs:436-447` |
  | Timeline | Measured; axis and rows from the item pitch | `catalog/families.rs:448-456` |
  | Kanban | Measured; gaps from the largest item | `catalog/families.rs:457-463` |
  | Embedded | Measured; footprint not read, scale from spacing | `catalog/families.rs:464-473` |
  | Radial | Measured; ring spacing from the item pitch | `catalog/families.rs:474-488` |
  | Partition | Assigned; a tile rect or a ring sector, its area the item's weight share | `families/partition.rs:119,191` |
  | Sets | Measured; regions only, no footprints written | `families/setscape.rs:104-130` |
  | Mosaic | Assigned; a tile rect, its area weight times `unit_area` | `families/mosaic.rs:85` |

- 2026-11-14 (M1 entry): a card is the default look in four places. The recipe compiler
  stamps every item `Representation::Card` at the host's one measured size, `ItemSizes.card`
  (`scenomise/src/projection/compilation.rs:51-52,83-89`; `projection.rs:128-131`); the
  opened-link scene does the same (`scenomise/src/opened_link.rs:243`); cartography's physics
  scene falls back to Card (`cartography/src/canvas/physics_controls.rs:917-920`) and its node
  class falls back to the card class (`canvas/node_presentation.rs:82-86`). The representation
  registry picks a profile by content class (`cartography/src/representation.rs:217-240`, used
  at `spiral_score.rs:116-117`), but every profile, the "unknown" fallback included, takes the
  same ladder with a Card rung (`representation.rs:245-283,376`). A dataset occurrence carries
  field values only, nothing that marks it a document (`scenograph/src/dataset.rs:70-83`).
- 2026-11-14 (M1 entry): the registry holds two of a mark's four parts: detail, as
  `RepresentationProfile.ladder`, and body, as `PrimitiveProfile.body`, a silhouette
  (`representation.rs:24-38,198-205`). Paint is pushed per node into the canvas as `Face`
  (`cartography/src/canvas/types.rs:160-179`); extent is not named anywhere. Cartography also
  calls a `Representation` a face: `Canvas::set_node_rule_faces` takes
  `HashMap<Uuid, sceno::Representation>` and accepts only Glyph and Card
  (`canvas/node_presentation.rs:48-73`). `Representation::Sprite` (`sceno/src/scene.rs:101`)
  and `Face::Sprite` (`types.rs:175`) name the same picture as a rung and as a paint.

- 2026-11-15 (M1 code): the compiler could not reach the canvas's registry, since cartography
  depends on scenomise; the registry now lives in scenograph (`presentation/registry.rs`,
  `presentation/ladder.rs`) and cartography re-exports it.
- 2026-11-15 (M1 code): `ProjectionOccurrence` is built by struct literal in seven Graphshell
  files and in Cleromancy (`src/ambient_scene.rs`), Knot (`apps/desktop/src/composition/recipe.rs`,
  `crates/knot-composition/tests/support/recipe.rs`), Mer3ly (`src/host_dataset.rs`,
  `src/host_history.rs`, `src/spec_roadmap.rs`) and Woodshed
  (`crates/woodshed-core/examples/stage_projection_export.rs`); a new field breaks each.
- 2026-11-15 (M1 code): a saved `"representation": "Sprite"` exists in
  `scenomise/fixtures/coastal_map.json:15`; it now reads as `Open { kind: "sprite" }`
  (`sceno/src/scene.rs`, `RepresentationWire`). Isocosm builds `Representation::Sprite` at
  `isocosm/crates/isocosm-views/src/projection.rs:41`, its only use; that line becomes
  `Representation::Open { kind: "sprite".into() }` in Isocosm's lane.
- 2026-11-15 (M1 code): the canvas drew three silhouettes; diamond and hexagon now collide at
  `Silhouette::outline` scaled to the node and draw clipped to it (`.gnode-diamond`,
  `.gnode-hexagon`). Edge attachment and picking still follow the square's box. No host
  calls `Canvas::set_node_shapes` (no caller in any repository outside tests), so no
  running application shows either shape yet.
- 2026-11-15 (M1 code): giving non-document profiles the ladder without its Card rung moves
  four pins beyond the three D30 named: the spiral score tests' untagged fixtures, the
  registry's serialization test (it reads the Card rung from the first profile), and four
  arrangement golden score hashes for Spiral (position hashes unchanged). The three named
  card-class pins hold.
- 2026-11-15 (M1 code): a node's material is `seiche::NodeMaterial` (restitution, friction,
  density; `cartography/src/canvas/cartography/appearance.rs:61-76`); scenograph does not
  depend on seiche, so the mark's body carries no material yet.

- 2026-11-17 (M2): a mark includes floating-point material values. Recipes,
  drafts and their relationship wrappers now derive PartialEq. Complete literals
  need the new marks slot, and source-shaped draft copies must carry it.

## Progress

- 2026-11-14: cut from `2026-06-23_node_body_face_model_plan.md` with the scene composition
  brief; nothing landed.
- 2026-11-14: forks F1 and F3 answered (D5 to D7). M5 rewritten, phases Z and N added; nothing
  landed.
- 2026-11-14: forks F10 and F11 answered (D8, D9); phase N's done-condition extended.
- 2026-11-14: the arrangement named Crystal (D10); F2 answered (D11, D12).
- 2026-11-14: context scenes ruled (D13); phase X added.
- 2026-11-14: commands as a dataset (D14); the physics profile is Lattice for now (D15).
- 2026-11-14: forks F4, F5 and F8 and the trigger set answered (D16 to D19).
- 2026-11-14: forks F6 and F7 and the command format answered (D20 to D22). Every fork of
  the brief is answered.
- 2026-11-14: Crystal probe 1 run (a ceiling on three document graphs); the guest-command
  finding corrected. Nothing landed in code.
- 2026-11-14: the table projection plan cut (D18).
- 2026-11-14: Crystal probe 2 run (a greedy packer). Nothing landed in code.
- 2026-11-14: unlinked tiles never touch (D23).
- 2026-11-14: Crystal's smallest tile side and shortest border ruled (D24, D25).
- 2026-11-14: the physics profile `crystal` renamed `lattice` in cartography (D26); the
  body designer's home ruled (D27).
- 2026-11-14: M1 entry assessed: the fourteen families' extent rules and the card defaults
  recorded in Findings; the content catalog and the card summoning design reworded to the mark
  vocabulary. No code landed.
- 2026-11-14: M1's eight forks ruled (D28 to D35).
- 2026-11-15: on the lane's branch `m1-mark-vocabulary`, unpushed: the mark type
  (`NodeMark`, a proposed name), paint, silhouettes and body in scenograph; each family's
  imposed extent and the compile refusal; cartography on the shared paint and silhouette
  types; `set_node_rule_faces` renamed. Three of the lane's six new forks ruled (D36 to
  D38). Open: a `classes` field on a dataset row, which breaks struct literals in
  Cleromancy, Knot, Mer3ly and Woodshed; where a body's material comes from; whether an
  open paint survives a save; the mark type's name.
- 2026-11-15: M1 in part, on the lane branch: the node mark, paint, silhouette and body types
  in scenograph; each family's imposed extent and the compile refusal; cartography's `Face`
  and `PrimitiveBody` are scenograph's types; `set_node_rule_faces` renamed. Row classes, the
  registry's reach, the Sprite rung, the canvas silhouettes and material wait on forks.
- 2026-11-15: on the lane branch, the registry moved to scenograph and resolves a mark from a
  node's classes (G1); the Sprite rung retired with a decode alias (G3); diamond and hexagon
  drawn on the canvas (G4). The non-document ladder waits on a ruling for four further pins.
- 2026-11-15: M1 landed on main except row classes: the non-document ladder is in (D41),
  `ProjectionOccurrence::new` is in (D40), the name is `NodeMark` (D42). Still open: the
  `classes` field once the four repositories use the constructor; material (G5); open
  paints in the sidecar (G6); what a registry mark accepts (G7); picking and edge
  attachment for diamond and hexagon, which still use the square box (G8). No host calls
  `set_node_shapes`, so no running app shows a node shape yet and M1 has no picture.
- 2026-11-15: M1's tails landed: `Material` on the mark and
  `Canvas::set_node_mark_material` (D44); an unknown paint code reads as the open paint
  (D45). Body authoring ruled for B3 (D43); registry marks keep both extents (D46).
- 2026-11-16: Projection occurrences disclose class labels (D30, D40). Older datasets read
  with an empty class list. The compiler selects the detail rung through the shared registry
  at the reference item size (D41). Document rows can use Card, and unclassed rows use Glyph.
  An explicit host mark still overrides the registry. Class labels count toward existing
  relationship disclosure limits. This projects the detail rung into the score. Body and
  paint still belong to the mark slots work in M2.
- 2026-11-17: G8 routes diamond and hexagon picking and link attachment through the
  same polygon vertices as the painted clip and physics hull. Resting paths, responsive
  paths and flat self-links meet the body outline. Raised paths still join the stem foot.
- 2026-11-17: A native Turnstone fixture shows both polygons and checks their picking and
  link attachment. Production class-to-body resolution is part of M2.
  [G8 receipt](../../archive_docs/receipts/2026-11-14_primitive_representations_plan_receipts.md#2026-11-17-g8-polygon-picking-and-link-attachment).

- 2026-11-17: M2 adds optional node, relation and field mark slots to recipes.
  Binding, draft promotion and editor undo carry the slots. Compilation resolves
  occurrence rules before placement and retains their defaults and winning trace.
  The canvas takes complete compiled marks and live rule marks through atomic
  replacement. Relation-form realization is M3. Field forms are carried as requests.

  [M2 receipt](../../archive_docs/receipts/2026-11-14_primitive_representations_plan_receipts.md#2026-11-17-m2-recipe-mark-slots).
- 2026-11-17: Turnstone preserves recipe marks when reopening a draft and
  installs whole-mark live rules (`985a4060`). Explicit silhouettes govern
  rounding in both canvas grammars (`ac71400`). The M2 receipt records the
  completed checks and the remaining default GPU-physics Wasm dependency failure.
