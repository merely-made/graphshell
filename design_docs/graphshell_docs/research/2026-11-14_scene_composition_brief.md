# Scene composition brief: scenes built from pickable parts

**Date**: 2026-11-14
**Status**: design brief (with Mark). No code. It proposes a shape and lists the forks that
are Mark's; nothing below the Rulings section is ruled. The representation work it frames is
planned in the [primitive representations plan](../implementation_strategy/2026-11-14_primitive_representations_plan.md).
**Code read**: `crates/sceno` (scene, score, subdivision), `host/crates/cambium/scenes/scenograph`
(recipe, presentation rules, swatch, options), `host/crates/cambium/scenes/scenomise` (catalog,
families, matrix, document), `host/crates/canvas/cartography` (face, representation registry).
Read at `b957d819`.

## What was asked

Mark, 2026-11-13: tables, maps, calendars, timelines, graphs and documents "feel like the six
ways to project linked data", and scenes should "modularly use the ways we represent the basic
stuff graphs are made of (so like if you wanted to have adjacency ruled node cards like a
mosaic or a subdivision of the viewport like a partition, then that could be possible) ... i
would generally like people to be able to design scenes out of these ways of visualizing data".

Two questions follow. Which projections does the stack owe its applications? And what has to be
true of a scene for a person to assemble one from parts, instead of picking a finished family?

## Rulings this brief stands on

Given 2026-11-13 and recorded as decisions D1 to D4 of the primitive representations plan:

- The brief covers the slot model and the ten projections together.
- The body and face plan is archived and its open work re-cut under this frame.
- The forcing-proof and second-consumer gate does not hold this work back: "Everything in the
  stack runs on linked data, yes? So we have to build stack capabilities to adopt them. All are
  potential and intended consumers, which is plenty reason to implement a plan. That rule is
  poorly motivated for this scope".
- The first scene designer is a person in Graphshell's projection editor.

Given 2026-11-14, after reading this brief (D5 to D7):

- Mosaic keeps its name and its steady behaviour, and stays distinct from Partition: "a
  partition is not on an infinite canvas, but it's a subdivision of the viewport. So zooming
  a partition expands or shrinks cells relative to their containment relations." This answers
  fork F1: they are not two presets of one arrangement.
- A third arrangement is wanted, named Crystal (D10; section 3.7): tiles on the infinite canvas
  that touch when linked. Pinning starts it: "pinning creates a nucleation point for the
  restructuring of the layout". Links not shown as borders appear on selection only. This
  answers fork F3 for that arrangement.
- Dragging there: "If no pin, dropped tile stays. If pin, reflow, unless you adjust it around
  the adjacent node (for which any position adjacent is valid, but it will pack nodes into
  adjacency essentially)".

It also stands on the [projection grammar catalog](2026-08-15_projection_grammar_catalog.md)'s
ruling that a graph is not a node-link picture and that scenes are compositions of the grammar.
This brief does not replace the catalog. It says which of the catalog's layers must become
independent choices in code, and what that costs.

## 1. The ten projections

Each projection takes one facet of the data and makes it the layout. A projection is missing
when a facet has no home.

| Facet used as layout | Projection |
| --- | --- |
| Attributes | Table |
| Geographic coordinates | Map |
| Cyclic time | Calendar |
| Linear time | Timeline |
| Links | Graph |
| Authored reading order | Document |
| Containment | Outline |
| Quantity | Chart |
| Authored position | Canvas |
| One entity | Record |

Feed (rank or recency) is a possible eleventh; it may be a one-column table (fork F7). Board
and gallery are tables grouped by a category or led by an image. Matrix is the dense form of
graph. History is a mode on every projection, the time of the data and not time in the data,
and scenotime already carries it.

Where each stands in the tree:

| Projection | In code today | What it lacks |
| --- | --- | --- |
| Graph | Landed: Spiral, Radial, Embedded and the settled dynamics | Nothing structural |
| Canvas | Authored positions and holds (`Embedded`, pins) | Nothing structural; props and backdrops are the editor plan's B1 |
| Map | `Geographic` and the minimum backdrop contract | Raster basemap, field context |
| Timeline | `Timeline` arrangement | Axis guide; an interval's length as an encoded extent |
| Table | `Grid`, `Kanban`, `scenomise::matrix`, the long-form table realization | Rows that carry many fields (`Encoding` has x, y, color and label only); header guide; cell marks |
| Record | Read-only inspector drafts | Writes (editor plan, phase P); a recipe form, so a record is a scene and not a side panel |
| Document | `scenomise::document` is a bundle of linked scenes, not narrative text | A flow arrangement, or a ruling that the document projection is Knot's and embeds scenes (fork F8) |
| Outline | None | Hierarchy reading; indented arrangement |
| Chart | None | Derived marks, scales, guides (catalog gap 1) |
| Calendar | None | Cyclic-time arrangement; binning by day; day guide |

