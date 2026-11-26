# Mod projects plan

**Status:** 2026-11-26: in progress; MP0 to MP8 met; MP7b and MP7c remain. Cut from GB6 P2 to P5
and their tails in `2026-10-28_graph_behaviors_plan.md`; P1 landed as
`2026-11-24_command_packs_plan.md`. D1 to D52 settle building, admission, reload, `.cwasm`, the
first host, fixtures, signing, network, the authoring loop, the parser and viewer worlds, the
browser table, grants, pins on reload, live pages and traps. MP5, MP5b (live pages), MP6 (the
authoring loop), MP7 (parser and viewer mods) and MP8 (the browser table) are met; MP7b and MP7c
(viewer panes) are next.

**For:** Wasm component mods a person (or any agent working in the same folder) can create, check,
build, attach to a document, exercise against fixtures and reload without a restart, holding no
authority the user did not grant; then parser and viewer mods on the same loop.

Not here: Lua command packs (their own plan); GB6's E2 non-blocking fiber suspension and
resolve-then-pin DNS defence, which stay parked in the graph behaviors plan.

Receipts from this plan's runs go to `archive_docs/receipts/2026-11-25_mod_projects_plan_receipts.md`,
never into this plan (DOC_POLICY §8).

## Constraints

- Home: the host toolkit's `mod_loader` registry and `DocumentScript`
  (`host/crates/script/document-host`); the first app is desktop Turnstone, beside command packs
  (D5, `command-packs` D11).
- Authority is linked, not trusted: a denied interface is never linked, so a component importing it
  fails to instantiate, and that failure names the import (Findings 1). Nothing a model or a mod
  writes widens a grant (`graph-behaviors D5`).
- Admission levels as for command packs: conformant pins at admission, dev mode reads live and
  never grants (D2, `graph-behaviors D3` to D5, `command-packs` D10).
- The WIT is Graphshell's `graphshell-endpoint/wit/world.wit`; `script/wit/world.wit` is a copy
  `build.rs` materialises (Findings 1).

## Phases

### MP0. No ambient authority

`DocumentScriptRuntime::activate` stops granting `Grant::allow_all()`: an activated mod links only
the interfaces its manifest declares and the user granted (D2).

- Done when: a test activates a mod whose manifest declares no network and shows its `net` import
  unlinked (instantiation refused, naming the import); every existing document-host test passes.

### MP1. Admission and the install record

Wasm mods admit at the two levels (D2). Conformant admission pins the component's hash; dev mode
reads the project live. An install record keeps id, version, source hash, build hash, declared and
granted capabilities, provided surfaces, and the last check, build and trap; no signing key yet
(D7). A `.cwasm` is loaded only when the host built it from an admitted component and its hash
matches the record (D4).

- Done when: tests cover a conformant admission pinning a hash and refusing a changed component
  until re-admitted, a dev-mode project reloading live, a foreign `.cwasm` refused, a host-built
  `.cwasm` loaded, and the record round-tripping.

### MP2. Check and build

The host owns `check`: load the component, compare its imports with its grants and its exports
with its world, and report each mismatch by name. Building is an optional adapter that shells out
to an external toolchain (`cargo build --target wasm32-wasip2`) when one is present; without it a
project uses prebuilt components (D1).

- Done when: check passes a matching component and names an ungranted import and a missing export;
  the build adapter builds a fixture project where the toolchain exists and reports its absence
  plainly where it does not.

### MP3. Attach, run, reload and fixtures

Attach a checked component to a document through `DocumentScript`; reload is detach then attach to
the same scripted DOM, keeping the document and losing instance state, and a reload that fails
check or instantiation leaves the old instance attached (D3). A fixture is TOML: a starting
document, a list of event turns, and the expected DOM and turn log; a mismatch shows the diff (D6).
Runs keep logs.

- Done when: tests show a reload changing behaviour on the same document, a failed reload keeping
  the old instance, a passing fixture, a failing fixture's diff, and a trap named in the log.

### MP4. Network tails

`NetFetcher`'s contract is uncredentialed: no cookies or credentials, ever, asserted by a test
fetcher. Manifest origins refuse wildcards broader than one registrable domain (D8).

- Done when: the contract test passes and `*.com` and `*` are refused while `*.example.com` admits.

### MP5. Turnstone

A Mod projects section beside Command packs in settings: create or open a project folder, check,
build where an adapter exists, attach to a document, run fixtures, reload, and read logs (D5).

- Done when: a headed receipt creates a project from a fixture, checks it, attaches it, runs a
  fixture to a pass, edits it, reloads to changed behaviour without a restart, and shows a refused
  import by name.

### MP5b. Mods on live pages

A mod attaches to a live Livery page in Turnstone, not only to its project's own document (D14,
D15): it runs on a copy of the page, mapped to the page node by node (D22), and each applied turn
is replayed into the page through `apply_host_mutations` by node identity, refused as
`PageMoved` when the page has changed since the copy (D24, D28). Genet widens `HostMutationOp`
with set-text, remove, insert-before and append-node (D23, D27); Graphshell's document-host emits
the replay; Turnstone attaches to the focused page and detaches on navigation (D25).

- Done when: a headed receipt attaches a mod to an open page, a turn changes what the page shows
  through the engine's restyle path, a turn against a page that changed underneath is refused as a
  conflict, and detaching leaves the page as the last applied turn made it.

### MP6. The authoring loop (GB6 P3)

The project folder is the protocol: any agent, or a person, edits files there, and the pane's
explicit Check, Build and Run show the diff since the last passing build, the logs and any trap,
with rollback to that build. The host calls no model and holds no model key (D9).

- Done when: a receipt edits a project from outside the app into a trap, shows the failing import
  or turn, rolls back to the last passing build, and runs again.

### MP7. Parser and viewer mods (GB6 P4)

`parser` (bytes or text in, blocks and diagnostics out) and `viewer` (a graph or document in, a
scene or lens contribution out) worlds in graphshell-endpoint's WIT, each with a fixture form like
MP3's, packaged and installed through the same section (D10).

- Done when: a fixture parser and a fixture viewer pass their fixtures and install from the
  section, and a page of the parser's declared media type opens in Turnstone parsed by the mod
  (D37).

### MP7b. A Mod viewer pane (step toward MP7c)

One built-in Mod viewer pane, opened from a viewer project's row, feeds the installed viewer the
live graph as `mere.graph-snapshot-and-facets/json` and paints the scene and sheet it returns
(D45). The feed and the painter are built for MP7c to reuse.

- Done when: the fixture viewer's regions paint in the pane from the live graph, a trap shows in
  the pane and the row, and a headed receipt shows both.

### MP7c. A pane kind per viewer

Pane definitions that can be added at runtime; each installed viewer appears as its own pane kind
with a palette entry, and the single Mod viewer pane retires (D45).

- Done when: installing a viewer adds its pane kind without a restart, uninstalling removes it, and
  the single pane is gone.

### MP8. What a browser can load (GB6 P5)

A table here of each mod kind (Lua pack, document-script, parser, viewer, `.cwasm`, native) against
each target (desktop, browser), and a test that the browser build refuses native-only kinds with a
stated reason (D11).

- Done when: the table is written and the refusal test passes in the wasm32 build.

## Decisions

- **D1. The host checks; building is an adapter (Mark, 2026-11-25):** Evidence: the host only loads
  components (`Component::from_file`); the one build in the tree is `document-host/build.rs`
  running `cargo build --target wasm32-wasip2` at crate build time; no cargo-component, wasm-tools
  or jco dependency exists. Offered: check in the host, build through an optional external-toolchain
  adapter (recommended); prebuilt components only; the host always shells out to build. Answer:
  "Check in the host, build by adapter (Recommended)". What follows: projects without a toolchain
  still load prebuilt components, and check can serve the browser later. Outcome: new.

  Links: area The stack · plans mod-projects MP2 · summary The host checks a component against its grants and world; building goes through an optional toolchain adapter.

