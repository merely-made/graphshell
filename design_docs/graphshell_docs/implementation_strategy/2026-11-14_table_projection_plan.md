# Table Projection Plan

**Status:** 2026-11-14: plan. Nothing started. Opened by `primitive-representations D18`
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
- A field is described in SHACL's terms: path, name, description, order, group. One term of
  ours says how it is shown. No validator is implied (D2).
- A fields list is authored as TOML and exchanged as JSON (workspace data-format rule).
- Sorting, filtering, grouping and the window are reading parameters. They never change the
  source (catalog, section 7).
- The table reads and does not write in this plan (D4).
- Each phase is verified in its own worktree before it touches main: its tests pass and the
  existing scene pins hold, or move under a ruling.

## Phases

### T1. Fields on the recipe
- Done when: a recipe carries an ordered list of fields, each with a path, name,
  description, order, group and how it is shown (the plain default for its type, a chip, a
  bar in the cell or a check mark, D6); a recipe without one compiles to the same
  scene as before; a field naming a path the dataset does not disclose is refused at compile
  with the field named; a fields list round-trips through TOML and through a SHACL node
  shape with no loss of the five SHACL terms.
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
  and one column header per field; scrolling, sorting and filtering stay smooth at 10,000
  rows, with the same measured and reported at 100,000 (D5); a headed check shows it in
  Chrome and Firefox.
- Needs: T1, T3.

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

- What is "smooth"? D5 sets the row counts; the frame time or response time that counts as
  smooth is not ruled, and is to be offered as a choice of values before T4's acceptance.
- What is the added term called in the fields list? Its values are ruled (D6); its name is
  not.

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

## Findings

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

## Progress

- 2026-11-14: plan written with Mark; nothing landed.
- 2026-11-14: row budget, ways of showing a field and the group and section names ruled (D5
  to D7).