## 2. Why scenes do not compose today

The recipe already has slots. `ProjectionDefinition` carries `sources`, `reading`, `encoding`,
`arrangement`, `dynamics`, `interaction` and `appearance`
(`scenograph/src/lib.rs:354`). Presentation rules sit beside it and target the three
primitives, `PrimitiveKind { Node, Relation, Field }`, with effects that include
`Representation` and `RelationForm` (`presentation.rs:58,188`). So the skeleton is right. Four
things in it are fused or missing.

**2.1 Three families bundle four choices each.** `Mosaic`, `Partition` and `Setscape` are
`Arrangement` variants (`sceno/src/score/arrangements.rs:21`). Each fixes, together:

| | How a tile is sized | What sits beside what | Where it lives | What a node looks like |
| --- | --- | --- | --- | --- |
| Mosaic | Weight times `unit_area` | Group, then explicit order, by weighted bisection | Infinite canvas at `origin` | A tile |
| Partition | Share of a measured `whole` | Group, as nested enclosures or rings | `bounds`, the viewport | A tile or ring segment |
| Setscape | Fixed | Set membership picks the region | `bounds` | An item in a core or rim |

In code, Mosaic and Partition are close: both subdivide by weight, with a different sizing
rule and a different frame (`sceno/src/subdivision.rs`). Mark keeps them distinct (D5),
because the frame is what each one is. Neither can take another node look today.

**2.2 Relation form is declared and never read.** `RelationForm { Stroke, Containment,
Adjacency, MatrixCell }` exists as a rule effect. A search of the repository finds it only in
`presentation.rs`. No arrangement and no renderer consumes it, so choosing "adjacency" changes
nothing on screen.

**2.3 A node's look has three vocabularies and no slot.** `sceno::Representation` is a detail
ladder: Glyph, Card, Sprite, Snapshot, LivePane, Open. Cartography's `Face` is what is painted
on a canvas tile: Favicon, Derived, Sprite, Bare, with the body (collider and material) as its
other axis (`cartography/src/canvas/types.rs:161`). The catalog's mark table is a third list.
The body and face plan says "a card is not a node"; the catalog says a node can become a card.
Mark's reason for the first (D11) removes the conflict: it was a guard against drawing every
node as a card, "because not all data is documents".
The recipe has no field that names a node's look at all: the host's representation registry
picks it by content class.

**2.4 The frame is a field inside each family.** Bounded viewport, infinite canvas and nested
space are spelled `bounds: Rect` here and `origin: Vec2` there. A recipe cannot say "this
arrangement, in that frame".

## 3. The proposal: a scene is a set of slots

One choice per slot. The first five exist; the last four are what this brief adds.

| Slot | Chooses | State |
| --- | --- | --- |
| Source and reading | Which facts, selected, grouped, ordered, derived | Exists |
| Encoding | Which fields drive which channels | Exists, narrow |
| Arrangement | Geometry only: positions, and sizes where it assigns them | Exists, fused with the three below |
| Dynamics | Motion over the arrangement | Exists |
| Interaction | Selection, pan, zoom | Exists |
| **Node mark** | What an entity is drawn as | New slot |
| **Relation form** | How a relation is shown | Enum exists, unrealized |
| **Field form** | How a field or set is shown | Partly, as region guides |
| **Frame** | Where the scene lives | New slot |

The three mark slots are one per primitive, which is what "the basic stuff graphs are made of"
asks for and what `PrimitiveKind` already names.

### 3.1 Node mark: who owns the extent

The property that separates a graph node from a mosaic tile is who decides its size.

- **Measured.** The mark measures itself and the arrangement clears it. A graph node, a card on
  a canvas, a text block in a flow. This is today's `ScoreItem.footprint`, stamped by the host.
- **Assigned.** The arrangement hands the mark a rectangle and the mark fills it. A mosaic
  tile, a table cell, a calendar chip, a partition share.
- **Encoded.** A channel sets one dimension. A bar's length, an interval's span on a timeline.

