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
  editor actions until the header menu is built (D49).

Left from the headed pages (T4):

- A long title is cut without an ellipsis. Genet parses `text-overflow` and draws nothing
  for it; its line-breaking plan (`2026-10-02_livery_line_breaking_plan.md`) holds it as
  T4 behind S1.3.
- The browser's accessibility mirror shows rows and cells but no grid, no column header
  and no `aria-sort`: Genet's role lowering (`genet-render/src/a11y.rs`) has no entry for
  grid or column header and lowers a grid cell to a cell. Taproot has no role names for
  them either. Both are Genet's to change.


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

- **D40. The status line names the keys still sorting (Mark, 2026-11-19):** "Only keys still
  sorting". The line under a grouped table lists the grouping, then only the sort keys that
  order rows inside the groups; a saved sort key on a grouped field is not mentioned.

  Links: summary A grouped table's status line lists the grouping and only the sort keys still ordering rows.

  *Applies D39.*

- **D41. A row is keyed by its occurrence (Mark, 2026-11-19):** "Occurrence id". A table row
  keeps its node by the occurrence's id, through sort, filter and collapse; the plain grid
  follows.

  Links: summary A table row's node is keyed by its occurrence id, so it survives sort, filter and collapse.

  *Refines D33.*

- **D42. The plain grid takes keys through its own entry (Mark, 2026-11-19):** "Add
  data_grid_keyed". A caller with ids calls `data_grid_keyed` with a row-key closure;
  `data_grid` and `data_grid_with` keep index keys and their signatures.

  Links: summary A plain-grid caller passes row keys through data_grid_keyed, leaving data_grid unchanged.

  *Refines D41.*

- **D43. The unlanded retained-session work is taken over (Mark, 2026-11-19):** "eh,
  probably codex, maybe inactive or refocused on something else. you can take ownership of
  it". The uncommitted work in Genet's and Graphshell's main checkouts is brought forward
  and landed by this plan's session, which then carries the Genet steps of the adoption
  plan's ordered list.

  Links: summary The uncommitted retained-session work in Genet and Graphshell is landed by the table plan's session, which carries the Genet speed steps.

  *Contradicts D34, which it replaces.*

- **D44. One layout stamp (Mark, 2026-11-20):** "Keep SemanticStamp". The rescued
  `SemanticLayoutStamp` held the same seven fields; `SemanticStamp` is public, gains
  `matches(layout, dom)`, and serves both the native republish skip and the browser
  mirror's label-only patch.

  Links: summary SemanticStamp is the one layout stamp, public with matches, for the native republish skip and the browser label patch.

  *Applies D33.*

- **D45. A leaf or producer says when its meaning changed (Mark, 2026-11-20):** "Explicit
  hook". A custom leaf or producer raises a semantic revision when what it means changes
  with no DOM change; the host compares revisions before it skips an accessibility
  republish. Not built yet.

  Links: summary A leaf or producer raises a semantic revision when its meaning changes, and the host compares it before skipping a republish.

  *Refines D33.*

- **D46. The body scrolls by whole rows (Mark, 2026-11-20):** "Snap to whole rows". The
  scroll offset is always a row boundary and the body is as tall as the whole rows that
  fit; wheel travel short of a row is carried to the next turn.

  Links: summary The table body shows whole rows and scrolls by whole rows.

  *New.*

- **D47. The focus follows its row (Mark, 2026-11-20):** "Follow the row, scroll to it".
  Through a sort, a filter, a grouping or a collapse the focus stays on the same occurrence
  or heading, and the body scrolls the least that shows it; a row no longer shown leaves
  the focus at its position.

  Links: summary Through a change to the reading the focus stays on the same occurrence or heading and is scrolled into view.

  *Applies D41.*

