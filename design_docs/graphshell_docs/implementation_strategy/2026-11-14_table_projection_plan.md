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
