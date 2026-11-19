# Table Projection Plan

**Status:** 2026-11-19: T1, T2 and T3 done. T4 is on main with its speed gate open (D21);
T5 written in two stages, the second staged on its branch with the person's filter and
grouping control left; T6 to T8 not started. Opened by `primitive-representations D18`
(table is the first missing projection), with D20 (an ordered fields list) and D21 (a feed
is a reading).

**For:** a table is a scene recipe: entities as rows, their fields as columns, read, sorted,
filtered, grouped and windowed, shown in Graphshell and frozen as an accessible table. The
frame is the [scene composition brief](../research/2026-11-14_scene_composition_brief.md),
sections 1 and 3.10. Most of the parts exist; this plan joins them and adds the fields list.
Not here: editing cells, which follows as its own phase once graph semantics rules storage
and retraction (scenograph editor plan, phase P); the two-reading matrix, which stays as it
is; outline, calendar, chart and record, which reuse the fields list and get their own
plans; the node mark, relation form and context slots themselves
([primitive representations plan](2026-11-14_primitive_representations_plan.md)).

Receipts from this plan's runs go to `archive_docs/receipts/`, never into this plan
(DOC_POLICY §8).

## Constraints

- One row is one entity occurrence. A cell is addressed as a row and a field; it is not an
  instance of its own (D1).
- A field is described in SHACL's terms: path, name, description, order, group. One added term,
  `viewer`, taken from DASH, says how it is shown (D9). No validator is implied (D2).
- A fields list is authored as TOML and exchanged as JSON (workspace data-format rule).
- Sorting, filtering, grouping and the window are reading parameters. They never change the
  source (catalog, section 7).
- The table reads and does not write in this plan (D4).
- Each phase is verified in its own worktree before it touches main: its tests pass and the
  existing scene pins hold, or move under a ruling.

## Phases

### T1. Fields on the recipe
- Done when: a recipe carries an ordered list of fields, each with a path, name,
  description, order, group and a viewer, which says how it is shown (the plain default for
  its type, a chip, a bar in the cell or a check mark, D6, D9); a recipe without one compiles to the same
  scene as before; a field naming a path the dataset does not disclose is refused at compile
  with the field named; a fields list round-trips through TOML and through a SHACL node
  shape with no loss of the five SHACL terms; a viewer is a registered id and an unknown one
  is refused at compile with the field named (D11); a path is the dataset's key for the
  field (D12); the recipe crate reads and writes TOML behind a `toml` feature (D13).
- Needs: nothing. This is the slot outline and record will also read.

### T2. Field types
The dataset discloses text, number and true or false today. Reference, date and time, and
list are added (D3); media is phase T8.
- Done when: each new type has a disclosed value form, a defined sort order and a defined
  result for a missing value; a reference names the entity it points at by source identity,
  never by label; a list keeps its order and may be empty; existing datasets read unchanged.
- Needs: the dataset's owner for the type additions.

### T3. The table arrangement and the row mark
- Done when: a table arrangement places one row per occurrence in reading order with an
  assigned extent, in a flow frame; picking a point returns the row and the field under it;
  a selection can hold rows or cells and is saved as row and field, so it survives a sort;
  the arrangement declares what it assigns and accepts, as every arrangement does.
- Needs: the extent rule and the frame slot (primitive representations plan, M1 and M4).

### T4. The table in Graphshell
- Done when: a table recipe opens in Graphshell on the existing data grid, with a header
  built from the fields' names and only the visible rows drawn; column order follows the
  fields' order; clicking a header sorts and the sort is saved on the recipe as one undo
  step; keyboard reaches every cell and header; the frozen form is a table with a caption
  and one column header per field; at 10,000 rows scrolling holds a 16.7 ms frame at the
  95th percentile and a sort or a filter shows its result within 100 ms (D8), gated on the
  Windows laptop and measured and reported on the iMacs and the Linux machines (D10), with
  the same measured and reported at 100,000 (D5); a headed check shows it in
  Chrome and Firefox.
- Needs: T1, T3.
- Left (2026-11-19): the speed gate, fixed by incremental restyle and layout in Genet and
  the host (D22); the laptop's figures retaken after it, and the iMacs and Linux machines
  measured; three defects on the headed pages (see Open questions).

### T5. Sort, filter and group
- Done when: a recipe's reading carries a sort (several keys), a filter and a grouping; a
  group draws a heading row with its count; the editor calls grouping rows a group and a set
  of columns a section, and SHACL's `group` appears only in the exchanged shape (D7); each change is one undo step and round-trips
  through save and reopen; the same parameters drive the frozen form.
- Needs: T4.

### T6. The window (the feed reading)
A feed is "an updating log of n length where the entities are ordered and a bounded range of
them are shown" (`primitive-representations D21`).
- Done when: a reading can be ordered, updating and bounded to a range; new entities arrive
  without moving what a person is looking at or losing their selection; the range can be
  moved; a one-column table of cards ships as the feed preset; the window is a reading
  parameter that timeline and graph can take without change to this code.
- Needs: T5; the card mark (primitive representations plan, M1 and M2).

### T7. Reference cells
- Done when: a reference cell shows the entity it points at and opens a context scene for it
  on the triggers of `primitive-representations D17`; the relation behind the cell keeps its
  identity, so it can be cited and inspected; a list of references shows each.
- Needs: T2; context scenes (primitive representations plan, phase X).

### T8. Media cells
The pieces for one image per item exist: a presentation offer can be an image fetched by
content hash, and the widget layer composites textures. Nothing puts an image in a cell.
- Done when: a media field can be shown as a thumbnail (the fourth way of showing a field,
  held to this phase by D6) in its cell at the row's height, fetched by
  content hash through the existing resource path; a missing or oversized image shows a
  stated placeholder; the gallery preset (image-led rows) ships.