- **D2. Wasm mods admit at the command packs' levels (Mark, 2026-11-25):** Evidence:
  `DocumentScriptRuntime::activate` grants `Grant::allow_all()` whatever the manifest says
  (`runtime.rs`, Findings 1); command packs admit conformant (pinned) or dev mode (live, never
  granting). Offered: the same levels, conformant pinning the component's hash, activation linking
  only granted interfaces (recommended); a separate Wasm admission; fixing `allow_all` without
  levels. Answer: "Same levels as packs (Recommended)". What follows: MP0 removes `allow_all`; MP1
  adds the levels. Outcome: applies `graph-behaviors D3`.

  Links: area The stack · plans mod-projects MP0, mod-projects MP1 · summary A Wasm mod is admitted conformant or in dev mode and links only the interfaces the user granted.

- **D3. Reload is detach and reattach (Mark, 2026-11-25):** Evidence: `DocumentScript` has attach,
  `deliver_event` and detach and no reload; the scripted DOM lives outside the instance. Offered:
  detach and reattach to the same scripted DOM (recommended); a swap method carrying revisions
  across. Answer: "Detach and reattach (Recommended)". What follows: the document survives a
  reload, instance state does not, and a failed reload keeps the old instance. Outcome: new.

  Links: area The stack · plans mod-projects MP3 · summary Reloading a Wasm mod detaches the old instance and attaches the new one to the same document.

- **D4. A `.cwasm` loads only when the host built it (Mark, 2026-11-25):** Evidence: the loader can
  load `.cwasm` through wasmtime's unsafe `deserialize_file`, which trusts the bytes; discovery
  matches only `*.wasm` (tail B4). Offered: host-built only, its hash recorded (recommended); won't
  do. Answer: "Host-built only, hash recorded (Recommended)". What follows: a foreign `.cwasm` is
  never deserialized. Outcome: new.

  Links: area The stack · plans mod-projects MP1 · summary Precompiled Wasm is loaded only when the host compiled it and its hash matches the install record.

- **D5. Mod projects live in desktop Turnstone beside command packs (Mark, 2026-11-25):** Evidence:
  the archived done-condition names "one pane" in Meerkat, which is gone; command packs landed in
  desktop Turnstone with a settings section. Offered: a Mod projects section beside packs
  (recommended); a Turnstone pane of its own; the host toolkit with no UI first. Answer: "Desktop
  Turnstone, beside packs (Recommended)". What follows: MP5. Outcome: new.

  Links: area The stack · plans mod-projects MP5 · summary Wasm mod projects are managed in Turnstone's settings beside command packs.

- **D6. A fixture pins the DOM and the turn log (Mark, 2026-11-25):** Evidence: `run_turns` and
  `host::TurnLog` exist and no fixture format does; command packs and Mesocosm pin exact outcomes.
  Offered: turns in, pinned DOM and log out, in TOML (recommended); turns in, selector assertions
  out. Answer: "Turns in, pinned DOM and log out (Recommended)". What follows: a mismatch shows the
  diff. Outcome: new.

  Links: area The stack · plans mod-projects MP3 · summary A Wasm mod fixture is a document and event turns in, the exact DOM and turn log expected out.

- **D7. Hashes now, signatures later (Mark, 2026-11-25):** Evidence: the archived install record
  names a signing key; no key store exists; conformant admission pins a hash (tail E3). Offered:
  hash pinning and approval at install, signing later (recommended); optional signatures now with a
  small key store. Answer: "Hash pinning now, signing later (Recommended)". What follows: the
  install record has no key field yet. Outcome: new.

  Links: area The stack · plans mod-projects MP1 · summary Wasm mods are pinned by hash and approved at install; signing waits for a key store.

- **D8. A mod's fetch never carries credentials (Mark, 2026-11-25):** Evidence: `NetFetcher` takes
  only a URL, so credentials depend on the embedder, and none ships; origins match exactly or by
  `*.suffix` (tails E1a, E1b). Offered: uncredentialed by contract, with origins no broader than one
  registrable domain (recommended); a credentials flag defaulting to none. Answer: "Uncredentialed
  by contract (Recommended)". What follows: MP4. Outcome: new.

  Links: area The stack · plans mod-projects MP4 · summary A Wasm mod's network fetch never sends cookies or credentials, and its origins name at most one registrable domain.

- **D9. Any agent authors through the project folder (Mark, 2026-11-25):** Evidence: the archived
  P3 has a model patch project files under a pane that checks, builds, runs and rolls back;
  Turnstone has no model client. Offered: any agent edits the folder and the pane's explicit
  actions show the diff, logs and traps, with no model keys in the host (recommended); Turnstone
  calls a model API; both, the client behind an adapter. Answer: "Any agent, through the project
  folder (Recommended)". What follows: a model's edits carry exactly a person's authority. Outcome:
  new.

  Links: area The stack · plans mod-projects MP6 · summary Models take part in mod authoring by editing the project folder; the host calls no model.

- **D10. Parser and viewer worlds are defined (Mark, 2026-11-25):** Evidence: the archived plan
  names both; neither exists; the WIT is graphshell-endpoint's, which has a `block-guest` world.
  Offered: define both, each with a fixture form (recommended); parser on `block-guest`, viewer new;
  parser first. Answer: "Define both worlds (Recommended)". What follows: MP7. Outcome: new.

  Links: area The stack · plans mod-projects MP7 · summary Parser and viewer mods each get a WIT world and a fixture form.

- **D11. The browser table comes with a refusal test (Mark, 2026-11-25):** Evidence: GB6 P5 asks
  to document what a browser cannot load at runtime. Offered: a table of mod kinds against targets
  and a test that the browser build refuses native-only kinds (recommended); the table only.
  Answer: "A table plus a refusal test (Recommended)". What follows: MP8. Outcome: new.

  Links: area The stack · plans mod-projects MP8 · summary What each target can load is tabulated, and the browser build refuses the rest with a reason.

- **D12. Reload readies the new instance before dropping the old (Mark, 2026-11-25):** Evidence:
  taken literally, D3's detach then attach cannot keep the old instance when the new one fails,
  because `attach` consumes the document; MP3 checks the new component, instantiates it, moves the
  document into it and activates it, handing the document back if activation traps, and only then
  drops the old instance; a guest's `activate` cannot change the document. Offered: keep the swap
  order (recommended); literal detach then attach, rebuilding the old instance on failure. Answer:
  "Keep the swap order (Recommended)". Outcome: refines D3.

  Links: area The stack · amends mod-projects D3 · plans mod-projects MP3 · summary A reload makes the new instance ready on the same document before the old one goes.

- **D13. The build adapter's test skips without a toolchain (Mark, 2026-11-25):** Evidence: MP2's
  adapter test failed on a machine without cargo or the wasm32-wasip2 target, while
  `document-host/build.rs` only warns when the target is missing. Offered: skip with a stated
  reason (recommended); fail loudly. Answer: "Skip with a stated reason (Recommended)". What
  follows: the absence-reporting test still runs everywhere. Outcome: new.

  Links: area The stack · plans mod-projects MP2 · summary The cargo adapter's build test skips, saying why, where the toolchain is absent.

- **D14. Mods attach to live pages (Mark, 2026-11-25):** Evidence: MP5 attaches a mod to its
  project's own document, parsed from `document.txt`, because a Turnstone page is genet's
  `LiveryDocumentSession`, which keeps its `ScriptedDom` private (`pub(crate) doc`), and
  `DocumentScript::attach` takes the DOM by value. Offered: the project's document now and pages
  as later pelt work (recommended); pages required; project documents only. Answer: "Pages are
  required". What follows: MP5b. Outcome: new.

  Links: area The stack · plans mod-projects MP5b · summary A mod in Turnstone must be able to run on a live page, not only on its project's own document.