A mark that can accept an assigned rectangle can sit in any subdividing arrangement. That is
the whole of "node cards ruled like a mosaic": a card mark with an assigned extent, placed by
the mosaic's geometry.

Inside the mark, the existing pieces keep their jobs:

- **Detail** is `sceno::Representation`, unchanged: the rung the mark is drawn at.
- **Paint** is what `Face` holds now. Favicon, Derived, Sprite and Bare are paints for the
  small rungs.
- **Body** is the mark's part in physics: none, a silhouette, or an authored hull, plus
  material. Only a mark with a measured extent in a scene with dynamics has one.

So body and face survive as two properties of one mark kind, the physical tile. They stop
being the model of a node's appearance.

A card is another mark kind, and it has a second use (D12): as a companion to a node drawn
some other way, placed beside it or in an overlay. A document graph draws its nodes as
cards; a graph of other data draws tiles and summons cards. Both are recipes.

### 3.2 Relation form: drawn or placed

The forms split in two, and the split decides who implements them.

- **Drawn** forms are marks made after placement: stroke, ribbon, arc, matrix cell. A renderer
  realizes them.
- **Placed** forms show the relation by where things are: adjacency, containment, order,
  alignment. They are inputs to the arrangement, as constraints.

A placed form cannot always be honoured. Tiles can all touch their related tiles only when the
relation graph is planar, and often not then. The scene already has the pattern for this:
pins are reported honoured or unmet. Placed relations take the same report, so "adjacency
ruled" means best effort with the misses named, never a silent approximation (fork F3).

### 3.3 Field form

A field or set is shown as a region with a contour, a backdrop, a tint on members, or a guide.
`Region`, the backdrop contract, `RegionGuides` and Setscape's regions are the existing pieces.
Setscape becomes a field form (sets drawn as regions) over an arrangement that places members
by region.

### 3.4 Frame

Four frames: unbounded (a point of origin on the infinite canvas), bounded (a rectangle: the
viewport or an authored one), nested (inside a node or region, through `Space`), and flow (a
page width, height growing with content). The recipe names the frame and each arrangement
declares which frames it accepts. Some accept one only, because the frame is their identity:
Partition is bounded, Mosaic and Crystal are unbounded (D5).

### 3.5 What may combine

Not every combination means something. Each arrangement declares, as data, which extents it
assigns, which placed forms it honours and which frames it accepts. The pattern is already in
the tree: families and solvers declare their options, and an undeclared key is refused at
compile before solving (editor plan, SE7 and SE8). The editor reads the same declaration to
offer only what can work, and says why the rest cannot.

### 3.6 The named scenes become presets

| Preset | Arrangement | Node mark | Relation form | Frame |
| --- | --- | --- | --- | --- |
| Mosaic | Subdivision, by unit area | Tile, assigned | Containment by group, order | Unbounded |
| Partition | Subdivision, by share | Tile, assigned | Containment by group | Bounded, viewport |
| Sets | Region placement | Any, measured | Containment in regions | Bounded |
| Orrery | Spiral or settled | Tile, measured, with a body | Stroke | Unbounded |

Mark's two examples are then: Partition with whatever node mark is chosen, a card included;
and, for cards that touch when linked, Crystal of section 3.7, which is not
Mosaic.

The ten projections in the same terms:

| Projection | Arrangement | Node mark | Relation form | Frame |
| --- | --- | --- | --- | --- |
| Graph | Spiral, radial, settled | Measured | Stroke | Unbounded |
| Canvas | Authored | Measured | Stroke or none | Unbounded |
| Map | Geographic | Measured symbol | Route stroke | Bounded, with basemap |
| Timeline | Axis | Encoded on the axis | Order, stroke | Unbounded along the axis |
| Table | Rows | Assigned row of cells | Alignment by column | Flow |
| Outline | Indented rows | Assigned row | Containment as indent, order | Flow |
| Calendar | Cyclic grid | Assigned chip | Containment in a day | Bounded or paged |
| Chart | Cartesian | Encoded, derived | None | Bounded |
| Document | Flow | Measured block | Inline link, containment | Flow |
| Record | Stack | One detailed mark | Rows of backlinks | Nested or bounded |

### 3.7 Crystal

Mark, 2026-11-14: "adjacent nodes with adjacency indicating edges, nodes resizing such that
they can all fit adjacent to, you know, the nodes that are relevant to them ... a reflowing
layout on an infinite canvas that can be shifted around as you need."

As ruled (D6, D7):