- Needs: a check first of whether the widget layer can lay out and decode an image element
  or needs the texture path; T2.

## Open questions

Calls made while building, standing until ruled otherwise:

- **Date-time forms (T2).** A date-time is accepted only in a form both `xsd:dateTime` and
  RFC 3339 allow, with the offset required. A form one of them alone allows is refused.
- **Column width and row height (T3).** One `column_width` for every column and one
  `row_height` for every row. Per-column widths and a per-row height are not built.
- **Row mark (T3).** A table's rows take the row mark unless the recipe names another.
- **Where the sort is saved (T4).** On the reading (`Reading.sort`), where T5 puts the
  filter and the grouping. A top-level recipe slot was the alternative.
- **Where the table lives (T4).** `cambium::projection_table`; mere-view or a crate of its
  own were the alternatives.
- **The realization's name (T4).** `table`, accepted for the table arrangement only.

- **Date-time groups (T5).** Two date-times group together when they are the same instant,
  whatever offset each was written with.
- **Nested headings (T5).** A heading at any level is a row of the grid, indented by its
  level; the grid's role does not change to a tree grid.
- **Frozen form (T5).** A frozen table shows every group open; collapse is not carried.
- **Grouping elsewhere (T5).** A reading's grouping is accepted on any arrangement that
  places by order and orders the rows there, without headings.
- **No control in the view (T5).** Filter and grouping are set through the recipe and the
  editor actions; the table itself offers no control for them yet.

Left from the headed pages (T4):

- A long title is cut without an ellipsis. Genet parses `text-overflow` and draws nothing
  for it; its line-breaking plan (`2026-10-02_livery_line_breaking_plan.md`) holds it as
  T4 behind S1.3.
- The browser's accessibility mirror shows rows and cells but no grid, no column header
  and no `aria-sort`: Genet's role lowering (`genet-render/src/a11y.rs`) has no entry for
  grid or column header and lowers a grid cell to a cell. Taproot has no role names for
  them either. Both are Genet's to change.
- A row partly scrolled out shows as a strip under the header. It is clipped correctly;
  whether scrolling snaps to whole rows is not ruled.


## Decisions

- **D1. Rows are instances, cells are addressed (Mark, 2026-11-14):** "Rows are instances,
  cells addressed". A 10,000-row table with eight fields is 10,000 instances, not about
  90,000. A cell is a row and a field; it can be selected and cited without being an
  instance.

  Links: summary In a table each row is one instance and a cell is addressed as row plus field.

  *New.*

- **D2. Fields use SHACL's terms, plus one (Mark, 2026-11-14):** "Use SHACL's terms, add
  one", after asking "what's wrong with using shacl's terms?". A field is a SHACL property
  shape (path, name, description, order, group) with one added term for how it is shown. A
  fields list can be exported as a shape and read from one. No validator is implied.

  Links: summary A field is a SHACL property shape with one added display term; fields lists exchange as shapes.

  *Refines primitive-representations D20.*

- **D3. Field types for the first table (Mark, 2026-11-14):** all four offered, reference,
  date and time, list and media, with: "i'm down for media but can the stack handle it or do
  we need to work on it?" Answer recorded in T8: the fetch and compositing pieces exist and
  the cell does not, so media is its own phase behind a check.

  Links: summary The first table handles reference, date and time, list and media fields beside text, number and boolean.

  *New.*

- **D4. Read first, writes later (Mark, 2026-11-14):** "Read, sort, filter, group; writes
  later". Cell edits follow in their own phase through the editor plan's inspect, edit,
  preview, apply pattern.

  Links: summary The first table plan reads, sorts, filters, groups and windows; editing is a later phase.

  *New.*

- **D5. Row budget (Mark, 2026-11-14):** "10,000 gated, 100,000 measured". Acceptance
  requires smooth use at 10,000 rows; 100,000 is measured and reported and does not block.

  Links: summary The first table must stay smooth at 10,000 rows; 100,000 is measured, not gated.

  *New.*

- **D6. Ways of showing a field (Mark, 2026-11-14):** "Chip, Bar in cell, Check mark,
  Thumbnail after media?". The first table has the plain default for each type plus chip,
  bar in cell and check mark. Thumbnail arrives with media cells, in T8.

  Links: summary A field is shown as its type's default, a chip, a bar in the cell or a check mark; thumbnail follows media cells.

  *Refines D2.*

- **D7. Group and section (Mark, 2026-11-14):** "Group for rows, Section for columns".
  Grouping rows is a group; a set of columns is a section; SHACL's `group` keeps its name in
  the exchanged shape only.

  Links: summary In the editor a group is a grouping of rows and a section is a set of columns.

  *New.*

- **D8. What smooth means (Mark, 2026-11-14):** "Scroll 16.7 ms, sort/filter 100 ms".
  Scrolling holds 60 frames a second, taken at the 95th percentile frame; a sort or a filter
  shows its result within 100 ms. Nothing is measured yet.

  Links: summary Smooth is a 16.7 ms frame while scrolling and a sort or filter result within 100 ms.

  *Refines D5.*

- **D9. The added term is `viewer` (Mark, 2026-11-14):** "viewer". It is DASH's term, the
  vocabulary used beside SHACL, and DASH's `editor` is there for the writing side when D4's
  later writes arrive. The values are D6's.

  Links: summary The fields list's added term for how a field is shown is DASH's viewer.

  *Refines D2.*

- **D10. Where the gate is measured (Mark, 2026-11-14):** "Laptop gated, others measured".
  Acceptance is the Windows laptop; the two iMacs and the Fedora and Mint machines are
  measured and reported and do not block.

  Links: summary The table's speed gate is measured on the Windows laptop; other machines are measured, not gated.

  *Refines D8.*