- **D15. A page mod runs on a copy and its turns are replayed (Mark, 2026-11-25):** Evidence
  (Findings 7): the shipped way into a live page is `DocumentSession::apply_host_mutations`, whose
  four operations (set and remove an attribute, append a child, remove the last child) already
  restyle and repaint; `ScriptedDom` serializes (`outer_html`) and is not `Clone`. Offered: copy
  and replay, widening the operations in genet and gating on revision (recommended); genet lends
  the DOM out and takes it back; `DocumentScript` generic over a borrowed DOM. Answer: "Copy and
  replay (Recommended)". What follows: MP5b across genet, Graphshell and Turnstone. Outcome: new.

  Links: area The stack · plans mod-projects MP5b · summary A page mod runs on an outer_html copy, and each applied turn replays into the page through apply_host_mutations.

- **D16. MP5 lands on project documents; pages follow as MP5b (Mark, 2026-11-25):** Evidence: the
  project-document build passed its gates and headed receipt (Turnstone `40b4dbad`), and D17 to D19
  are small changes to it. Offered: land now with pages as MP5b (recommended); hold until pages
  work. Answer: "Land now, pages as MP5b (Recommended)". Outcome: new.

  Links: area The stack · plans mod-projects MP5, mod-projects MP5b · summary MP5 lands on project documents, and live pages are their own step before MP6.

- **D17. Grants are per-project toggles (Mark, 2026-11-25):** Evidence: MP5 granted nothing, so a
  mod importing `net` was always refused, with no control to change it. Offered: a toggle per
  declared capability on the project's row, kept in `installed.json` and enforced by linking
  (recommended); dev mode grants what it declares; nothing granted for now. Answer: "Per-project
  grant toggles (Recommended)". What follows: a changed toggle takes effect on the next attach or
  reload, at both levels. Outcome: new.

  Links: area The stack · plans mod-projects MP0, mod-projects MP5 · summary The user grants a mod's declared capabilities one toggle at a time, and linking enforces them.

- **D18. A reload's pin moves only when the swap succeeds (Mark, 2026-11-25):** Evidence: MP5
  re-admitted the edited component before swapping, so a swap that failed at activation left the
  old instance running under a pin naming the component that failed. Offered: pin after a
  successful swap, restoring the old admission otherwise (recommended); the pin follows the file on
  disk. Answer: "Only after the swap succeeds (Recommended)". Outcome: refines D12.

  Links: area The stack · amends mod-projects D12 · plans mod-projects MP5 · summary The pin always names the component that is running; a failed reload restores the old admission.

- **D19. A build overwrites the component and keeps the last passing one (Mark, 2026-11-25):**
  Evidence: MP5's build copies its artifact over `component.wasm`; MP6 (D9) rolls back to the last
  passing build. Offered: overwrite and keep `last-good.wasm` (recommended); overwrite only; a
  separate build slot chosen at attach. Answer: "Overwrite, keep last passing (Recommended)". What
  follows: MP6's rollback restores `last-good.wasm`. Outcome: new.

  Links: area The stack · plans mod-projects MP5, mod-projects MP6 · summary A build replaces component.wasm, and the last passing component stays beside it as last-good.wasm.

- **D20. A trapped turn detaches the mod (Mark, 2026-11-25):** Evidence: document-host's contract
  leaves the document unchanged on a trap and tells the caller to detach (`script.rs`,
  `deliver_event`). Offered: detach and keep the trap (recommended); restart on the same document
  up to a limit; detach and roll back to the last passing build. Answer: "Detach, keep the trap
  (Recommended)". Outcome: new.

  Links: area The stack · plans mod-projects MP5 · summary A turn that traps detaches the mod, the page keeps its last good state, and the trap is the project's last trap.

- **D21. A dev-mode mod can be granted (Mark, 2026-11-25):** Evidence: D2 carried the command packs'
  rule that dev mode never grants; D17's offered toggle said both levels work the same, and
  Turnstone `a12f2c44` built it so. Offered: dev mode takes grants, the row saying a grant reaches
  the unpinned component (recommended); dev mode never grants; a dev grant asks again after any
  change on disk. Answer: "Dev mode can be granted (Recommended)". What follows: the grants line
  says so when a dev-mode project holds a grant (Turnstone `6ab1aff7`). Outcome: refines D2.

  Links: area The stack · amends mod-projects D2 · plans mod-projects MP5 · summary A dev-mode mod takes the same grant toggles, and its row says a grant reaches whatever component is on disk.

- **D22. A replayed turn targets page nodes by identity (Mark, 2026-11-25):** Evidence (Findings
  9): `apply_host_mutations` finds its target by `id` attribute, first match winning, and most page
  elements carry none; a copy made with `outer_html` gets a fresh arena, so its node ids never equal
  the page's. Offered: a `Node(u64)` target beside `Id(String)`, resolved with `NodeId::from_raw`
  and `is_live`, the copy mapped to the page by a parallel walk and an attach refused when the
  shapes differ (recommended); stamp ids into the page; a hybrid. Answer: "Target nodes by identity
  (Recommended)". Outcome: refines D15.

  Links: area The stack · amends mod-projects D15 · plans mod-projects MP5b · summary Replay names page nodes by identity through a copy-to-page map, never by stamped ids.

- **D23. A mod edits text nodes themselves (Mark, 2026-11-25):** Evidence: the mod's `set-text`
  targets a `#text` node, and genet's page mutations reach elements only. Offered: real text-node
  targets, with remove and insert-before also taking text nodes and comments (recommended); set the
  parent element's text; elements only for now. Answer: "Real text-node targets (Recommended)".
  Outcome: new.

  Links: area The stack · plans mod-projects MP5b · summary Set-text, remove and insert-before reach text nodes and comments on a live page.

- **D24. Any DOM change but the mod's own is a conflict (Mark, 2026-11-25):** Evidence:
  `ScriptedDom::pending_mutations` gives a monotonic count of every DOM change, readable without a
  genet change; the structure-only epoch is private; whether image loads or hover add to the count
  is unmeasured. Offered: any change but the mod's own replays, measured first (recommended);
  structure changes only; only nodes the turn touches. Answer: "Any DOM change but the mod's own
  (Recommended)". Outcome: refines D15.

  Links: area The stack · amends mod-projects D15 · plans mod-projects MP5b · summary A page mod's turn is refused when the page's DOM changed since the copy, its own replays aside.

- **D25. A mod attaches to the focused page and detaches on navigation (Mark, 2026-11-25):**
  Evidence: Turnstone already finds the focused node's Livery session
  (`shell_focused_saved_reader_node.rs`), and the pelt controller's `session_identity` changes when
  the page is replaced. Offered: the focused page, detaching on navigate or close (recommended);
  pick from open pages; the focused page, re-copied on navigate. Answer: "Focused page, detach on
  navigate (Recommended)". What follows: the row says why it detached; the page keeps the last
  applied turn. Outcome: new.

  Links: area The stack · plans mod-projects MP5b · summary Attach takes the focused Livery page, and navigating or closing it detaches the mod.

- **D26. wasmparser 0.228 is skipped until wasmi is forked (Mark, 2026-11-25):** Evidence: genet
  `cbe8d3ed5d5` made the `scripted` feature pull `genet-bole/wasm`, so `genet-wasm`'s wasmi 2.0.3
  brings wasmparser 0.228 beside wasmtime 51's 0.263, and Graphshell's bans check refuses every
  fresh resolve; wasmi 2.0.3 is its latest release and wasmi's main still asks for 0.228. Offered:
  a `[[bans.skip]]` now, a patch-only wasmi fork onto the latest wasmparser as the way out
  (recommended); fork wasmi now; genet splits WebAssembly in pages into its own feature. Answer:
  "Skip now, fork wasmi later (Recommended)". What follows: the skip rows name the holder; the
  wasmi fork is open work outside this plan. Outcome: applies the one-version ruling's stopgap.

  Links: area The stack · plans mod-projects MP5b · summary A skip row carries the duplicate wasmparser until a wasmi fork reaches the latest.

- **D27. Genet appends any node to a live page (Mark, 2026-11-25):** Evidence: a mod's
  append-child of a text node had no genet operation to replay through; `AppendChild` builds
  elements only. Offered: an `AppendNode` taking any new node (recommended); refuse the turn by
  name. Answer: "Add AppendNode in genet (Recommended)". Outcome: refines D23.

  Links: area The stack · amends mod-projects D23 · plans mod-projects MP5b · summary AppendNode replays a mod's append of an element, text node or comment.