- **D48. Restyle stops on a document-wide test (Mark, 2026-11-20):** "Document-wide flag".
  Genet's restyle does not descend from a changed element whose inherited and custom
  properties are unchanged, unless anything in the document explicitly inherits a
  non-inherited property; then it descends as before. Step 3 of the adoption plan's order.

  Links: summary Genet's restyle stops at an element whose inherited values are unchanged unless the document explicitly inherits a non-inherited property.

  *Applies D43.*

- **D49. Filter and grouping from the column header (Mark, 2026-11-20):** "Header menu per
  column", built "After the speed gate". Each header opens a menu: sort either way, group
  by the field, filter the field with the operators its kind takes; conditions join with
  and, and the status line says what is applied.

  Links: summary A column header's menu sets sort, grouping and a filter on its field; it is built after the speed gate.

  *New.*

- **D50. Restyle stops on the element's own children (Mark, 2026-11-20):** "Check the
  root's own children". Genet's UA sheet gives every table cell `vertical-align: inherit`
  and every row and row group `border-*-color: inherit`, so D48's document-wide test never
  stopped in a document holding an HTML table. The restyle stops at a changed element
  whose inherited and custom values are unchanged when none of its own children
  explicitly inherits a non-inherited property.

  Links: summary Genet's restyle stops at an element whose inherited values are unchanged when none of its own children explicitly inherits a non-inherited property.

  *Contradicts D48, which it replaces.*

- **D51. One retained route for a scroll batch (Mark, 2026-11-20):** "Merge steps 4 and
  5". Since D46 every scroll frame that changes anything moves the strip and swaps rows,
  so the adoption plan's inset-only step never applies to the table. Genet gets one route
  for the whole batch: translate the strip, lay out each arriving absolutely positioned
  row on its own, drop the leaving rows. Anything else falls back to the full layout.

  Links: summary Genet handles a scroll batch (strip moved, rows arrived and left) in one retained route; the adoption plan's steps 4 and 5 are one step.

  *Refines D46.*

- **D52. An unchanged frame records nothing (Mark, 2026-11-20):** "Record nothing". A
  frame whose accessibility publish is skipped as unchanged leaves the last publication
  observation standing; no skip variant and no counter.

  Links: summary A frame skipped as unchanged adds no accessibility publication record; the last one stands.

  *Applies D33.*

- **D53. Fragment insert and remove live in Buckram (Mark, 2026-11-20):** "Add insert and
  remove to Buckram". `FragmentTree` had push, translate, resize and replace only, and
  replacing the strip's subtree drops the strip's static-position record. Buckram gains
  `insert_subtree` and `remove_subtree`, additive, with its own tests.

  Links: summary Buckram's FragmentTree gains insert_subtree and remove_subtree for D51's route; nothing existing changes.

  *Applies D51.*

- **D54. Whole index rebuild first (Mark, 2026-11-20):** "Whole rebuild first, measure".
  The two primitives rebuild the document's fragment indices as `replace_subtree` does;
  incremental repair only if the measurement keeps layout above the 1 to 3 ms target.

  Links: summary Buckram's insert and remove rebuild the fragment indices whole; incremental repair waits on a measurement.

  *Applies D53.*

  The measurement came the same day: the index and overflow rebuilds were about 10% of the
  route, so the two primitives now repair the indices in place and overflow is recomputed
  once a batch.

- **D55. The route reads the style session's restyled set (Mark, 2026-11-20):** "Restyled
  set from the style session". The whole-plane clone, resolve, compare and drop were 45%
  of the route (5.2 of 11.3 ms on a 546-element fixture) for about ten restyled elements.
  `IncrementalStyle` reports the elements it re-cascaded; the route resolves and compares
  only those and patches the previous plane in place. A full-document restyle refuses the
  route.

  Links: summary The scroll-batch route resolves and compares only the elements the style session re-cascaded, and patches the plane in place.

  *Applies D51.*

- **D56. The box tree waits on a measurement (Mark, 2026-11-20):** "Decide after the other
  cuts". Regenerating and reconciling the box tree is 20% (2.3 ms); splicing boxes for the
  entering rows, or reusing ids while generating, is chosen after D55, D57 and D58 are
  measured on the table.

  Links: summary Whether the scroll-batch route stops rebuilding the box tree is decided after its other cuts are measured.

  *Applies D51.*