- **D11. A viewer is a registered id (Mark, 2026-11-14):** "Registered id, like
  arrangements". `viewer = "chip"` is a string checked at compile against a declared
  catalog, so a host or a guest can add viewers; an unknown id is refused with the field
  named. A viewer exports as DASH's IRI where one matches and as our own otherwise.

  Links: summary A field's viewer is a registered id checked against a declared catalog, not a closed list.

  *Refines D6 and D9.*

- **D12. A field's path is the dataset key (Mark, 2026-11-14):** "The dataset key; IRI made
  on export". `path = "title"` names the dataset's field; the SHACL shape writes it as an
  IRI under the source's namespace, and the round trip keeps the key.

  Links: summary A field's path holds the dataset's key; the IRI is made when the shape is exported.

  *Refines D2.*

- **D13. Scenograph reads and writes TOML behind a feature (Mark, 2026-11-14):** first
  "Test-only dependency", then ":? Isn't toml the pack format?" and "Maybe scenograph does
  need toml, idk", and on the question put back: "In scenograph, behind a feature". The
  recipe crate gains `from_toml` and `to_toml` beside `to_json_bytes`, under a `toml`
  feature a host turns on, so Graphshell, Turnstone, Knot and the editor's save share one
  tested reader. Recipes stay authored as TOML (`primitive-representations D22`).

  Links: summary The recipe crate reads and writes TOML itself, behind a feature that hosts turn on.

  *New.*

- **D14. List position is a field's order (Mark, 2026-11-14):** "List position; `order`
  written on export". A field carries no order number in the recipe; export writes its
  position as `sh:order` and import sorts by it, so imported gaps (10, 20, 30) become 0, 1,
  2.

  Links: summary A field's order is its position in the list; SHACL's order is written on export and read on import.

  *Refines D2.*

- **D15. Viewer IRIs (Mark, 2026-11-14):** "https://mere.computer/ns/viewer#". Our viewers
  export under that namespace, beside the relation IRIs at `ns/rel#`; a viewer DASH already
  names exports as DASH's; an id that is already an IRI exports as itself.

  Links: summary Our field viewers export as IRIs under https://mere.computer/ns/viewer#.

  *Refines D11.*

- **D16. The recipe slot reaches Distillery by a crates.io release (Mark, 2026-11-15):**
  put four times as the cost grew, and answered each time for publishing: "Distillery
  first, by me", "Publish scenograph 0.0.7, bump Distillery", "Publish the whole chain",
  "Publish the chain first, T1 waits". On the first publish: "Yes, and carry on down the
  chain", stopping on a failure or a surprise. T1 stays unpushed until the chain is done.

  Links: summary The fields slot reaches Distillery through a published scenograph; T1 waits on the release chain.

  *New.*

- **D17. Family git rows take a lower bound before a bump (Mark, 2026-11-15):** "Loosen the
  rows first, then bump and publish". A row saying `version = "0.0.3"` on a git source
  means exactly 0.0.3, so a patch release upstream breaks every consumer; rows become
  `>=0.0.x, <0.1` first, and then the upstream bumps with no red window.

  Links: summary A family git row names a lower bound, so an upstream patch release does not break its consumers.

  *New.*

- **D18. The eidetic version stamp goes to an Eidetic session (Mark, 2026-11-15):** "Hand
  the stamp question to an Eidetic session, chain waits". The release chain is paused at its
  first crate until that is ruled in Eidetic.

  Links: summary The release chain waits on Eidetic's ruling about the version stamp in its stored records.

  *Applies D16.*

- **D19. Pins were the problem, not publishing (Mark, 2026-11-15):** "the publishing chain
  was not the problem and in fact it's good to publish, but the other problems were a
  result of a practice i banned anyway (pinning is a problem when the pins change, and
  that's what they do when you're developing prerelease software, so as a strategy, pins
  and locks were never gonna make sense for this stage)". And on how changes land: "what I
  would like is to raise a floor at the end of fixing breaking change, not to do a ceremony
  every time I make any change on the front end ... There's no team, there are no
  consumers. This is a prototype, these are prototypes." A change lands, what it breaks is
  fixed after, and the floor is raised then. Publishing stays welcome. T1 did not wait for
  a release: Distillery takes Graphshell's crates from Graphshell's git.

  Links: summary Exact-version rows caused the release chain's cost, not publishing; a change lands, its breakage is fixed after, and the floor is raised then.

  *Refines D16 and D17.*

- **D20. A family row names a floor only (Mark, 2026-11-15):** "why pick one version and
  call out that one version as the one to use, as opposed to just being like anything above
  this baseline version will be fine?" Rows read `>=0.0.x` with no cap; the floor is raised
  when a break has been fixed.

  Links: summary A family git row names a floor with no cap, raised after a break is fixed.

  *Refines D17.*

- **D21. T4 lands with its speed gate open (Mark, 2026-11-19):** "Land it, gate stays
  open". The table is on main; the 16.7 ms and 100 ms figures of D8 stand and are not met
  (scroll 104.9 ms, sort 133.8 ms at 10,000 rows on the laptop). T4 is not done until they
  are.

  Links: summary The table lands on main with D8's speed gate recorded as unmet and still required.

  *Applies D8.*

- **D22. The speed fix is at the cause (Mark, 2026-11-19):** "Incremental restyle and
  layout". A change to one node should cost one node in Genet and the cambium host; the
  table does not paint its cells as a layer to get around it. The work is its own lane,
  outside this plan's repository where it is Genet's.

  Links: summary The table's frame cost is fixed by incremental restyle and layout in Genet and the host, not by painting cells.

  *New.*

