# Cambium native hosts: incremental relayout adoption plan

**Date:** 2026-10-17. **Status:** approved by the owner 2026-10-17 as proposed; P1 landed be7906481.

Citations are `file:line`. Mere paths are relative to `host/crates/cambium/cambium-rootstock/src/`
(`mere:`), other Mere paths are repo-root-relative; Genet paths are relative to `components/genet-livery/src/` unless marked `genet:`.
Genet was read at `fa27ea5421c`; Mere pins `e37762d752e` (`Cargo.lock:6425`), which contains the
same retained path (`document/frame.rs` there references `layout_retained_formatting_root`).

## Verdict

1. Rootstock does a full restyle and a fresh layout for every drained mutation and never looks at
   the mutation list (`mere:frame.rs:335`, `:341`, `:359`; `mere:owned_layout.rs:156`, `:168`).
2. Genet has a complete retained pipeline, but only inside `LiveryDocument<D>`, which *owns* its
   DOM (`document.rs:185-190`). Rootstock's DOM is shared and borrowed through `WindowDom<'a>`
   (`mere:window_dom.rs:29-32`), so Rootstock cannot adopt it as is.
3. Even where the retained pipeline is admitted it is only about 2-3x faster, not the hundreds of
   times the owner wants, and it has one correctness bug and one coverage hole (measured below).
   Adoption therefore needs Genet work as well as Rootstock work.

## Answers

**1. Does any host drive retained layout? What would Rootstock call?**

- `genet-winit-host` does not. It is presentation plumbing only: surface, renderer, wheel and
  AccessKit (`genet:components/genet-winit-host/Cargo.toml:10`).
- The retained path is driven by `LiveryDocument<ScriptedDom>` in three Mere/Genet consumers:
  `genet:components/genet-documents/src/engines/livery.rs:434` and `:598`,
  `host/ports/pelt/desktop/frisket_surface.rs:741`, and
  `crates/canvas/pictograph/src/canvas.rs:407` *(historical citation)* <!-- doc-audit: historical-path -->.
- The entry points are `LiveryDocument::apply_dom_mutations(&[DomMutation])` (`document.rs:377`) or
  `mutate_dom` (`:399`), then `frame(w, h)` (`document/frame.rs:301`). `frame` tries, in order:
  repaint-only for positioned motion (`:354-360`), repaint-only for `background-color` (`:462`),
  one selected-root formatter (`:475-545`), and finally `layout_with_text_system` (`:553`).
- Rootstock cannot call `LiveryDocument` because of ownership (verdict 2). The recommended seam is
  to extract the retained state (`LayoutState`, damage, `IncrementalStyle`, identity source,
  transitions, text system; `document.rs:92-98`, `:185-221`) into a DOM-borrowing type whose
  `frame` takes `&D` per call, leaving `LiveryDocument` as a thin owner. Rootstock's
  `OwnedLayout` would hold that type and pass its `WindowDom` view (`mere:frame.rs:336-339`).

**2. Can the drained `DomMutation`s feed the damage record directly?**

Yes, with no translation. `record_dom_layout_damage` takes `&[DomMutation<NodeId>]`
(`document/frame.rs:46`), the same type Rootstock drains (`mere:frame.rs:334-335`), and maps each
mutation to its nearest formatting context via `formatting_damage_root` (`:140-165`). Caveats:

- The selected-root formatter runs only for a single damage root (`document/frame.rs:475-481`).
  Several changed rows in one frame fall back to full layout (a multi-root splice exists only as
  post-layout publication, `:566-577`).
- Multi-window: `MutationRouter` files each mutation under its window and sends unplaceable ones to
  every window (`mere:multi_host.rs:228-236`, `:326-327`). That is conservative and gives at worst
  a full-document damage.
- Interaction, stylesheet, resource and viewport events are full-document by design; their receipts
  are listed as missing in the K5h plan (`genet:docs/2026-08-11_buckram_k5h_dirty_root_execution_plan.md`,
  damage-class matrix). Rootstock's hover and focus path is one of them (`mere:owned_layout/interaction.rs:82-88`).

**3. Is style resolution incremental since the Stylo fork was deleted?**

Yes, in Livery's own `IncrementalStyle` (`invalidation.rs:65-299`). It snapshots changed
attributes, builds restyle hints, coalesces roots and re-cascades only those subtrees. It consumes
the same `DomMutation` batch (`invalidation.rs:116-124`), and it is public and DOM-borrowing, so
Rootstock could use it without the larger extraction. It falls back to full on a stylesheet or
device change (`invalidation.rs:136-153`). Differential tests exist
(`genet:components/genet-livery/tests/invalidation.rs`, 5 tests). The 2026-05-25 "fine-grained
Stylo restyle" plan (`genet:docs/2026-05-25_fine_grained_restyle_plan.md:1-8`) describes the
deleted Stylo machinery and is stale; `IncrementalStyle` is the plan of record.
Residual linear work remains (see below): `resolve_counters` over the whole plane
(`invalidation.rs:282`) and a whole-plane clone and `ch` pass in `frame` (`document/frame.rs:333-342`,
`layout.rs:1057-1059`).

## Measurements (2026-10-17, release, macOS, no fonts registered)

Keyed list, one row = row `div` (flex) + 4 cell `div`s + 3 text nodes = 8 nodes. One text node in
the middle row changes. "Rootstock-equivalent" is `resolve_styles` + `layout_with_text_system`
over the final DOM (what `OwnedLayout::rebuild` does). The retained numbers are
`LiveryDocument::frame` after `mutate_dom`. Source and raw output are in the session notes folder
(`livery_document_spike.rs.txt`, `spike_results.txt`); the probe was deleted from the repo.

| Rows | Rootstock-equivalent | Retained, block cells (not admitted) | Retained, flex cells (admitted) | Retained phases, flex (style / layout / paint) | Meristem diff |
|---|---|---|---|---|---|
| 100 | 69 ms | 57-61 ms | 26-28 ms | 4.7 / 9.1 / 13.6 ms | 0.15 ms |
| 1,000 | 685 ms | 668-697 ms | 300-311 ms | 52 / 124 / 126 ms | 1.6 ms |
| 5,000 | 3,363 ms | 4,546-4,620 ms | 2,342-2,388 ms | 450 / 1,268 / 618 ms | 10.5 ms |

### P1 timing gate (T), 2026-10-17

One changed text node in the middle row of the keyed list, run through the real frame path
(`Harness` over `GenetAppRunner`, `Host::relayout`; release, macOS, no fonts registered). Row =
`div(span label, span count, button)` with `on_click`, so 8 nodes as above. Median of 5 frames
(3 at 5,000) after one warm-up. "Before" is the same build with `IncrementalStyle::invalidate()`
forced each frame, which is the old full `resolve_styles` path plus the plane bookkeeping; the
probe was an untracked example and is deleted. Times in ms.

| Rows | Style before | Style after | Layout (full, both) | Extent | `layout_update` before | `layout_update` after |
|---|---|---|---|---|---|---|
| 100 | 11.6 | 0.1 | 36 | 0.2 | 49.7 | 37.2 |
| 1,000 | 133.3 | 1.2 | 396-409 | 3.1 | 546.6 | 424.5 |
| 5,000 | 678.1 | 10.3 | 2,192-2,308 | 18.5 | 2,957.3 | 2,426.9 |

The style stage fell to about 1 percent of its former cost (the residual is
`resolve_counters` and plane bookkeeping); the frame fell about 22 percent, as the plan expected
from leaving layout full. Layout is now 94-95 percent of the Rootstock frame at every size, so P2/P3
carry the remaining gain. The 5,000-row text-change frame is 2.4 s against the P3 bar of 50 ms.

Findings:

- **Coverage hole.** A plain text change under a block container (the common Cambium leaf) takes
  *no* retained path: 0 selected-root relayouts, and the timing equals a fresh layout. The text's
  nearest formatting context is `Inline`; `retained_root_box` accepts only Block, Flex and Grid
  (`layout/retained.rs:287-299`), and the frame loop breaks on `Unsupported` instead of promoting
  to the parent (`document/frame.rs:545`; promotion only happens from the formatter,
  `:539-544`). Admitted: text directly in a flex cell, or directly in the flex row. Rejected: text
  in a block cell, or in a block inside a flex cell. Confirmed with a temporary in-crate test using
  `retained_root_relayout_generation`, since reverted.
- **Correctness bug.** When the selected-root formatter *is* admitted, the retained paint list
  differs from a fresh document: the changed text is painted at the local origin (`y=0`) instead of
  its row (`y=1000` for row 50 of 100), at 100 and 1,000 rows. The formatter translates fragments
  to the retained origin (`layout/retained.rs:151-158`) but the paint shows the text frame was not
  translated. The existing receipts miss it because their changed text sits at the origin. The
  K5h reconciliation says `TextFrame::translate_subtree` exists for the positioned routes
  (`genet:docs/2026-08-24_k5h_retained_text_frame_reconciliation.md:17-21`); that it is the missing
  call here is my hypothesis, not verified.
- **Remaining linear cost.** Even admitted, the frame is O(N): the style phase does about 90 us per
  row with zero restyled elements, layout rebuilds the whole box tree (`layout/retained.rs:37`) and
  clones the previous styles and fragments (`document/frame.rs:487-488`), and paint re-emits the
  whole list (`document/frame.rs:599`). Incremental layout alone will not reach the diff's cost.
  Whether Rootstock's own paint (`mere:owned_layout.rs:649`) is also linear is unmeasured.
- The spike built Genet's working tree, which had another session's uncommitted
  `genet-scripted-dom` edits; they do not touch layout or style, but the numbers are not from a
  clean checkout.

## Adoption plan

Every phase lands only if its two gates pass. **Equivalence gate (E):** after each mutation the
retained result must equal a fresh layout of the same final DOM: paint-list commands, content
extent, hit-test of every row's rect, and text-find rects. This reuses Genet's pattern
(`retained_mutation_paints_like_a_fresh_final_document`, K5h plan) and compares an
`OwnedLayout` built by `new` against one updated incrementally. Cover: text change, attribute
change, row insert, row remove, row move, and two rows changed in one frame, at 100 and 1,000 rows,
with and without a scrolled viewport. **Timing gate (T):** rerun the 100/1,000/5,000-row keyed list
(recreate the probe under the scratchpad or as an untracked example, delete it afterwards), record
`stage_timings` (`mere:owned_layout.rs:190-196`) and total, one changed text node.

- **P0 (Genet, fix first): correctness and coverage.** Translate the local text frame in the
  selected-root path; add a failing receipt with the changed row away from the origin. Make an
  `Inline`-context damage root promote to its nearest Block/Flex/Grid ancestor instead of breaking.
  Gates: E on Genet's own suite plus the new receipts (not Rootstock yet); T on the
  `LiveryDocument` probe: block-cell 1,000 rows must beat the current 668 ms by a wide margin.
- **P1 (Rootstock only): incremental style.** Replace `resolve_styles` in `rebuild`
  (`mere:owned_layout.rs:156`) with `IncrementalStyle::update`, feeding the drained mutations
  through `relayout` instead of discarding them (`mere:frame.rs:335`). Full on stylesheet change,
  including the titlebar sheet (`mere:frame.rs:328-332`, `:340`). Gates: E on resolved styles
  (style plane equals `resolve_styles`) plus layout equality; T expects the style stage to fall
  from 164 ms to near the residual at 1,000 rows. Honest expectation: about 25 percent of the
  frame, because layout stays full.
- **P2 (Genet): borrowed-DOM retained session.** Extract the retained state machine from
  `LiveryDocument` so `frame` takes `&D`; `LiveryDocument` keeps its API. Moves with it: fonts and
  images (`document/resources.rs:22-76` versus `mere:owned_layout.rs:104-120` font registration),
  element and viewport scroll (`nested_scroll`, `scroll`, versus `mere:owned_layout.rs:240-262`),
  interaction states, and `caret_into_view`. Gate: zero behavior change in Genet's suite, Pelt,
  and Pictograph.
- **P3 (Rootstock): switch `OwnedLayout` to the P2 session.** Route `rebuild` through
  `apply_dom_mutations` and `frame`; keep `new` for size, sheet or font-ledger change. Gates: E
  as above across all of Rootstock's layout tests (`mere:owned_layout/tests.rs`,
  `hidden_panel_tests.rs`, `text_paint_tests.rs`); T must show the 1,000- and 5,000-row single-text
  case below the proposed bar. Proposed bar for the owner to set: 5,000 rows under 50 ms, and
  5,000-row time under 2x the 1,000-row time (sublinear).
- **P4 (Genet): remove residual O(N).** In priority order by measured share: whole-plane style
  passes in `frame` (450 ms at 5,000), box-tree regeneration and clones in the local formatter,
  and paint-list reuse for unchanged regions. A multi-root local formatter is needed for Meristem
  batches that change several rows. Hover/focus: map `set_interaction` to the existing
  `background-color`-only path (`document/frame.rs:462`) rather than `layout_resolved`
  (`mere:owned_layout/interaction.rs:82-88`); needs its own E receipt since K5h marks interaction
  events unproven.

## Measurements (2026-11-19, release, Windows laptop, a 10,000-row table)

Taken on the table view of the table projection plan (its D21, D22, D33, D34), whose gate is a
16.7 ms scroll frame at the 95th percentile and a sort shown within 100 ms. Medians; other
builds were running, so the 95th percentiles are not quoted. Raw runs, scripts and both positive
controls: `testing/graphshell/frame-cost/`.

| ms | unchanged | scroll, strip moves | scroll, window shifts | sort |
|---|---|---|---|---|
| UI total | 21.1 | 75.9 | 111.4 | 73.1 |
| style | 0 | 20.0 | 30.1 | 7.4 |
| layout and text | 0 | 24.3 | 32.0 | 22.7 |
| paint list | 3.3 | 5.6 | 6.8 | 5.0 |
| accessibility publish | 15.3 | 14.3 | 20.9 | 16.8 |
| render worker, off the UI thread | 0.1 | 42 | 42 | 39 |

- **Style.** A scroll writes `top` on the strip's `style` attribute. `IncrementalStyle::update`
  makes that element a hint root and `resolve_subtree` re-cascades its whole subtree: 529 of 543
  elements, about 37 us each. Nothing checks whether inherited values changed.
- **Sort hints.** `form_validity_tree_root` walks the document once per inserted or removed
  node; a sort makes about 190 of them (5.3 of the 7.4 ms).
- **Layout.** `OwnedLayout::layout_resolved` always calls the full `layout_with_text_system`.
  K5h's positioned-inset translation route, which is exactly the strip move, lives only inside
  `LiveryDocument::frame` and cannot be reached from the host. Inside the pass: inline groups
  18.6 ms, container-relative styles 4.5 ms (whole plane), box tree 1.9 ms.
- **Accessibility.** `sync_a11y_tree` re-projects the whole document and submits the full tree
  every presented frame, changed or not (about 21 us a node).
- **Host view.** `data_grid_with` built rows positionally, so a one-row window shift rewrote
  about 155 nodes instead of about 14.

Order, with the expected effect of each (estimates until measured):

1. Host, in Graphshell (table plan D33, in hand): no accessibility republish on an unchanged
   frame (21 to about 6 ms); keyed grid rows.
2. Genet: the form-tree check once per batch, not per mutation (sort style 7.4 to about 2 ms).
3. Genet: descend in restyle only when the element's inherited or custom values changed
   (strip-move style 20 to about 0.5 ms). Descend as before when any of
   the element's own children explicitly inherits a non-inherited property (table plan D50).
4. P2 and P3 here, carrying the positioned-inset route, with P4's whole-plane passes
   (strip-move layout 24 to about 1 to 2 ms).
5. Genet: a retained route for inserting or removing an absolutely positioned row in a
   positioned containing block, two roots a frame (window-shift layout 32 to about 2 to 3 ms).
6. Incremental accessibility updates (needs a changed-node set from Genet's layout) and
   paint-list reuse. Without these a scroll frame stays near 30 ms; with them about 12 to 17.

Not diagnosed: the render worker's 40 ms a changed frame, in presentation and netrender; it
counts toward a sort being shown.

### State on 2026-11-20

The uncommitted work in Genet's and Graphshell's main checkouts was taken over and landed
(table plan D43, D44).

- **Genet (6225f4690b5).** P0 was already on main (9a0e91074cd). `relayout_borrowed_dom`
  in `layout/retained_session.rs` is P2 for one case: a batch of text mutations at an
  unchanged viewport, refused under container queries. The positioned-inset route is not
  carried yet.
- **P3, text only.** `OwnedLayout::rebuild` takes `retain` and calls
  `relayout_borrowed_dom` when a batch is text edits at an unchanged viewport; a refusal
  falls back to the full layout. Receipt:
  `cards_title_edits_use_retained_layout_and_match_fresh_paint`. The T gate is not run, and
  a table scroll is not a text edit, so the table's numbers above do not move.
- **Label-only accessibility updates** (a first part of step 6). A producer reports a
  semantic revision and the label changes since an earlier one; the browser mirror patches
  list-item text in place, or moves the items together on a camera change, instead of
  rebuilding. The client derives the update when a diff only replaces offer labels. The
  mirror's path has no test; wasm check and clippy only.
- **Board culling.** Cartography's board emits no fill, card or title outside the viewport
  plus one pixel.
- **Portable scene reuse.** A producer may tell presentation to keep the last admitted
  portable scene when source identity, revision and size are unchanged; export always asks
  for a full scene.

Steps 2 and 3 are on Genet's main. Step 2 (48e5524aa07): a batch walks each DOM tree for
form dependencies once. Step 3 (06c3630b189, table plan D50): a change to an element's own
`style` attribute cascades that element alone when its inherited and custom values are
unchanged and none of its own children explicitly inherits a non-inherited property; restyle
roots below it are restyled on their own. Steps 4 and 5 are open and belong to the table
plan's session.

Measured after both, 2026-11-20, same table, release, one run with the machine otherwise
quiet (`testing/graphshell/frame-cost/after-genet-step3/`):

| | before | after |
|---|---|---|
| scroll frame, 95th percentile | 51 to 67 ms | 33.1 ms |
| style in a frame where a row arrives | 20 to 30 ms | 1.2 ms |
| layout and text in that frame | 24 to 32 ms | 11.6 ms |
| accessibility publish in that frame | 14 to 21 ms | 6.6 ms |
| unchanged frame, 95th percentile | 21 ms | 5.3 ms |
| sort to shown | 73 ms median | 84 to 102 ms, ten sorts |

The body now scrolls by whole rows (table plan D46), so every changed frame is a window
shift of one row (83 mutations). The breakdown is one sampled frame, not a distribution.
The scroll gate (16.7 ms) is not met: layout and the accessibility publish are what is
left, which is steps 4 to 6. Sort to shown did not improve and sits at the 100 ms gate;
its style share was not sampled in this run.

Steps 4 and 5 are one step (table plan D51). Because a changed scroll frame always carries
the strip's inset, rows built and rows removed together, an inset-only route never applies.
The merged route in `relayout_borrowed_dom` admits a batch that is one absolute element's
inset change plus inserted and removed absolutely positioned subtrees under one positioned
containing block, and refuses anything else. It needs a local formatter that takes an
absolute root with absolute children (today `supports_retained_root_formatting` refuses any
positioned box), and insert and remove, not only replace, for fragments, boxes and the text
frame. Expected, until measured: layout 11.6 ms to about 1 to 3 ms, which leaves the frame
near 20 ms until step 6's accessibility publish.

## Decisions for the owner

1. Approve the Genet extraction (P2) as the route, versus owning a Rootstock-side copy of the
   retained logic. A copy would diverge from Pelt and Pictograph.
2. Set the T bar for P3 (my proposal above is a placeholder).
3. Order: P0 is independent and fixes a real divergence; P1 is the only phase that is Rootstock-only
   and can land now for a modest gain.