- **D57. Check first, then change in place (Mark, 2026-11-20):** "Check first, then change
  in place". The route no longer clones the layout for rollback (1.1 ms): every check that
  can refuse runs before the first mutation, and a debug assertion guards the rest.

  Links: summary The scroll-batch route runs every refusing check before its first mutation and changes the published layout in place.

  *Applies D51.*

- **D58. The text frame merges in place (Mark, 2026-11-20):** "Merge in place". Adding a
  row's shaped text copied the whole frame (1.3 ms); `TextFrame` removes the leaving
  sources and adds the entering ones without copying the rest.

  Links: summary TextFrame removes leaving sources and adds entering ones in place rather than copying the frame.

  *Applies D51.*

- **D59. Publish only the changed accessibility nodes (Mark, 2026-11-20):** "Publish only
  changed nodes". The publish rebuilds 526 nodes each scroll frame though about two rows
  change, 7.1 ms of a 21 ms frame: Genet's projection 3.2 ms, AccessKit's submit 1.7, two
  geometry walks 1.0, envelopes and native ids 0.9. Genet projects only the subtrees a
  batch touched, the host keeps the last tree and sends a partial update, and the strip
  carries a transform so a kept row's bounds do not change. Publishing only when a scroll
  settles was the option not taken.

  Links: summary The accessibility publish sends only the nodes a batch changed, with the row strip's offset carried as a transform.

  *New.*

- **D60. The box tree stays until the gate is closer (Mark, 2026-11-20):** "Leave it until
  the gate is closer". Layout is 3.3 ms of the frame; the accessibility publish, the paint
  list and the render worker are larger and go first.

  Links: summary The scroll-batch route keeps rebuilding the box tree until the larger frame costs are cut.

  *Refines D56.*

- **D61. No attribute skip; the selector tree is noted (Mark, 2026-11-20):** "Withdraw D61,
  note the selector tree". A skip for attribute writes no selector reads was ruled on a
  lane's claim of 210 re-cascaded nodes a frame, then withdrawn the same day: measured, a
  scroll frame restyles 23 or 45 elements, exactly the entering rows, and every attribute
  write lands on a row its insertion already cascades. The style pass does build its
  selector tree over 512 elements, the whole document, each frame; that is assessed after
  D59, with the box tree (D60).

  Links: summary The style session gets no attribute skip; its whole-document selector tree is a candidate cut after the accessibility publish.

  *New.*

- **D62. The retained projection is a session in Genet (Mark, 2026-11-20):** "A retained
  session in Genet". genet-render gains an accessibility session, like the style session:
  it takes the DOM, the layout and a batch's mutations, projects only the touched
  subtrees, and reports the nodes changed, added and removed. Rootstock and the winit
  adapter consume that change set; Pelt and Pictograph can too.

  Links: summary Genet owns a retained accessibility projection session that reports a per-batch change set.

  *Applies D59.*

- **D63. The session admits or refuses (Mark, 2026-11-20):** "Admit or refuse, like the
  scroll batch". A name can read other nodes, so the session patches only a batch it can
  prove local: subtrees inserted or removed that nothing references by id and that
  reference nothing outside, and attribute writes inside them. Any other batch rebuilds
  the whole projection. Tests compare each patched projection against a full one.

  Links: summary The accessibility session patches only batches it can prove local and otherwise rebuilds the whole projection.

  *Applies D59.*

- **D64. The partial update reaches AccessKit (Mark, 2026-11-20):** "All the way to
  AccessKit". The change set passes through Rootstock's composition and the adapter's id
  map, envelopes and routes, and AccessKit receives a partial tree update.

  Links: summary A batch's accessibility change set flows through Rootstock and the adapter to a partial AccessKit update.

  *Applies D59.*