- **D23. A filter is conditions that all hold (Mark, 2026-11-19):** "Conditions that all
  hold". Each condition is a field, an operator and a value. Every kind takes is, is not, is
  empty and is not empty; ordered kinds take less than, at most, greater than, at least and
  between, by the dataset's own order; text takes contains and starts with; a list takes
  contains an item. There is no OR yet; an any block can be added without changing the saved
  form.

  Links: summary A reading's filter is a list of field conditions that must all hold.

  *New.*

- **D24. Grouping takes several levels (Mark, 2026-11-19):** "Several levels". Saved as a
  list of field and direction like the sort, one heading level per key.

  Links: summary A reading's grouping is an ordered list of fields, one heading level each.

  *New.*

- **D25. A group heading is a band in the arrangement (Mark, 2026-11-19):** "Bands in the
  arrangement". The table arrangement carries each heading's position, label and count and
  rows skip past it; picking can hit a heading; the grid draws it as a row with one spanning
  header cell. A heading is not an instance and selecting one selects nothing.

  Links: summary Group headings are bands the table arrangement places, not instances.

  *Applies D1.*

- **D26. Shift adds a sort key (Mark, 2026-11-19):** "Shift+click adds a key". Shift+click
  or Shift+Enter on a header adds the column as the next key, or flips it if it is already
  one; a plain click replaces the sort as before. Headers show each key's priority. Pointer
  clicks carry modifier keys.

  Links: summary Shift+click on a header adds or flips a sort key and a plain click replaces the sort.

  *New.*

- **D27. Groups follow the field's order (Mark, 2026-11-19):** "Field's order, 'No value'
  last". Ascending or descending by the grouping field's own order, with the rows that have
  no value in a last group named No value.

  Links: summary Groups are ordered by the grouping field, with a No value group last.

  *New.*

- **D28. A list groups by its whole value (Mark, 2026-11-19):** "Whole value". Each distinct
  list is one group and a row appears once. A date groups by its exact value; month and year
  buckets wait for the calendar plan's binning.

  Links: summary Grouping by a list or a date uses the whole value, so a row appears in one group.

  *Applies D1.*

- **D29. Collapse is view state (Mark, 2026-11-19):** "Collapse, as view state". Groups
  collapse. Which are collapsed is saved beside the table's selection, keyed by group value;
  it is not in the recipe and not an undo step.

  Links: summary Collapsed groups are view state saved beside the selection, not recipe state.

  *New.*

- **D30. The filter stays in the recipe (Mark, 2026-11-19):** "No, recipe only". The
  exchanged shape stays the fields list; filter, sort and grouping travel only in the
  recipe.

  Links: summary Filter, sort and grouping are not written into the exchanged SHACL shape.

  *Applies D2 and D12.*

- **D31. Text matching (Mark, 2026-11-19):** "contains ignores case, is is exact". Contains
  and starts with ignore case by simple lowercasing; is matches exact bytes.

  Links: summary Text contains and starts-with ignore case; is compares exact bytes.

  *New.*

- **D32. The saved key is section (Mark, 2026-11-19):** "Yes, rename the saved key". A
  field's `group` is `section` in code and in the saved recipe; `sh:group` is written only
  in the exchanged shape. No alias for the old key.

  Links: summary A field's column set is saved as section, and sh:group appears only on export.

  *Applies D7.*

- **D33. Host speed fixes first (Mark, 2026-11-19):** "Yes, both now". The accessibility
  tree is not republished when nothing changed, and the grid's rows are keyed. Both are in
  Graphshell and are measured again after.

  Links: summary The unchanged-frame accessibility republish and unkeyed grid rows are fixed first, in Graphshell.

  *Applies D22.*

- **D34. Genet's part goes to the session that holds it (Mark, 2026-11-19):** "Hand findings
  to the Genet session". The measured breakdown and the ordered list go into the incremental
  relayout adoption plan (`cambium_docs`, 2026-10-17) and to the session holding the
  uncommitted retained-session work in Genet; no second lane starts beside it.

  Links: summary The Genet speed work is carried by the session already holding the retained-session work.

  *Applies D22.*

- **D35. A grouped field's header shows the group's direction (Mark, 2026-11-19):** "The
  group's direction". The header of a field the table is grouped by shows the grouping's
  direction and a group mark, not a sort rank.

  Links: summary A grouped field's column header shows the grouping's direction and a group mark.

  *Refines D26.*

- **D36. A list has its own contains (Mark, 2026-11-19):** "contains_item, separate". Text
  contains matches a substring; a list is tested with `contains_item`, which matches one
  whole item.

  Links: summary List membership is the contains_item operator, separate from text contains.

  *Refines D31.*

- **D37. Ordered operators (Mark, 2026-11-19):** "Number, date, date-time". Less than,
  greater than and between are accepted on those three kinds and refused on the rest.

  Links: summary Ordered filter operators apply to number, date and date-time fields only.

  *Applies D23.*

- **D38. A missing value (Mark, 2026-11-19):** "Passes is-not; empty includes blank". A row
  with no value for the field passes is not and fails every other comparison; is empty holds
  for a missing value, blank text and an empty list.

  Links: summary A missing value passes is-not, and is-empty also holds for blank text and an empty list.

  *Applies D23.*

- **D39. Ranks count the keys still sorting (Mark, 2026-11-19):** "Renumber: 1 and 2". With
  a table sorted by three fields and grouped by the first, the other two show ranks 1 and 2.
  The saved sort keeps all three, so ungrouping brings back 1, 2, 3.

  Links: summary Sort ranks number only the keys that order rows inside the groups.

  *Refines D35.*

## Findings