- Tiles sit on the infinite canvas. An unpinned tile stays where it is dropped.
- Pinning a tile makes it a nucleus. The layout reflows so the tiles linked to it pack
  against it, resizing to fit.
- A tile touching a nucleus may be slid to any other position against it; the rest pack
  around the change.
- A link that is not a shared border is not drawn until one of its tiles is selected.

In the slot terms: the node mark has an assigned extent, the relation form is adjacency
(placed) with a selection-only stroke as the fallback, and the frame is unbounded. A pin here
is the existing pinned role (the tile stays put) read by this arrangement as its starting
point; it is not a second kind of pin.

Also ruled (D8, D9): packing goes one step, to the nucleus's own linked tiles, and a person
grows the structure by pinning more; when a nucleus has more linked tiles than its border
holds, the ones that do not fit stay loose. Mark pictured the nucleus growing instead, and
chose pinning outward as the way to build: "pin a node, pin some adjacent nodes, then some
of their adjacent nodes". Touching rectangles can show every link only for a planar graph,
so some links will always fall to the selection-only fallback. None of this is probed.

### 3.8 Partition zoom

Ruled (D5): zooming a Partition does not scale the picture. It changes how much of the
viewport each cell gets, along its containment relations, so zooming in on a group gives it
and its members more of the viewport and the rest less. Nothing in
`scenomise/src/families/partition.rs` does this today; the family has the Tiles and Rings
forms and no zoom or drill-in input.

### 3.9 Context is a scene

Mark, 2026-11-14 (D13): "the question of what shows up contextually when a node is hovered
or brought into focus is also a question of a scene ... In some cases, you would want ... the
thing shown to be other nodes ... Other entities, data that is like pure data. But in other
cases, you want resources to be shown or controls and commands to be shown ... that
contextual question should not be prefigured. We should build to allow for the complexity of
that."

So the companion of section 3.1 is not a card slot. It is a context: a second scene, with
its own recipe, shown for a subject when something happens to it. Four parts, each a choice:

| Part | Chooses | Examples |
| --- | --- | --- |
| Trigger | What brings it up | Hover, focus, selection, context request (D17) |
| Subject | What it is about | A node, a relation, a field, a selection |
| Recipe | What is shown, by any of the slots above | Linked nodes as tiles; a resource as a live pane or snapshot; commands as a list |
| Placement | Where | Beside the subject, in an overlay, in place of the subject, in a pane |

A card beside a node, a snapshot preview and a context menu are then three recipes in the
same slot, and none is built in.

What exists to build it from, by its types: a swatch already pairs a scope with a recipe, and
its scopes include one node, a subgraph, a link, a nested graph and a selection
(`scenograph/src/swatch.rs:25`); `scenomise::document` already carries scenes linked to one
occurrence (`SceneDocument::for_occurrence`); presentation rules already fire effects on
conditions per occurrence. What does not exist: a context declared on a recipe, a trigger
vocabulary, and commands as something a recipe can read. Commands are
a dataset a recipe reads like any other (D14), so a menu is a list projection.

Commands are already data in three places, which is what the dataset is built from. Hest's
`Command` has an id, label, category, effect, destructive flag, shortcut and disabled reason,
and `CommandChoices` holds a person's added, removed and recent commands
(`crates/hest/src/menu/mod.rs:26,83`). The protocol's `AdvertisedAction` has an intent, label,
explanation, payload schema, input form and effect (`crates/chirograph/src/lib.rs:141`). The
WIT `actions` interface carries an emission as a name and a JSON payload
(`crates/graphshell-endpoint/wit/world.wit:146`).

Format, proposed under the workspace's data-format rule and not ruled: what a person authors
or edits (which commands a context shows, their order, labels, shortcuts) is TOML; the
dataset a recipe reads at run time is typed in memory and JSON where it is interchange; a
command crossing a component boundary is a WIT record. A Wasm guest can emit an action
through WIT today. A guest advertising commands of its own was not found in the WIT read
here and is not claimed. WASI adds nothing to this: it is data over WIT.

## 4. The person in the editor

The projection editor's swatch grid already varies a recipe along axes:
`AxisKind { Scope, Arrangement, Dynamics }` (`scenograph/src/swatch.rs:75`), with a live
focused cell and a pick applied as one undo step. The slot model adds axes for node mark,
relation form, field form and frame. A person then designs a scene by picking along each axis
and watching the cell, and the saved result is a recipe that an application can also ship.
Nothing new is needed for history, options or refusal; those are the editor plan's E1 to E5.