- **D28. A moved page is its own turn outcome (Mark, 2026-11-25):** Evidence: document-host could
  report a moved page as `Conflict(rev)`, the stale-revision outcome, or as a new variant. Offered:
  `TurnOutcome::PageMoved` (recommended); reuse `Conflict`. Answer: "Its own PageMoved
  (Recommended)". Outcome: refines D24.

  Links: area The stack · amends mod-projects D24 · plans mod-projects MP5b · summary A turn refused because the page changed reads PageMoved, apart from a stale revision.

- **D29. The diff is a line diff against a snapshot of the last pass (Mark, 2026-11-26):**
  Evidence (Findings 11): Turnstone keeps only `last-good.wasm`, a binary, and some projects carry
  no `src/`; `similar` is already in Turnstone's and Graphshell's graphs. Offered: snapshot the
  project's text files into `last-good/` at each pass and show a line diff with `similar`, plus
  whether the component's hash changed (recommended); changed file names only; a git repository
  per project. Answer: "Snapshot text, line diff (Recommended)". Outcome: new.

  Links: area The stack · plans mod-projects MP6 · summary A pass snapshots the project's text files, and the pane shows a line diff against that snapshot.

- **D30. Rollback restores the component only (Mark, 2026-11-26):** Evidence: D19 has rollback
  restore `last-good.wasm`; an agent's source edits may sit beside the broken component. Offered:
  the component only, the source left as edited and still in the diff (recommended); component and
  source; component, with a separate action for the files. Answer: "Component only
  (Recommended)". Outcome: refines D19.

  Links: area The stack · amends mod-projects D19 · plans mod-projects MP6 · summary Rollback puts last-good.wasm back over component.wasm and leaves every edited file as it is.

- **D31. A rollback reloads an attached mod through the pin (Mark, 2026-11-26):** Evidence: a
  reload moves the pin only after a successful swap (D18), and a page-attached project refuses
  reload. Offered: restore, then reload a document-attached mod the way Reload does, a
  page-attached one refused with "detach it first" (recommended); restore the file only; detach,
  then restore. Answer: "Reload through the pin (Recommended)". Outcome: new.

  Links: area The stack · plans mod-projects MP6 · summary Rollback reloads a document-attached mod as Reload does, and a page-attached one must detach first.

- **D32. A row notices outside edits on actions and pane open (Mark, 2026-11-26):** Evidence: D9
  makes Check, Build and Run explicit, and an agent edits the folder while the app runs. Offered:
  a hash scan when the pane opens and on each action, the row reading "changed since the last
  pass: N files" (recommended); a live file watcher; explicit actions only. Answer: "On actions
  and pane open (Recommended)". Outcome: refines D9.

  Links: area The stack · amends mod-projects D9 · plans mod-projects MP6 · summary A project's row counts files changed since the last pass, scanned on pane open and each action, with no watcher.

- **D33. Knot moves to similar 3 rather than Turnstone carrying a skip (Mark, 2026-11-26):**
  Evidence: D29's diff made `similar` a direct Turnstone dependency, resolving to 3.2.1, while
  knot-editor and knot-sync asked for 2.7, so the bans check refused the graph; knot-sync's one use
  (`djot_merge` `line_edits`) builds unchanged on 3. Offered: bump knot to similar 3 and land MP6
  without a skip (recommended); skip now and bump later; Turnstone asks for similar 2.x. Answer:
  "Bump knot to similar 3 (Recommended)". What follows: knot-editor `db15eb1`. Outcome: applies
  the one-version ruling.

  Links: area The stack · plans mod-projects MP6 · summary Knot's crates moved to similar 3 so the mod diff adds no duplicate.

- **D34. A viewer takes a graph and returns a scene and a sheet (Mark, 2026-11-26):** Evidence
  (Findings 13): "lens" in the code is a topology choice (`ProjectionLens`), a UI combinator or a
  window, none producing a scene; block-adapter's view returns a `sceno` scene and a
  `vector-sheet` the host paints; domain-authority takes a graph as
  `mere.graph-snapshot-and-facets/json`. Offered: the graph snapshot-and-facets payload in, a scene
  and sheet out, contributed as a Pane (recommended); a graph or a document in; a document only.
  Answer: "Graph in, scene+sheet out (Recommended)". Outcome: refines D10.

  Links: area The stack · amends mod-projects D10 · plans mod-projects MP7 · summary A viewer mod reads the graph snapshot with its facets and returns a scene and sheet the host paints in a pane.

- **D35. A viewer fixture asserts the scene's structure (Mark, 2026-11-26):** Evidence: today's
  fixtures are document-shaped (a document tree in, a DOM outline expected). Offered: the scene's
  named regions, their count and labels, and its size range (recommended); a hash of the payload
  bytes; a rendered image compared. Answer: "Scene structure (Recommended)". Outcome: new.

  Links: area The stack · plans mod-projects MP7 · summary A viewer fixture expects named regions and a size range, not bytes or pixels.