- 2026-11-15: adding a slot to a recipe type breaks every repository that builds one as a
  struct literal. Distillery has one site (`crates/distillery/src/chronicle_recipe.rs:34`),
  and Graphshell's own full build pulls Distillery, so T1's commit hook fails; Turnstone has
  nine sites; Knot none. `Default` for `ProjectionDefinition` and
  `AuthoredProjectionDefinition` is on main (`f0e974d4`) so a caller can write
  `..Default::default()`.
- 2026-11-15: Distillery takes scenograph 0.0.6 and scenomise 0.0.7 from crates.io;
  Graphshell's build patches them to its own copies. So Distillery's recipe must compile
  against the published crate and the working copy at once.
- 2026-11-15: the release chain, traced from `cargo metadata` over the 41 crates scenograph
  and scenomise reach from outside crates.io, compared with the crates.io index by version
  and feature. Six need a release, in this order: eidetic (0.0.3 published without the
  `product` feature catchment needs; Eidetic), catchment 0.1.0 (never published; Mere),
  incipit 0.2.3 (0.2.1 published), sceno-backdrops 0.0.1 (never published), scenograph
  0.0.7, scenomise 0.0.8. Not checked: whether a crate's code drifted from a published
  version of the same number (sceno last changed 2026-11-06, after its 0.0.7 bump on
  2026-10-31). A dry run cannot verify a crate until the one below it is on crates.io.
- 2026-11-15: rows loosened to `>=0.0.x, <0.1` on main (D17): Mere `6eeef0408` (five
  eidetic rows), Turnstone (eidetic, scenograph, scenomise), Knot `281fc99` (the same
  three), Distillery `477c5f7` (two eidetic rows), Graphshell `9cf676de` (one).
- 2026-11-15: the chain stopped at eidetic. At 0.0.4, two tests in
  `eidetic-core/tests/capture_golden.rs` fail, because eidetic stamps its crate version into
  the content-addressed schema records it stores (`tooling: "eidetic/0.0.3"`). Nothing was
  published and nothing was pushed to Eidetic. The brief for the Eidetic session is at
  `Code/testing/eidetic_version_stamp_brief.md`.

- 2026-11-19 (T2): `xsd:dateTime` and RFC 3339 disagree at the edges. XSD allows a
  date-time with no offset, which names no instant, and `24:00:00`; RFC 3339 allows lower-case
  `t` and `z`, a leap second `:60` and offsets up to 23:59, where XSD stops at fourteen hours.
  T2 accepts only the forms both allow (`scenograph/src/temporal.rs`), and a date only as
  `YYYY-MM-DD`, though `xsd:date` also allows a time zone on a date. No date library was
  needed: about seventy lines read both forms and turn a date-time into an instant.
- 2026-11-19 (T2): presentation rules read an occurrence's values as scalars
  (`scenomise/src/projection/marks.rs`); references, dates and lists are left out of a
  rule's facts, so no rule can test them yet. Turnstone keeps its own copy of that mapping
  (`ports/graphshell/src/host_presenter.rs:200`).

- 2026-11-14 (T1): today's recipe survives TOML with no special handling; an absent
  optional value is left out on writing and read back as absent
  (`scenograph/src/toml_io.rs`, test `a_recipe_written_by_hand_reads_and_round_trips`).
- 2026-11-14: TOML is parsed in one place today, the mod loader
  (`host/crates/script/document-host/src/mod_loader/loader/free_fns.rs:114`), for
  `.mod.toml` and `.wasm.toml` manifests. It carries no recipe or projection contribution,
  so a recipe shipped in a pack is not built anywhere. Scenograph writes its own JSON
  (`to_json_bytes` on three recipe types) and has optional fields with no skip rule, which
  TOML cannot write as empty; whether today's recipe survives TOML is unchecked.
- 2026-11-14: DASH's form document (datashapes.org/forms.html, marked work in progress as of
  June 2019) defines ten viewers: BlankNodeViewer, DetailsViewer, HTMLViewer,
  HyperlinkViewer, ImageViewer, LabelViewer, LangStringViewer, LiteralViewer, URIViewer and
  ValueTableViewer. None is a chip, a bar or a check mark, and there is no viewer for true
  or false. `dash:ImageViewer` draws an image and says nothing of thumbnails. A property
  shape names its viewer with `dash:viewer`.

- 2026-11-14 (at `a0469fc1`): the data grid exists. `cambium::data_grid` is a sticky header
  over virtualized, absolutely placed rows with header clicks for sorting
  (`host/crates/cambium/cambium/src/grid.rs`), over `sprigging::GridSpec` and
  `VirtualWindow` (`host/crates/cambium/sprigging/src/grid.rs`).
- 2026-11-14: a dataset already discloses typed fields per occurrence:
  `ProjectionDataset.fields: BTreeMap<String, ProjectionFieldType>` with `Text`, `Number`
  and `Boolean`, and each occurrence's `values` (`scenograph/src/dataset.rs:25,79`).
- 2026-11-14: the recipe's `Encoding` has x, y, color and label only
  (`scenograph/src/lib.rs`), so a table has nowhere to list its columns today.
- 2026-11-14: the frozen forms exist: `FrozenGrid` is a captioned table with row and column
  headers, and `FrozenScene` has a long-form table alternate
  (`crates/graphshell-client/src/frozen.rs`).
- 2026-11-14: the matrix makes cells, rows and columns each an instance
  (`scenomise/src/matrix.rs`: `cell_instance`, `row_instance`, `column_instance`). The table
  does not follow it (D1).
- 2026-11-14: an image can be offered per item as `PresentationCodec::ImageV1` with a
  content hash (`crates/chirograph/src/lib.rs:202`), and `cambium::external_texture`
  composites a texture into a box (`host/crates/cambium/cambium/src/tags.rs:74`). No image
  element helper was found in `tags.rs`, and the dataset has no media field type. Whether
  the layout engine decodes an image element was not checked.