## 5. What this costs

In rough order of dependence. None of it is scheduled here.

1. One mark vocabulary: detail, paint and body named as parts of a mark, with the extent rule.
   `Representation` and `Face` keep their types; the docs and the registry stop presenting them
   as rivals.
2. Mark slots on the recipe, with presentation rules as the conditional override (fork F4).
3. Relation form read by something: drawn forms by the renderer, placed forms by arrangements,
   with the honoured and unmet report.
4. Frame taken out of the family structs.
5. Partition zoom (section 3.8), and Crystal (section 3.7) after a probe.
6. The compatibility declaration, and the four new swatch axes.
7. The projections that have nothing today: outline, calendar, chart, then document and
   record as recipes. Table needs the wider encoding first.

Items 1 to 6 are the primitive representations plan's phases. Item 7 is not planned yet; its
order is fork F5.

## 6. Forks

These are Mark's. Each returns as a question before any code that depends on it.

- **F1. Answered 2026-11-14 (D5): two distinct arrangements.** The question was: one subdivision arrangement with the two as presets, or two
  variants that each learn to accept mark, relation and frame slots. The first refines the
  2026-10-16 ruling ("Mosaic keeps its steady behaviour on a pannable infinite canvas;
  Partition is the viewport-bounded subdivision") by keeping both names and behaviours as
  presets; the second leaves the ruling as written and duplicates the slot handling.
- **F2. Answered 2026-11-14 (D11, D12).** Mark: the line "was to correct a particular
  tendency to turn all nodes into cards in a document graph, when not all nodes should be
  that, because not all data is documents." So the rule is about defaults, not vocabulary:
  no mark is the look of every node, and a card is for data that is a document. And
  (D12): "nodes can be cards, but not every node is a card". A card is a form with two
  places it can appear: as the node itself, or as a companion to a node (beside it, or in an
  overlay: a contextual card, a snapshot card, a context menu). Which place is a choice of
  the presentation. The question was: keep the line, with Card as a detail rung of a node and the
  summoned preview card as a separate overlay; or drop the line and call both cards.
- **F3. Answered 2026-11-14 (D6): relation-driven, in Crystal only; Mosaic
  keeps group and order.** The question was: relation-driven adjacency as best effort with an unmet report; or
  adjacency by group and order only, which is what Mosaic does now.
- **F4. Answered 2026-11-14 (D16): a default on the recipe, rules override.** The question
  was where a mark choice lives. A default on the recipe with rules overriding it; or
  rules only, with a default rule. The role defaults of the dynamics grammar (F22) took the
  first shape.
- **F5. Answered 2026-11-14 (D18): table first.** The candidates were: Table, outline, calendar, chart.
- **F6. Encoding width.** Table and record need many fields per item. A list of columns on the
  encoding, or a separate "fields" slot.
- **F7. Feed.** A projection of its own, or a table preset.
- **F8. Answered 2026-11-14 (D19): the document is Knot's, embedding scenes.** The question
  was: A flow arrangement in the scene stack, or Knot's projection that embeds
  scenes. This crosses into Knot and is not decided from here.
- **F12. Answered 2026-11-14 (D14): a dataset.** The question was: a recipe reads the commands available on a subject as a
  dataset, so a menu is a list projection like any other; or commands stay a separate channel
  that a context can host but not arrange.
- **F9. Answered 2026-11-14 (D10): Crystal, with the physics profile of that name renamed.**
  Mark offered "crystalline, or solid? Idk".
  `crystal` is already a physics profile id (`cartography/src/canvas/physics_catalog.rs:187`)
  and "crystallize" already names freezing a selection into a subgraph
  (`mere-view/src/subgraph.rs:228`).
- **F10. Answered 2026-11-14 (D8): one step, pin to grow.** The question was: only the nucleus's own neighbours, or onward through
  their neighbours until it runs out.
- **F11. Answered 2026-11-14 (D9): the overflow stays loose.** The question was: when neighbours cannot all fit on its border even at their
  smallest: a second ring, or the overflow stays loose with selection-only links.

## 7. What this brief does not claim

- That any combination has been built or measured. Section 3 is a proposed shape.
- That relation-driven adjacency is tractable at size. It needs a probe before F3 is put.
- That the gate in the projection grammar plan is lifted anywhere but this scope. The engine
  composition ledger (C3, C4, the ledger rows) is untouched by D3.