- **D36. A parser returns inker's EngineDocument as versioned CBOR (Mark, 2026-11-26):**
  Evidence (Findings 14): Fleece's blocks carry no serde and their anchors are minted only from a
  DOM; the reader lowers Fleece to `EngineDocument`, dropping anchors and table spans.
  `EngineDocument` is what every reader-style page renders from, is serde throughout, already
  crosses a process boundary as JSON from the smolweb packs, and its `Block` is non_exhaustive.
  Djot reaches it only through nematic's knot lowering, which drops footnotes and math. Mark first
  asked for a closer look at Fleece ("The question of blocks deserves more scrutiny even if we go
  with 2. Analyze 1 more"). Offered: `EngineDocument` as a version byte and CBOR, with WIT
  diagnostics, capped, strictly decoded, address, provenance and trust overwritten by the host
  (recommended); `EngineDocument` as JSON; Djot text; Fleece blocks. Answer: "EngineDocument,
  versioned CBOR (Recommended)". Outcome: refines D10.

  Links: area The stack · amends mod-projects D10 · plans mod-projects MP7 · summary A parser mod returns an EngineDocument as versioned CBOR, and the host fixes its address, provenance and trust.

- **D37. MP7 routes installed parsers into browsing (Mark, 2026-11-26):** Evidence (Findings 14):
  MP7's parser world, host and fixtures come to roughly 550 to 900 lines in Graphshell; routing adds
  roughly 600 to 1,200 in Turnstone (estimates), with forks on the registry, rule order against
  built-ins, a bytes body and the visible receipt. Mark asked to note the routing step and how much
  larger it was ("Note the latter step. How much larger?"). Offered: its own phase after MP7
  (recommended); inside MP7; after MP8. Answer: "Inside MP7". What follows: MP7's done-condition
  adds a routed page. Outcome: refines D10.

  Links: area The stack · amends mod-projects D10 · plans mod-projects MP7 · summary An installed parser mod handles pages of its declared media types within MP7, not as a later step.

- **D38. One dispatching engine serves every parser mod (Mark, 2026-11-26):** Evidence (Findings
  14): Turnstone's content-engine registry is built once at startup, keyed by engine id; the packs
  register every known id at startup and check availability at spawn. Offered: one "mod parser"
  engine registered at startup that finds the installed parser for the page's media type at spawn
  (recommended); a mutable registry with an engine per mod; register at the next startup. Answer:
  "One dispatching engine (Recommended)". What follows: installing a parser takes effect without a
  restart. Outcome: new.

  Links: area The stack · plans mod-projects MP7 · summary A single startup-registered engine dispatches each page to the installed parser for its media type.

- **D39. A parser mod takes unclaimed media types, and claimed ones on opt-in (Mark, 2026-11-26):**
  Evidence: built-ins claim `text/gemini`, `text/markdown` and HTML, and routing decides
  content-type first. Offered: unclaimed types by default, a claimed type only when the user turns
  it on in the project's row (recommended); built-ins always win; mods always win. Answer:
  "Unclaimed types; opt in for more (Recommended)". Outcome: new.

  Links: area The stack · plans mod-projects MP7 · summary A parser mod reads only media types no built-in claims unless the user hands it a claimed one.

- **D40. A parser reads bytes, carried through inker (Mark, 2026-11-26):** Evidence: the spawn
  request and engine input carry the body as a UTF-8 `String`; `FetchedDocument` holds the bytes.
  Offered: bytes and the media type in the parser world, with a bytes field added to inker's spawn
  request (recommended); text only for now; bytes in the world and fixtures, text when routed.
  Answer: "Bytes now, through inker (Recommended)". What follows: an inker change before
  Turnstone's routing. Outcome: refines D10.

  Links: area The stack · amends mod-projects D10 · plans mod-projects MP7 · summary A parser mod is handed the fetched bytes and their media type, through a new field in inker's spawn request.

- **D41. A mod-parsed page names its parser, and HTML capture skips it (Mark, 2026-11-26):**
  Evidence: no page says which parser made it; pack errors open a status page; the trail capture
  annotator parses every body as HTML. Offered: the parser named in the page's status line and
  inspector, an error page naming the mod on a trap or refusal, and HTML capture skipped for pages
  that are not HTML (recommended); name it on errors only; name it and leave capture alone. Answer:
  "Say the parser, skip HTML capture (Recommended)". Outcome: new.

  Links: area The stack · plans mod-projects MP7 · summary A page parsed by a mod says so, a failing mod names itself, and only HTML goes through HTML capture.

- **D42. The bytes ride genet's spawn request, not inker (Mark, 2026-11-26):** Evidence: D40 put
  the bytes field in inker's spawn request, but `SessionSpawnRequest` (`body: Option<String>`) is
  genet's (`document-session-api/src/session_engine.rs:100`) and inker only re-exports it; an
  additive `bytes` on inker's `EngineInput` (inker `803522b`, unpushed) never reaches a Turnstone
  session engine, and D38's engine calls the parser from the spawn request without an
  `EngineInput`. Offered: an additive `body_bytes` on genet's request, the inker change dropped
  (recommended); genet and inker both; a Turnstone table keyed by address; text only. Answer:
  "Genet spawn request, drop inker change (Recommended)". Outcome: refines D40.

  Links: area The stack · amends mod-projects D40 · plans mod-projects MP7 · summary The fetched bytes reach a parser mod through a body_bytes field on genet's SessionSpawnRequest; inker is unchanged.

- **D43. A mod-parsed page loads images from its own origin, others only with the grant (Mark,
  2026-11-26):** Evidence: a parser returns `Image` blocks whose URLs it chose, and Turnstone
  fetches inline images for parsed pages under the inline-media caps (8 images, 8 MiB encoded,
  64 MiB decoded), so a mod without the network grant (D17) could make the browser fetch any URL;
  Graphshell `e78fbfc0`'s host leaves them as given. Offered: the page's origin loads, other
  origins only when the mod holds the network grant, alt text otherwise (recommended); only with
  the grant; as the packs do. Answer: "Page's origin; others need the grant (Recommended)".
  Outcome: refines D17.

  Links: area The stack · amends mod-projects D17 · plans mod-projects MP7 · summary A mod-parsed page's images load from the page's own origin, and from elsewhere only under the mod's network grant.

- **D44. A parser's media types are declared in mod.toml and checked against the guest (Mark,
  2026-11-26):** Evidence: the parser world's `describe()` reports media types, and `mod.toml`
  declares them too, so the two can disagree. Offered: `mod.toml` declares them, and check refuses
  a component that reports different ones, naming both lists (recommended); the guest only;
  `mod.toml` only. Answer: "mod.toml, checked against the guest (Recommended)". Outcome: new.

  Links: area The stack · plans mod-projects MP7 · summary Routing reads a parser's media types from mod.toml, and check refuses a component that claims others.

- **D45. Each viewer is its own pane kind; one Mod viewer pane is the step there (Mark,
  2026-11-26):** Evidence (Findings 15, 16): nothing paints a viewer's scene and sheet; Turnstone's
  pane kinds are a static list of 18 in `src/panes/registry.rs`, so a kind is a compile-time edit;
  no Turnstone code paints a vector sheet (frozen-projection paints a sceno scene through cambium
  and netrender) or writes the graph-snapshot-and-facets JSON a viewer reads. Offered: one built-in
  Mod viewer pane opened from the project's row (recommended); a pane kind per viewer, which needs
  pane definitions that change at runtime; leave it for a later plan. Answer: "If 1 is an
  intermediary step to 2, i guess. But 1 is not a desired end state, you feel me?" What follows:
  MP7b builds the single pane, its feed and its painter for MP7c to reuse; MP7c gives each
  installed viewer its own pane kind and retires the single pane. Outcome: refines D34.

  Links: area The stack · amends mod-projects D34 · plans mod-projects MP7b · summary Every installed viewer becomes its own pane kind; a single Mod viewer pane comes first only as the step toward that.

- **D46. A wasmtime-free admission module answers per target, tested on wasm32 (Mark,
  2026-11-26):** Evidence (Findings 16): the browser build (`ports/graphshell/web`) carries no mod
  loader; `document-host` depends on wasmtime unconditionally and does not build for wasm32; no
  repository runs `wasm-bindgen-test`. Offered: a small wasmtime-free module in Graphshell's host
  answering whether a kind loads on a target and why not, with a test run on wasm32 under Node and
  the plan's table drawn from the same data (recommended); the same module tested natively with a
  wasm32 check only; `document-host` gated to build for wasm32. Answer: "Small admission module,
  wasm32 test (Recommended)". Outcome: refines D11.

  Links: area The stack · amends mod-projects D11 · plans mod-projects MP8 · summary One wasmtime-free table says which mod kinds each target loads, and its refusals are tested in a wasm32 build.

- **D47. The browser table states what loads today and marks what is planned (Mark,
  2026-11-26):** Evidence: no Wasm engine ships in the browser build; the guest-runtime P7 probe
  runs Pulley bytecode in Chromium; a `.cwasm` is machine code for the host's CPU (D4); a native mod
  is a function compiled into the binary; whether script-didge builds for wasm32 is unchecked.
  Offered: refuse document-script, parser, viewer and `.cwasm` with reasons, native as compiled-in
  only, Lua if script-didge builds for wasm32, Pulley-opened rows marked planned (recommended);
  refuse only `.cwasm` and native. Answer: "State today, mark planned rows (Recommended)".
  Outcome: new.

  Links: area The stack · plans mod-projects MP8 · summary The browser refuses every kind it cannot load today, with a reason, and the table marks what a browser engine would open.

- **D48. The browser's Lua row reads planned until a pack loader exists (Mark, 2026-11-26):**
  Evidence (Findings 17): script-didge (piccolo 0.3.3) passes `cargo check --target
  wasm32-unknown-unknown`, a type check that links and runs nothing, and the browser build has no
  pack loader. Offered: planned until a loader exists (recommended); allowed as written. Answer:
  "Planned until a loader exists (Recommended)". Outcome: applies D47.

  Links: area The stack · plans mod-projects MP8 · summary A Lua pack reads as planned in the browser until the browser build can load one.

- **D49. Turnstone calls the admission table when a target gets a mods surface (Mark,
  2026-11-26):** Evidence: on desktop every kind is allowed, so a call there shows nothing; the
  browser build has no mods UI and refuses by compiling `mod_projects`, `mod_parser` and
  `command_packs` out (`src/lib.rs:38-43`). Offered: not yet, wired with a loader (recommended); a
  browser-support line on each desktop row; a browser mods notice too. Answer: "Not yet; wire with
  a loader (Recommended)". What follows: MP8 closes on the crate, its wasm32 test and the table.
  Outcome: new.

  Links: area The stack · plans mod-projects MP8 · summary Turnstone asks mod-targets once a target first loads mods, not before.