- 2026-11-14: Mere's design docs mention SHACL in twelve files; a tracked-file search found
  no shape reader in code in Mere, Graphshell or Knot.

- 2026-11-19 (T3): the data grid T4 will use sizes a column by a fixed width and every row
  by one height, under a header height, with no gaps (`sprigging/src/grid.rs:14-50`).
  `sceno::Table` (`crates/sceno/src/table.rs`) has the same shape: an origin, column
  widths in field order, a row height and a header band.
- 2026-11-19 (T3): no family could see the recipe's fields when its arrangement was built;
  `catalog::Measure` gains `columns`, the field count or one. Knot builds a `Measure` as a
  struct literal at three sites (`apps/desktop/src/composition/recipe/editor.rs:46,61,423`),
  which break.
- 2026-11-19 (T3): the compiler refused every selection mode but single, for every family
  (`scenomise/src/projection/validation.rs`); a table now takes multiple as well.
- 2026-11-19 (T3): Turnstone's comparison grid lists every catalog family
  (`ports/graphshell/src/projection_compare.rs:88`), so a Table swatch appears there with
  no change in Turnstone.

- 2026-11-19 (T4): before T4 nothing in Graphshell opened a recipe on screen: the
  compiler's only callers outside the scenes crates were cartography's tests and examples,
  and the compiler refused every realization but `canvas`
  (`scenomise/src/projection/validation.rs`). Turnstone's `ports/graphshell` is where a
  projection workspace runs. T4's table is a Cambium component,
  `cambium::projection_table`, beside `opened_link`, drawn by the desktop host and the
  browser host from one fixture (`cambium/examples/support/table_fixture.rs`).
- 2026-11-19 (T4): a scene's source interning searched the source list once per item
  (`sceno::Scene::intern_source`, called from `scenomise/src/solve.rs`), which made a
  compile quadratic: 752 ms at 10,000 rows. The solver now keeps an index; 101 ms.
- 2026-11-19 (T4): any DOM change in a Cambium host restyles and lays out the whole
  document and republishes its accessibility tree. One scroll frame of the 480 px table
  costs about 95 ms of UI time with one mutation and 97 ms with 24; a frame with nothing
  changed costs 21 to 23 ms. The table's speed gate cannot be met by the table alone
  ([receipts](../../archive_docs/receipts/2026-11-14_table_projection_plan_receipts.md)).
- 2026-11-19 (T4): Taproot's role selectors have no token for `grid`, `columnheader` or
  `gridcell` (Genet's `accesskit_role_token`), so a scenario reaches the table by class
  and attribute (`click .grid-header-cell @aria-colindex=2`).
- 2026-11-19 (T4): in Chrome and Firefox the browser host's accessibility mirror held no
  `columnheader` or `gridcell` element for the table, and Roboto, the only face the page
  brings, has no arrow glyph for the sorted header.
- 2026-11-19 (T4): Cambium's runner takes focus from key listeners, not `tabindex`; the
  grid's roving tab stop marks only the stop's key listener focusable.

- 2026-11-19 (T5): a several-key sort is already whole below the headers: the reading holds
  `Vec<SortKey>`, `sort::order` breaks ties key by key, the compiler refuses an undisclosed
  or repeated key, `EditorAction::SetSort` takes any list as one step, and the view's
  sort-only path re-sorts without a compile. Only the header sets one key
  (`scenograph/src/sort.rs`, `header_click`), and a click cannot ask for more:
  `PointerClick` carries no modifiers (`cambium/src/event.rs:42`); `KeyEvent` does.
- 2026-11-19 (T5): ARIA puts `aria-sort` on one header at a time, so the grid and the
  frozen table mark the lead key only; the status line now names every key.
- 2026-11-19 (T5): timed outside the frame, median of 9 runs (3 at 100,000), release
  build on the laptop with other lanes building (dev build in brackets). At 10,000 rows: a
  compile 107 ms (203); a one-key re-sort 6.8 ms (21.5); a date key 22.1 ms; three keys
  36.8 ms (125); one filtering pass over the sorted rows 4.2 ms (5.3); a compile of the
  filtered dataset (511 rows kept) 60 ms (45); grouping by one field as a stable partition
  38 ms (45), or as a leading sort key 30 ms (103). At 100,000: compile 1,286 ms, one key
  142 ms, three keys 626 ms, filter pass 63 ms, filtered compile 428 ms, group 506 to 615
  ms. So a filter or grouping that compiles again cannot meet D8's 100 ms at 10,000 on its
  own; one applied over the reading order, as the sort is, can. Frame cost is separate
  (D21, D22) and adds about 95 ms a frame today.
- 2026-11-19 (T5): a date or date-time is parsed on every comparison
  (`dataset::compare_values`, `parsed_cmp`), which makes a date key about three times a
  number key; parsing each row's key once before sorting would remove it.
- 2026-11-19 (T5): the saved fields list called a field's set of columns `group`, the
  word D7 keeps for the exchanged shape; no saved recipe in Graphshell, Turnstone,
  Distillery, Knot or Mere carried one.
- 2026-11-19 (D33): an accessibility publication reads the DOM through its mutation
  sequence, the retained layout's session, generation and scroll planes (the identities
  `PresentedInput::matches_dom` and `matches_layout` already use for admission), the focus,
  the layout scale and the adapter's state (`frame/accessibility.rs`, `A11yInputs`). Leaves
  and texture producers describe themselves through `Leaf::accessibility` and
  `TextureProducer::semantics` with no change signal; `SceneProducer` reads an
  `Rc<RefCell<_>>` source the application also holds. While either registry holds anything,
  the tree is read every frame as before.
- 2026-11-19 (D33): a window shift's remaining DOM mutations are the arriving row's own
  construction, 81 per row of six cells; the rows that stay are not written.