- **D65. A generic container is named only from its own direct text (Mark, 2026-11-20):**
  "Name a generic only from its own direct text". Genet named every plain `div` from all
  its text, so the table's strip carried 30 rows' text (1,680 characters) and each
  entering row renamed two ancestors. Exact AccName 1.2 step 2F ("Follow AccName: generic
  containers get no name") was ruled first and reopened the same day: Genet projects no
  text nodes, so a `<p>`'s text reaches a reader only as its name, which is why Genet's
  names plan keeps its 15 October denylist "until real projected text nodes permit a full
  allowlist". So a generic keeps a name from content when it has a direct text child and
  gets none when it has only element children. Direct text is read in the flat tree and
  includes generated text ("Flat-tree children and generated text count"), as Genet names
  those today. Interim until text nodes are projected.

  Links: summary Until Genet projects text nodes, a generic container takes a name from content only when it has a direct text child.

  *New.*

- **D66. The change set is ids (Mark, 2026-11-20):** "Ids only: added, changed, removed". An update returns the ids of
  nodes added, changed and removed, or says it rebuilt; the host reads node data from the
  session by id. Equality with a full projection is by id, each node's child order exact.

  Links: summary The accessibility session reports a batch as added, changed and removed node ids, or as rebuilt.

  *Applies D62.*

- **D67. The session refuses on what the projection reads (Mark, 2026-11-20):** "Refuse on what the projection reads". It
  refuses when an inserted or removed id is named by any reference, dangling ones
  included; when an inserted reference resolves outside its subtree; when a control,
  label, heading or caption crosses a kept owner; under an `aria-hidden` ancestor; and at
  a shadow host or slot. The reference attribute names are one list exported by the name
  code. `aria-owns`, `aria-controls`, `aria-activedescendant` and `form=` are not read
  today and do not refuse.

  Links: summary The accessibility session refuses a batch only for dependencies the projection reads today.

  *Refines D63.*

- **D68. Writes on kept elements use the restyled set (Mark, 2026-11-20):** "Use the style session's restyled set". The session
  re-projects each written or restyled kept node and reports it changed only when its
  node data differs; a write to a reference attribute refuses. No list of inert
  attributes.

  Links: summary The accessibility session re-projects kept nodes named by the style session's restyled set and reports only real differences.

  *Applies D55.*

- **D69. Bounds refresh every update and report apart (Mark, 2026-11-20):** "Refresh all, report separately". The strip's
  `top` write moves about 500 nodes a frame; refreshing every node's bounds costs 0.04 ms.
  Bounds-only moves stay out of the changed list and are reported separately, so a host
  that owns bounds ignores them.

  Links: summary The accessibility session refreshes all bounds each update and reports bounds-only moves apart from changed nodes.

  *Applies D59.*

- **D70. Focus moves are node changes (Mark, 2026-11-20):** "As node changes". The update takes the focus; the old and new
  focused nodes appear as changed, and a removed focused node as removed.

  Links: summary The accessibility session reports a focus move as changes to the two nodes whose focused state differs.

  *Applies D62.*

- **D71. Every accessibility node is placed relative to its parent (Mark, 2026-11-20):**
  "Every node relative to its parent". Rootstock published bounds in window coordinates,
  so the strip's move rewrote about 510 nodes a frame. Each node now carries a translate
  from its accessibility parent and bounds at its own origin; a moved container changes
  one node, and no rule picks special elements. Final positions are unchanged.

  Links: summary Rootstock publishes each accessibility node as a translate from its parent with bounds at its own origin.

  *Refines D59.*

- **D72. Genet's AccessKit bridge keeps the whole tree (Mark, 2026-11-20):** "Change Genet's bridge to keep the tree". The bridge held
  one initial tree and the last update, so a partial update was right for a reader
  attaching later only if the caller re-sent the whole tree each publish. The bridge
  retains and patches the whole tree itself; the adapter sends only the partial update.
  The adapter cloning its tree each publish (0.23 ms) was the option not taken.

  Links: summary Genet's AccessKit bridge retains the whole tree and takes partial updates, so a late reader still gets the current tree.

  *Applies D64.*

- **D73. The structure lint runs on the whole retained tree (Mark, 2026-11-20):** "Lint the whole retained tree each publish". 0.42 ms
  a publish, about 2%; today's guarantee is kept exactly.

  Links: summary Every accessibility publish, partial or full, lints the adapter's whole retained tree.

  *Applies D64.*

- **D74. One geometry walk, offsets diffed (Mark, 2026-11-20):** "Merge into one walk and diff offsets". Rootstock walks
  the painted geometry once a publish instead of twice and compares each node's
  parent-relative offset with the retained one. Genet's moved list ignores Rootstock's
  scroll planes and is not used. Skipping the walk on a layout signal is left for later.

  Links: summary Rootstock finds moved accessibility nodes with one painted-geometry walk per publish, diffing parent-relative offsets.

  *Applies D71.*

- **D75. Parent-relative bounds everywhere, and the web host takes the change set (Mark, 2026-11-20):** "2, 3". Of three
  options he chose the second and third together: the shared composition becomes
  parent-relative and the web mirror sums translates ("Relative everywhere"), and the web
  mirror then consumes the session's change set as the native adapter does ("Move the web
  host onto the change set too"), in its own lane after the native path.

  Links: summary The shared composition is parent-relative for both hosts, and the web mirror moves onto the session's change set after the native path.

  *Refines D71.*

- **D76. The session lives while something consumes it (Mark, 2026-11-20):** "Only while something consumes it". Rootstock
  creates the session on the first publish for a native adapter or the web mirror, drops
  it when the adapter, owner or DOM is replaced, and collects mutations and restyled sets
  only meanwhile. Batches between publishes accumulate; a missing restyled set or a full
  layout in between rebuilds.

  Links: summary Rootstock keeps the accessibility session only while an adapter or the web mirror consumes it.

  *Applies D62.*

- **D77. Kept envelopes re-check their lives each publish (Mark, 2026-11-20):** "Re-check owner and mount each publish". A partial
  publish reuses the envelopes of unchanged nodes and re-checks their owner and mount
  lives every time.

  Links: summary A partial accessibility publish re-checks the owner and mount lives of every envelope it keeps.

  *Applies D64.*

- **D78. Genet projects grid roles and grid properties (Mark, 2026-11-22):** "Roles and the
  grid properties". Genet's neutral roles had Table, Row and Cell only: `role=grid` and
  `role=columnheader` fell to Unknown and no row count, index or sort was projected, so a
  reader heard neither a grid nor where a row sat among the table's 10,001. Genet adds
  Grid, TreeGrid, ColumnHeader, RowHeader and GridCell, with row and column counts,
  indexes, spans and sort direction, lowered to AccessKit. Roles alone was the option not
  taken.

  Links: summary Genet's accessibility projection carries grid roles and the row, column and sort properties a virtualized table needs.

  *New.*

- **D79. Graphshell's own producers write zero-based heading levels (Mark, 2026-11-24):**
  "Move uxtree and outline to zero-based". Genet's heading levels became zero-based under
  AccessKit's documented contract, while uxtree (`forme/uxtree/src/lib.rs`) and mere-view's
  outline (`gloss.rs`) still wrote an h1 as level 1, so one tree carried both conventions.
  The web mirror read the zero-based level straight into `aria-level`, publishing an h2 as
  level 1 (its `states_lower_to_their_aria_attributes` test failed on Graphshell main).
  Both producers now subtract one, and the mirror adds one when it writes `aria-level`. A
  separate follow-up and leaving them one-based were the options not taken.

  Links: summary Every AccessKit heading level in Graphshell is zero-based; the web mirror converts to ARIA's one-based aria-level.

  *Applies* the Genet ruling "Follow AccessKit's documented contract" to Graphshell's own producers.

- **D80. Find the unlogged frame time before the next gate cut (Mark, 2026-11-24):** "Find
  the unlogged 8 ms first". With the partial publish in, accessibility is about 3 ms of the
  slowest scroll frames, which average 24 to 25 ms. Style, layout, accessibility, emit and
  raster sum to about 16 ms, leaving about 8 ms the per-frame log does not name. The next
  step adds the frame's remaining stages (acquire, compose, present, frame hook) to that
  log and reads what fills the gap; measurement only. The selector tree and box tree
  (D60, D61) and emit and raster were the options not taken first.

  Links: summary The next scroll-gate step names the roughly 8 ms of each slow frame outside the logged stages before any cut.

  *New.*

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
- 2026-11-19: D40 and D41 written (staged, not committed). Under a grouping the
  status line names only the sort keys that order rows inside the groups. A table row
  is kept by its occurrence id (`GridBand::Id`, `GridRowKey::Id`), so a re-sort, a
  filter or a collapse moves its node and writes none of its text; the plain grid
  keeps the row index. A one-row scroll is still 83 mutations, the arriving row's
  construction. Tests: cambium 452, graphshell-client 73, cambium-rootstock run
  serially.
- 2026-11-19: D42 written (staged, not committed). `cambium::data_grid_keyed` takes
  a `row_key(row) -> GridRowKey`, so a plain grid whose caller knows each row's
  identity keeps a row's node through a reorder; `data_grid` and `data_grid_with`
  keep the row index and their signatures. No sibling repository changed. Tests:
  cambium 453.
- 2026-11-20: the rescued work landed (D43, D44); what it holds is in the incremental
  relayout adoption plan under "State on 2026-11-20".
- 2026-11-20: D46 and D47 written (staged, not committed), with two focus fixes: a
  pressed header takes the focus, and a focused heading row carries the row's focus
  class. The fixture's caption no longer counts its rows; the status line does. Genet's
  form-tree check runs once per batch (Genet 48e5524aa07, step 2 of the adoption plan's
  order). Tests: cambium 459.
- 2026-11-20: a focus moved from a frame hook, a timer or a view's focus request was never
  restyled: only the key, pointer and accessibility paths refreshed `:focus`, so the old
  node stayed painted as focused. The frame now refreshes it after its relayout
  (`focus_style_is_stale`). This was the missing highlight after Shift+Enter and on a
  clicked heading; the scenario driver dispatches keys from the frame hook.
- 2026-11-20: Genet's restyle stop landed (Genet 06c3630b189, D50). At 10,000 rows a
  scroll frame is 33.1 ms at the 95th percentile (was 51 to 67) and style in a frame where
  a row arrives is 1.2 ms (was 20 to 30); the 16.7 ms gate is not met, with layout
  (11.6 ms) and the accessibility publish (6.6 ms) left. Sort to shown is 84 to 102 ms.
  Numbers and what is next: the incremental relayout adoption plan.
- 2026-11-20: with whole-row scrolling (D46) no scroll frame only moves the strip: a
  changed frame carries the strip's `top`, one or two rows built and one or two removed
  (83 mutations). Genet's inset route admits one changed element and no added or removed
  node, and its local formatter refuses any positioned box
  (`supports_retained_root_formatting`), while every row and cell is absolutely
  positioned. Hence D51.
- 2026-11-20: the scroll-batch route (D51, D53) is written in Genet's `scroll-batch`
  worktree, not landed. On the table, same session, a busy machine, mean of 142 changed
  frames: layout 19.0 ms on Genet main against 8.8 ms on the route, which every changed
  frame took; the whole frame 46.9 against 35.2 ms; scroll at the 95th percentile 58.7
  against 40.6 ms (the quiet run that morning was 33.1 on main). The accessibility
  publish, 10.7 ms in these runs, is now the frame's largest cost. What is left in the
  route is whole-document work: D55 to D58
  (`testing/graphshell/frame-cost/scroll-batch/`, `scroll-batch-baseline/`).
- 2026-11-20: D55, D57 and D58 are built and the route is on Genet's main with Rootstock's
  caller. Same session, mean of 142 changed frames, every one taking the route: layout
  3.3 ms (19.0 on Genet main, 8.8 before these cuts), the whole frame 31.1 ms, scroll at
  the 95th percentile 36.5 ms. The captures match the full layout's to one part in 255,
  which two full-layout runs also differ by. The speed gate is not met: the accessibility
  publish is 11.4 ms of the frame in these runs, then the paint list at 4.9 ms. Open: the
  box tree (D56), about 2.3 of the route's remaining 5.3 ms on Genet's own fixture, and
  the style pass (see D61)
  (`testing/graphshell/frame-cost/scroll-batch-2/`).
- 2026-11-20: the accessibility publish by stage, mean of 160 scroll frames on a calmer
  machine (whole frame about 21 ms, publish 7.1): Genet's document projection 3.2 ms,
  AccessKit's submit 1.7, the two painted-geometry walks 0.6 and 0.4, native ids 0.5,
  envelopes 0.4, lowering 0.1; 526 nodes each frame. D59 to D61 follow
  (`testing/graphshell/frame-cost/a11y-stages/`).
- 2026-11-20: what a scroll batch restyles, counted on the table: 83 mutations (27
  insertions, 1 removal, 55 attribute writes, all on the entering row) restyle 23
  elements, 165 restyle 45; the selector tree covers 512 elements either way. A sort
  replaces every row: about 1,890 mutations, 507 to 516 restyled.
- 2026-11-20: Genet's projection by stage on a 525-node fixture shaped like the table
  (release, whole 5.97 ms): names 2.13 ms, descriptions 1.83, roles and states 0.55,
  bounds 0.04, the rest about 1. A description recomputes the whole name only to decide
  whether `title` may describe the node; checking for `title` first gives the same result
  and is being made. The strip's own name alone is 0.38 ms (D65).
- 2026-11-20: Genet's accessibility session is on its main (b7ef644b623). On the fixture
  a table scroll batch patches in 0.75 ms against 4.02 ms for a full projection (release,
  one run; later runs on a loaded machine kept the ratio near 0.19). Rootstock and the
  winit adapter do not use it yet (D64, D71).
- 2026-11-20: the native publish by stage on Genet b7ef644b623, a busy machine, means of
  142 changed frames, 528 nodes (publish 12.9 ms, p95 14.3): composition (Genet's
  projection, both walks, lowering) 11.2 ms, AccessKit's submit 4.27, envelopes 1.31, id
  remap 1.13, structure lint 0.42, the whole-tree clone in `submit_root` 0.23. A write to
  an attribute Rootstock reads itself (`data-cambium-text-value`, `data-key`, `id` and
  the like) never appears in the session's change set, so such a batch publishes whole
  (`testing/graphshell/frame-cost/a11y-host-baseline-b7ef/`, `a11y-host-probe-b7ef/`).
- 2026-11-24: the partial accessibility publish (D59 to D78) against its own parent
  af622d92, release, 10,000 rows, calm machine (no rustc, under 30% CPU), two alternating
  pairs. Publish p95 over scrolled frames 6.10 and 6.73 ms before, 3.01 and 3.17 after,
  142 of 418 frames publishing. Scroll p95 23.0 and 26.0 ms before, 23.1 and 24.3 after;
  the slowest 5% averaged 24.8 and 27.3 ms before, 24.1 and 25.2 after. Sort p95 117 and
  102 ms before, 119 and 135 after: the second pair's lane sorts were slower on all ten,
  unexplained, a follow-up (is the refused patch attempt costing a rebuild's worth?).
  Idle p95 unchanged near 5.2 ms. The gate still fails both (`testing/graphshell/
  frame-cost/ab-base-{1,2}`, `ab-lane-{1,2}`).