- **D50. The sheet painter moves into its own Graphshell crate (Mark, 2026-11-26):** Evidence
  (Findings 18): the only `VectorSheet` painter is `SheetLeaf` (477 lines, Turnstone
  `ports/graphshell/src/block_editing/sheet_leaf.rs`), needing sprigging, vector-sheet and a
  cartography theme type, behind the graphshell crate's `editable-blocks` feature, which would pull
  web, illume, block-adapter, netrender and wgpu into Turnstone. Offered: a small crate beside
  vector-sheet in Graphshell's host, used by the block pane and the Mod viewer (recommended); a
  narrow feature in `ports/graphshell`; region labels now and the sheet later. Answer: "Own small
  crate in Graphshell (Recommended)". Outcome: new.

  Links: area The stack · plans mod-projects MP7b · summary One vector-sheet painter lives in its own Graphshell crate, shared by the block pane and viewer mods.

- **D51. The Mod viewer is a contributed surface (Mark, 2026-11-26):** Evidence (Findings 18): a
  built-in pane touches about twelve renderer arms plus a11y and input; a contributed pane
  (`SurfaceProvider`, as personae and distillery) already paints leaves, takes a per-pane source,
  dedups by kind and source, and registers providers at runtime, though each kind keeps one static
  registry row. Offered: one Mod viewer provider opened with the mod id as its source
  (recommended); a built-in renderer like frozen-projection. Answer: "Contributed SurfaceProvider
  (Recommended)". What follows: MP7c makes each viewer its own provider and kind. Outcome: new.

  Links: area The stack · plans mod-projects MP7b · summary The Mod viewer pane is a contributed surface opened with the mod id, the route MP7c extends to a kind per viewer.

- **D52. MP7b's smaller choices take the recommendations (Mark, 2026-11-26):** Evidence (Findings
  18). Answer: "yep, i'll accept the recommendations unless there are significant forks". Taken:
  the viewer re-runs when the graph's revision or the pane width changes, not every frame; the feed
  is the full snapshot-and-facets contract (`encode_candidate`'s shape); the width asked is the
  pane's logical width; mod-host's `first_line` reports a trap's cause rather than the backtrace
  header, for parsers, viewers and fixtures alike; a viewer's refusal reaches the row as a trap
  does; the fixture viewer reads a node's `title`, then `label`, and centres its items on their
  translate as block-adapter's `regions()` reads it. Outcome: new.

  Links: area The stack · plans mod-projects MP7b · summary The viewer re-runs on graph change, gets the full graph contract and the pane width, and its failures reach the row with their real cause.

## Open questions

None open.

## Findings

1. 2026-11-25, read (Graphshell `a2b4138a`; host paths under `host/crates/script/`):
   `DocumentScriptRuntime::activate` calls `grant_for`, which returns `Grant::allow_all()` for every
   manifest (`document-host/src/runtime.rs`); manifest capabilities are only a policy check. A
   denied interface is not linked, so a component importing it fails to instantiate. The host loads
   components with `Component::from_file` and compiles nothing at runtime; `document-host/build.rs`
   builds two fixture guests with `cargo build --target wasm32-wasip2`. `DocumentScript`
   (`document-host/src/script.rs`) has `attach`, `deliver_event`, `dom`, `revision` and `detach`,
   no reload, and no production caller. `discover_wasm_mods_in_dir` matches only `*.wasm`; its only
   callers are tests. `precompile_to_cwasm` and `load_component` exist, the latter deserializing
   unsafely. WIT `net.fetch` carries status; `NetFetcher` takes a URL only, and origins match
   exactly or by `*.suffix`. No `ModProject`, `RunSession`, fixture runner, signing or key store
   exists. The WIT is materialised from graphshell-endpoint's `wit/world.wit`, whose worlds include
   `document-core` and `block-guest` and no `parser` or `viewer`.
2. MP0, 2026-11-25 (graphshell `24399d06`): `DocumentScriptRuntime::new(allowed)` is the user's
   grant, empty by default; `grant_for` links `mere:script/net` only when the manifest declares
   `Network` and `allowed` holds it, `log` and `document` stay intrinsic, and the other
   capabilities gate no import yet. An undeclared import fails instantiation naming it; a declared
   but ungranted one is refused earlier, by policy, naming the same import. Two lifecycle tests now
   declare and grant `Network`, since the fixture guest imports `net`.
3. MP1, 2026-11-25 (graphshell `8a12d45d`, `wasm_admission.rs`): `admit_wasm` and
   `DocumentScriptRuntime::admit(manifest, version, path, level, readmit)`; conformant pins a
   BLAKE3 hash (`blake3:<hex>`) and refuses a changed component (`ComponentChanged`) until
   re-admitted, and activation refuses it too; dev mode pins nothing and records no grants.
   `InstallRecord` (id, version, level, source and build hashes, declared, granted, provided, last
   check, build and trap) is JSON; it lives in the runtime's memory, not on disk, and the registry is
   not wired to it yet. `build_cwasm` precompiles only a conformant, still-matching component and
   records the `.cwasm` hash; `load_component` hashes the bytes it read before deserializing those
   same bytes, so a `.cwasm` without a matching record is refused first; plain `attach` refuses a
   `.cwasm` and `attach_pinned` takes the record. `ModManifest` has no version field, so `admit`
   takes one. `document-host` tests pass (58 lib and every integration group); `command-packs` 23.
4. MP2, 2026-11-25 (graphshell `6aa6b6a8`): `check(path, &Grant, &World) -> CheckReport` lists
   imports, exports, ungranted and unknown imports and missing exports; `World::document_core()`
   and `World::requiring(export)`. `ComponentBuilder` with `CargoBuilder` (`with_cargo`,
   `with_target_dir`) runs `cargo build --target wasm32-wasip2 --release`, failing with
   `BuildError::{ToolchainAbsent, TargetMissing, BadProject, Failed, NoArtifact}`; the runtime's
   `check` and `build_project` write `last_check` and `last_build`.
5. MP3, 2026-11-25 (graphshell `5e94508a`): `DocumentScript::reload` runs check, instantiates the new
   component, moves the document and revision into it and activates it, and drops the old instance
   only after (D12); `ReloadError::{Check, Refused, Instantiate, Activate}`. A fixture is TOML: a
   `[document] tree` outline (two spaces a level, `#text` with a JSON string), `[[turn]] kind` and
   `payload`, and `[expect] dom` and `log` (`<n> <kind>: <outcome>`); `run_fixture` returns
   `Pass` or `Mismatch` with line diffs, and a trap ends the run and is written to `last_trap`.
6. MP4, 2026-11-25 (graphshell `50f855f2`): `NetFetcher` takes a URL, so a request can carry
   credentials only as userinfo; the host refuses such a URL before the backend runs
   (`url_carries_credentials`), and the trait's doc gives the embedder the duty of no cookie jar,
   auth header or session. `validate_origin` (on `psl` 2, already in the graph through netfetcher)
   refuses `*`, wildcards not leading with `*.`, single labels, wildcards on a public suffix
   (`*.com`, `*.co.uk`, `*.github.io`) and origins with a scheme, port, path or `@`; admission
   refuses a bad origin as `WasmRefusal::BadOrigin`.

7. MP5 forks, read 2026-11-25 (genet and Graphshell `main`): a Livery page is
   `LiveryDocumentSession { pub(crate) doc: LiveryDocument<ScriptedDom> }`
   (`genet-documents/src/engines/livery.rs`), reached from Turnstone through pelt-core's
   `session_as_any_ref`, which yields only `&ScriptedDom`. `apply_host_mutations` targets elements
   by `id` attribute with `HostMutationOp::{SetAttribute, RemoveAttribute, AppendChild,
   RemoveLastChild}` (`document-session-api/src/session_engine.rs`) and runs inside
   `LiveryDocument::mutate_dom`, which restyles incrementally and marks layout and paint dirty.
   `ScriptedDom` has `outer_html` (`genet-scripted-dom/serialize.rs`) and no `Clone`. Outside tests,
   no live page runs a `DocumentScript` today.

8. MP5, 2026-11-25 (Turnstone `40b4dbad`, `a12f2c44`, `6ab1aff7`): projects live under
   `<data_root>/mod-projects/<mod_id>/` (`mod.toml`, `component.wasm`, `document.txt`,
   `fixtures/*.toml`, and `Cargo.toml` with `src/` where it builds), with `installed.json` keeping
   each root, level and granted capabilities. The section sits after Command packs; refusals join
   the pack status view. A reload admits the new component with `admit_wasm` and moves the pin only
   when `script.reload` succeeds; `last-good.wasm` is written when check and every fixture pass.
   The headed receipt (`scenarios/mod_projects_mp5.scn`, seven frames in
   `Code/testing/turnstone/mod-projects/`) creates, checks, attaches, passes a fixture, reloads to
   changed behaviour, refuses `mere:script/net` by name, keeps the old instance answering, then
   grants net and reloads it in. The settings pane clips long rows at its width.

9. MP5b survey, 2026-11-25 (genet `main`): `NodeId` is a u64, the top 24 bits an arena and the
   low 40 a serial never reused, so `is_live` refuses a node removed since the copy. Document-host's
   WIT mutations already name raw `NodeId`s, text nodes included. `apply_host_mutations` resolves
   targets with `element_with_id`, a document-order walk, elements only. Genet branch
   `mp5b/host-mutation-ops` (`825707757fd`, unpushed) adds `Remove` and `InsertBefore`; a remove
   with no dependent selector repaints with `restyled_elements == 0`, so restyle counts only where
   a rule depended on the removed node. The only exhaustive match on `HostMutationOp` is the Livery
   impl.

10. MP5b, 2026-11-25 (genet `eea33cb1c71`, `ffd90dabf97`; Graphshell `4a940526`, `6b54afe9`;
   Turnstone `5ca294c6`): `HostMutation` targets `HostTarget::Id` or `HostTarget::Node(u64)`, and
   `HostMutationOp` adds `Remove`, `InsertBefore`, `SetText` and `AppendNode`. Document-host's
   `PageScript` copies the page through `inner_html`, maps copy to page by a parallel walk
   (attach refused when the shapes differ), replays each batch to the page before the copy, and
   subtracts its own replays from the page's mutation count; `attach_pinned` checks the install
   record as the document attach does (D4). Image loads, focus and typing do not advance the
   count. Turnstone's `mod-attach-page` attaches to the focused Livery page; `reconcile_mod_pages`
   detaches on close, reload or a replaced session and the row says why; a page-attached project
   refuses reload. Restyle per turn: a set-text restyled 1 element, an insert into the styled
   page 6, and a remove with no dependent selector 0. The headed receipt
   (`scenarios/mod_projects_mp5b.scn`, six frames in `Code/testing/turnstone/mod-projects-pages/`)
   sets text, inserts through a `p:empty` rule, refuses a turn as page moved after a second mod's
   change, keeps the last turn on detach, and detaches on reload. Following a link or `open` makes
   a new node, so reload is the only in-place replacement a scenario drives; close is covered by
   unit tests only. `wasmparser 0.228` carries a skip row in both repos (D26).

11. MP6 survey, 2026-11-26 (Turnstone `5ca294c6`): a project already keeps the last
   `LOG_TAIL` (6) guest log lines, its last trap, and `last-good.wasm`, written by `promote` when
   check and every fixture pass (or check alone with no fixtures). Nothing restores it, and nothing
   records the project's text at a pass. Check names a refused import (MP5's receipt refused
   `mere:script/net`). The `similar` diff crate resolves in both Turnstone's and Graphshell's
   graphs.