- 2026-11-19 (T5 stage 2): through the table's own calls, with no compile, at 10,000
  rows (release): a three-condition filter 4.7 ms, a grouping by one field 26.8 ms
  and by two 31.0 ms, a three-key sort 14.8 ms, collapsing a group under 0.1 ms. At
  100,000: 50, 396, 445 and 257 ms
  ([receipts](../../archive_docs/receipts/2026-11-14_table_projection_plan_receipts.md)).
  Each is inside D8's 100 ms at 10,000 before the frame (about 95 ms today, D21).
- 2026-11-19 (T5 stage 2): one sort serves sort and grouping: the group keys lead the
  sort keys (`scenograph::group::order_keys`), and the filter runs first, so fewer
  rows are sorted (`scenomise/src/projection/compilation.rs`, `table_reading`). The
  compile and the view read through that one function; tests compare them.
- 2026-11-19 (T5 stage 2): serde reads a struct from a list of its fields' values, so
  an untagged filter value tried as a reference before a list read `["2020-01-01",
  "2024-01-01"]` as a reference; the list form is tried first
  (`scenograph/src/filter.rs`, `FilterValue`).
- 2026-11-19 (T5 stage 2): ARIA's grid role has no group level; a heading row is a
  `row` with one `rowheader` spanning the columns (`aria-colspan`) and
  `aria-expanded`, and its level shows only by indent and words
  (`cambium/src/grid/heading.rs`). A `treegrid` would carry `aria-level`; Genet's
  role lowering has neither yet (see Open questions).
- 2026-11-19 (T5 stage 2): HTML cannot nest a `<tbody>`, so the frozen form gives
  each innermost group a body; an outer heading leads its first inner group's body,
  and each inner `<th scope="rowgroup">` names its outer heading with `headers`
  (`graphshell-client/src/frozen_table.rs`).

## Progress

- 2026-11-14: plan written with Mark; nothing landed.
- 2026-11-14: row budget, ways of showing a field and the group and section names ruled (D5
  to D7).
- 2026-11-14: smooth defined and the added term named `viewer` (D8, D9); the gate machine
  ruled (D10).
- 2026-11-14: T1's forks ruled (D11 to D13).
- 2026-11-14: T1 written (branch `t1-fields`, unpushed, D16). A recipe carries `fields` (`scenograph::FieldSpec`: path, name,
  description, group, viewer), absent from the saved form when empty, so earlier recipes
  read and write unchanged. The compiler refuses a path the dataset does not disclose and a
  viewer the host does not declare, naming the field (`scenomise/src/projection/fields.rs`);
  `ViewerCatalog` holds default, chip, bar and check and a host adds to it with
  `ProjectionCompiler::with_viewers`. `fields::to_shape` and `from_shape` carry the list as
  a SHACL node shape in JSON-LD; `to_toml` and `from_toml` sit behind the `toml` feature.
  Not in T1: an editor action for the fields list, and a recipe contribution in the pack
  loader. Tests: scenograph 57 passed with `--features toml`, scenomise 200 passed.
- 2026-11-15: T1 is written and its tests pass (scenograph 57 with `--features toml`,
  scenomise 200) on branch `t1-fields`, unpushed: it waits on the release chain (D16),
  which waits on Eidetic (D18). Field order and viewer IRIs ruled (D14, D15); recipe
  defaults landed; family rows loosened in five repositories (D17).
- 2026-11-15: T1 landed on main, merged with M1 (the compiler checks both the mark's extent
  and the fields). Distillery takes sceno, scenotime, scenograph, scenomise, incipit,
  chirograph, graphshell-endpoint, cambium, hest and mere-surface-api from Graphshell's git
  and its recipe defaults later slots (`cf8b9e4` and the commit before it); Graphshell
  patches those to its local copies. Floor-only rows are on main in thirteen repositories
  (D20). Another session published eidetic 0.0.4, catchment 0.1.0, incipit 0.2.3 and
  sceno-backdrops 0.0.1 before the chain was dropped; scenograph 0.0.7 and scenomise 0.0.8
  are the versions on main and are not published. Tests: scenograph 70 with
  `--features toml`, scenomise 202.
- 2026-11-15: D19 corrected. It first read as "no release is needed to develop"; Mark's
  point was that the pins were the fault and publishing is good. Every stop in the chain
  (the eidetic bump breaking four repositories, Inker, the five-repository sweep) was an
  exact 0.0.x row changing under a consumer. scenograph 0.0.7 and scenomise 0.0.8 are on
  main and can be published when he says.
- 2026-11-16: Turnstone `63affcef` took the new fields slot at all nine sites and raised
  its two Git dependency floors. The definition-to-draft conversion preserves the ordered
  fields through edit, save and reopen. Three projection tests and the Windows check
  with authoring enabled passed against Graphshell `1bdb2ce3`.
- 2026-11-19: T2 written (branch `t2-field-types`, unpushed). The dataset discloses
  reference (a `sceno::SourceRef`), date, date-time and a list of one item type
  (`{"list": "<item>"}`; a list of lists cannot be disclosed); the first three types keep
  their saved spelling. `ProjectionValue::refusal` checks a value against its disclosed
  type, dates by their lexical form, and the compiler names the occurrence and field;
  `dataset::compare_values` is each type's sort order, a missing value last, date-times as
  instants. Tests: scenograph 91 with `--features toml`, scenomise 245.