12. MP6, 2026-11-26 (Turnstone `263c9a8b`, `894cec4d`; knot-editor `db15eb1`):
   `src/mod_projects/authoring.rs` snapshots `mod.toml`, `Cargo.toml`, the manifest's document,
   `src/` and `fixtures/` into `last-good/` at each pass, line endings normalised, and diffs against
   it with `similar`. The row reads "Changed since the last pass: N files", whether
   `component.wasm` matches `last-good.wasm`, and up to ten diff lines with a count of the rest.
   The scan runs when the settings pane is summoned and after each action; the per-frame listing
   reads no disk. `mod-rollback` and a Roll back button (shown once `last-good.wasm` exists)
   restore the component, keep sources and the last trap, and reload through Reload's own path;
   a page attachment is refused with "detach it first". The headed receipt
   (`scenarios/mod_projects_mp6.scn`, nine frames in `Code/testing/turnstone/mod-projects-authoring/`)
   passes and attaches, takes an outside edit into a refused import (`mere:script/net` named in the
   status view) and shows its diff, rolls back and runs a turn on the same document, takes an
   outside edit into a component that traps on its first turn (fixture `alt/trap_turn.wasm`),
   detaches with the trap kept, rolls back, attaches and runs again. The diff is a capped list,
   not a collapsible one, and only the settings pane's summon triggers the open scan.

13. MP7 survey, 2026-11-26 (Graphshell `e91638e1`, Turnstone `894cec4d`): `ModType` is
   `Native`, `Wasm` or `Lua` (`document-host/src/mod_loader/loader/types.rs`), and Turnstone's
   `manifest.rs` sets `Wasm` for every project; a project is document-world only (attach, turn,
   check). `block-host` hosts `block-guest` with `bindgen!`, an epoch watchdog and store limits,
   tested in `block-host/tests/host.rs`; no world but the document world has a TOML fixture form
   (`document-host/src/fixture.rs`). Turnstone has no block type of its own: its reader and clip
   use Fleece's `Block` (serde under Fleece's `wire` feature); gemtext and Markdown are parsed by
   the smolweb pack engines into `SmolwebDocument`, routed by inker's `EngineRoutePolicy`, which
   `route_policy()` builds and nothing registers into at runtime. No WIT type carries a diagnostic;
   the nearest Rust one with severity and span is genet's `WebGlDiagnostic`. Lens types choose a
   topology or host a window and produce no scene. WIT variants cannot nest, so a block tree
   crosses as a payload or flattened.

14. MP7 parser blocks and routing, 2026-11-26 (inker `a25e408`, genet `9d57062`): genet's
   `components/fleece` is canonical for extraction (inker's `mere-fleece` re-exports it and adds
   clip and bridges). Fleece `Block` and `Inline` have no serde; its `wire` feature carries only
   canonical text with anchors; anchors are `TextAnchor`s over `FleeceDomTextV1`, minted from a
   `LayoutDom`. `lower_article` turns a Fleece article into `EngineDocument` and is anchor-blind.
   `EngineDocument` (`inker/src/document.rs:72`) is serde throughout, `Block` non_exhaustive; the
   smolweb packs return it as JSON under a 16 MiB reply cap and a 3 s timeout, and
   `SmolwebDocument::from_document_with_theme` renders it. A guest can set provenance, trust,
   image URLs and colours. `EngineRoutePolicy.rules` is a plain `Vec` on `Shell` and takes rules at
   runtime; `content_engines` is built once at startup, keyed by engine id; pack engines are reached
   only by a viewer override, never auto-routed. Spawn bodies are UTF-8 `String`s
   (`FetchedDocument` holds bytes); the trail capture annotator parses every body as HTML. Turnstone
   reads no `Presentation` contribution. `PackParser` (`src/engine_pack.rs`) is the in-process
   template for a session engine around a parser.

15. MP7, 2026-11-26 (genet `d2b4a55d3b5`, Graphshell `e78fbfc0`, Turnstone `5536ffe7`):
   Graphshell's `host/crates/script/mod-host` hosts the `parser-guest` and `viewer-guest` worlds on
   block-host's engine and watchdog, with 16 MiB caps on component, input and document, 1 MiB on a
   viewer's scene and sheet, strict CBOR decode, the host overwriting address, provenance and
   trust, and at most 256 diagnostics. In Turnstone, `mod.toml` carries `kind` and a parser's
   `media_types`; check compares the component with its kind's world and refuses a parser whose
   `describe()` reports other types, naming both lists (D44); fixtures run by kind. One engine,
   `turnstone.mod-parser` (`src/mod_parser.rs`), is registered at startup and finds the installed
   parser per media type at spawn, handing it `body_bytes` and engine id `mod:<id>` (D38, D42).
   Unclaimed types route to it; a claimed type only after "Hand over" on the row, kept in
   `installed.json` as `claimed` (D39). A type an installed parser takes is a document, not a
   download. The status line and an inspector row name the parser; a refusal or trap opens a page
   naming the mod, the type and the address; the trail capture annotator skips non-HTML bodies
   (D41). Other-origin images become alt text without the network grant (D43, unit tests only:
   the fixture guest emits no images). The headed receipt (`scenarios/mod_projects_mp7.scn`, 13
   frames in `Code/testing/turnstone/mod-projects-parsers/`) installs both fixture projects, passes
   their fixtures, and opens a parsed page, a refusal, a trap and a parsed page after the trap. A
   viewer installs and passes fixtures, and `view_graph` returns its scene and sheet, but nothing
   paints a viewer pane yet. A trap's message is mod-host's first error line, the backtrace
   header, which Turnstone replaces with "a wasm trap". If two projects declare one type, the first
   by id wins.

16. Viewer pane and MP8 survey, 2026-11-26 (Turnstone `5536ffe7`, Graphshell `e93a1c53`): pane
   kinds are `BUILTIN_PANES`, a static slice in `src/panes/registry.rs` (18 `kind::` ids), and
   `pane_definition` searches only it; `SurfaceProviderRegistry` is a runtime `Vec` keyed by kinds
   that already have a definition. `frozen_projection_pane.rs` paints a sceno scene through a
   cambium DOM scene and `netrender::Scene`; nothing in Turnstone paints a `VectorSheet` or writes
   `mere.graph-snapshot-and-facets/json` (named in Graphshell's authority-host, mod-host and
   cleromancy). The browser build is `ports/graphshell/web` (`graphshell-web`, its own workspace,
   `src/web.rs`), with no mod loader; Turnstone compiles `mod_projects` and `mod_parser` out on
   wasm32. `document-host` depends on wasmtime 51 unconditionally and has no wasm32 gating;
   nothing refuses a kind by target. No `wasm_bindgen_test` runs in Turnstone or Graphshell; the
   guest-runtime probes run wasm32 through JS harnesses. `NativeModRuntime` is an in-binary
   function table, not a dylib loader. script-didge's source is not checked out locally.

17. What each target loads, MP8, 2026-11-26 (Graphshell `b8e4037e` plus the `mod-targets` crate): the
   answer lives in `host/crates/script/mod-targets` (`Kind`, `Target`, `admit`, `refusal`), a crate with no
   wasmtime and no Lua VM, so the browser build can link it. Its `TABLE` is the data; this table is
   its `markdown()` pasted, and `tests/plan_table.rs` fails when the two differ. "Planned" does not
   load: `admit(..).loads()` is false and `refusal` words it "not yet loadable". The kinds line up with
   `ModType` (Native, Wasm, Lua) and Turnstone's `ModKind` (Document, Parser, Viewer) by name only; the
   crate depends on neither.

   | kind | desktop | browser |
   |---|---|---|
   | Lua pack | allowed | planned: script-didge builds for wasm32; the browser build has no pack loader yet |
   | document-script | allowed | planned: no Wasm engine in the browser build yet; Wasmtime on Pulley runs this guest in a page (guest-runtime P7) but is not built in |
   | parser | allowed | planned: no Wasm engine in the browser build yet; Wasmtime on Pulley runs this guest in a page (guest-runtime P7) but is not built in |
   | viewer | allowed | planned: no Wasm engine in the browser build yet; Wasmtime on Pulley runs this guest in a page (guest-runtime P7) but is not built in |
   | .cwasm | allowed: only when the host built it and its hash matches (D4) | refused: machine code built for the host's CPU |
   | native | allowed: compiled into that binary only | refused: a native mod is a function compiled into the binary; the browser build loads none at runtime |

   The Lua row reads planned (D48). A throwaway crate outside any workspace depending on
   script-didge (the inker `main` git row, `>=0.0.1`) passed `cargo check --target
   wasm32-unknown-unknown`, 40 s, resolving inker `6bd98cb3`, script-didge 0.0.1 and piccolo 0.3.3 from
   didgeridoo `7ed3b9fd`. That is a type check of the VM for wasm32, not a link or a run, and the
   browser build has no pack loader. The wasm32 test
   is `tests/wasm.rs` run under Node: `CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner
   cargo test -p mod-targets --target wasm32-unknown-unknown --test wasm` from `host/`. The runner is
   0.2.129 and the workspace pins wasm-bindgen `=0.2.129` (tabard-workshop), so the crate takes
   `wasm-bindgen-test >=0.3.79`, which resolves to 0.3.79; 0.3.80 needs wasm-bindgen 0.2.130 and
   cannot resolve here; the runner rejects cargo's `-q` (passed on as `--quiet`). Nothing calls
   `admit` yet: Turnstone's desktop and web builds do not link the crate (D49).

18. MP7b survey, 2026-11-26 (Turnstone `5536ffe7`, Graphshell `d746ca8f`): `VectorSheet`
   (`crates/vector-sheet`) is width, height and `Op`s (fill, stroke, text) painted in theme roles,
   capped at 1 MiB and 50,000 ops. Its one painter, `SheetLeaf`, is a sprigging leaf that fills and
   strokes non-zero paths and hands text runs to the host as DOM spans (`block_pane.rs`); netrender
   and mere never name the sheet. Scene and sheet are independent payloads; `FrozenScene::freeze`
   reads no regions and paints no leaves, while `ContributedSurfacePane` paints leaves through
   `scene_scrolled_with_leaves`. The feed's shape is `{"snapshot", "facets"}`, and Graphshell's
   `mere-resident` `encode_candidate` writes it from a `Graph` (a Turnstone dev-dependency only).
   A live snapshot's nodes carry `title`, not the `label` the fixture viewer reads. Panes render
   every frame; `Graph::revision()` exists to gate recomputation. A pane's parameter travels as its
   `PaneSpec` source (`Action::SummonContributedPane`); providers register at startup beside a
   static registry row. A viewer's trap reaches the row as mod-host's `first_line`, the backtrace
   header; a refusal reaches it not at all.

## Progress

- 2026-11-25: cut from GB6 P2 to P5 and their tails; D1 to D11 ruled.
- 2026-11-25: MP0 and MP1 met (Findings 2, 3).
- 2026-11-25: MP2 to MP4 met (Findings 4 to 6); D12 and D13 ruled.
- 2026-11-25: MP5's forks ruled, D14 to D20 (Findings 7); live pages split out as MP5b.
- 2026-11-25: MP5 met on project documents (Findings 8); D21 ruled. MP5b is next.
- 2026-11-25: MP5b surveyed (Findings 9); D22 to D25 ruled.
- 2026-11-25: genet's identity targets pushed (`eea33cb1c71`); D26 to D28 ruled.
- 2026-11-25: MP5b met (Findings 10). MP6 is next.
- 2026-11-26: MP6 surveyed (Findings 11); D29 to D32 ruled.
- 2026-11-26: MP6 met (Findings 12); D33 ruled. MP7 is next.
- 2026-11-26: MP7 surveyed (Findings 13); D34 and D35 ruled; the parser's blocks and reach are open.
- 2026-11-26: parser blocks and routing read (Findings 14); D36 and D37 ruled.
- 2026-11-26: MP7's routing forks ruled, D38 to D41.
- 2026-11-26: D42 ruled (the spawn request is genet's).
- 2026-11-26: genet `d2b4a55d3b5` (body bytes) and Graphshell `e78fbfc0` (parser and viewer worlds, mod-host) pushed; D43 and D44 ruled.
- 2026-11-26: MP7 met (Findings 15, Turnstone `5536ffe7`); the viewer pane is open. MP8 is next.
- 2026-11-26: viewer pane and MP8 surveyed (Findings 16); D45 to D47 ruled; MP7b and MP7c added.
- 2026-11-26: MP8 met (Findings 17); D48 and D49 ruled. MP7b is next.
- 2026-11-26: MP7b surveyed (Findings 18); D50 to D52 ruled.