- 2026-11-19: T3 written on branch `t3-table-arrangement`, unmerged. A `table` family in the
  catalog: one row per occurrence in reading order (x orders it, occurrence id breaks
  ties), each row an assigned rectangle the width of its columns, in a flow or unbounded
  frame; one column per field in list order, one label column when the recipe lists none;
  options `column_width`, `row_height` and `header`, measured from the host's item size.
  Rows default to the row mark (`NodeMark::row`, an open `row` rung that accepts an
  assigned extent only). `scenomise::table::TableLayout` picks a point to a row and field
  or a header, and gives a target's rectangle; `scenograph::table::TableSelection` holds
  rows and cells saved as row and field and keeps them across a sort. Column widths and a
  per-row height wait on rulings. Tests: sceno 43, scenograph 79 with `--features toml`,
  scenomise 251.
- 2026-11-19: T4 landed with its speed gate open (D21). A reading carries `sort`
  (`scenograph::SortKey`, field and direction, absent when empty); order-placing families
  place by it, and a sort on an undisclosed or repeated field, or on a family that does
  not place by order, is refused. The compiler accepts the `table` realization for the
  table arrangement. `EditorAction::SetSort` is one undo step and never coalesces.
  `cambium::projection_table` opens a recipe on `data_grid_with` (sort state, a roving
  focus, a grid key handler): headers from the fields' names in list order, only the
  window's rows, header click or Enter sorts, arrows, Home, End and Page keys reach
  every header and cell, the four viewers draw, and a sort alone re-sorts through the
  compiler's reading order. `graphshell_client::frozen_table::FrozenTable` is the frozen
  form. Not met: the speed gate (scroll p95 104.9 ms, sort 133.8 ms at 10,000 rows on
  the laptop); iMacs and Linux not measured. Headed in Chrome 157 and Firefox 160: scroll
  frame interval p95 106.8 and 206.0 ms.
  Distillery's reading literal defaults its later slots first, so the new slot does not
  break it. Open on the headed pages: the sort arrow has no glyph in the page's font, and
  the browsers' accessibility mirror showed no column headers or cells.
- 2026-11-19 (T4 defects): the sorted header's arrow is a drawn box (`grid-sort`, CSS borders),
  not a glyph, so Chrome shows it with Roboto alone; `aria-sort` is unchanged. Not changed:
  `text-overflow` has no code in Genet's Livery, so a long title is still cut without an
  ellipsis; the row "sliver" under the header is the partly scrolled row, drawn the same on
  the desktop, not a clipping fault; the browser mirror lowers `row` and `cell` but not
  `grid`, `columnheader`, `aria-sort` or the index attributes, a gap in Genet's role
  vocabulary.
- 2026-11-19: T5 stage 1 landed; when it was written the filter,
  the grouping, heading rows and a several-key header gesture waited on rulings. A field's
  set of columns is saved as `section` and written as `sh:group` only in the shape (D7).
  The table's status line names every sort key in priority order. New tests: a
  several-key sort read from TOML and round-tripped; one undo step, the status line and
  the frozen form's order for a several-key sort. Totals: scenograph 100 with `--features toml`,
  scenomise 278 with `--features authoring`, cambium 438, graphshell-client 72.
- 2026-11-19: T5's forms ruled (D23 to D32). A filter or grouping change takes the table's
  no-compile path, as the sort does: one shared reading order, then filter, then group (a
  full compile is 107 ms at 10,000 rows). Speed: the frame's cost was broken down (receipts
  under `testing/graphshell/frame-cost/`); host fixes go first (D33) and Genet's part is
  handed over (D34).
- 2026-11-19 (D33): the host republishes the accessibility tree only when something its
  projection reads moved, and the data grid keys its rows by row index (branch
  `host-frame-fixes`, staged, not committed). An unchanged frame went from 9.5 and 12.7 ms
  (p50) to 3.4 ms, its accessibility share from 6.7 and 8.9 ms to 0.3 ms; scroll and sort
  are within noise. A window shift no longer rewrites the rows that stay (155 to 161
  mutations before, 83 a row after). With a leaf or producer registered every frame still
  republishes. Tests: cambium 439, cambium-rootstock 246 (18 ignored; the native tail
  control passed when run), cambium-a11y 53 and 10, sprigging 30, cambium-desktop 26.
  Speed gate still open
  ([receipts](../../archive_docs/receipts/2026-11-14_table_projection_plan_receipts.md)).
- 2026-11-19: T5 stage 2 written (branch `t5-sort-filter-group`, staged, not
  committed). A reading carries `filter` (`scenograph::filter::Condition`: field, op,
  value; D23, D31) and `group` (`SortKey`s, D24), absent when empty in TOML and JSON.
  The compiler refuses a condition or group key on an undisclosed field, an operator
  the field's kind does not take, a value of the wrong kind, a field grouped twice and
  either on an arrangement that does not place by order, naming the field. The compile
  and the table read through one function (`ProjectionCompiler::table_reading`): the
  filter, then one sort with the group keys leading, then a band per group, "No
  value" last (D27, D28). `sceno::Table` carries `headings` and rows move past them;
  `TableLayout::pick` returns `TableHit::Heading` (D25). `EditorAction::SetFilter` and
  `SetGroup` are one undo step each. The grid draws a heading as a row with one
  spanning `rowheader`, its count and `aria-expanded`, counted in `aria-rowcount`
  (`grid::data_grid_banded`); a click, Enter or Space collapses it, saved as
  `scenograph::table::CollapsedGroups` beside the selection (D29). `PointerClick`
  carries modifiers from both hosts; Shift+click and Shift+Enter add or flip a sort
  key and the headers show each key's priority (D26). The status line states the
  filter and grouping in words; the frozen form puts each innermost group in a
  `<tbody>` with `<th scope="rowgroup">` headings. A date is parsed once before a
  sort. Pictures under `testing/graphshell/table-t5/desktop-10k/`. Not built: a
  person's control for adding a filter or a grouping; the example's scenario sets
  them. Tests: sceno 44, scenograph 112 with `--features toml`, scenomise 286 with
  `--features authoring`, cambium 446, graphshell-client 73.
