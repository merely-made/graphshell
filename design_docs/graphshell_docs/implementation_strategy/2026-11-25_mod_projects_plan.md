# Mod projects plan

**Status:** 2026-11-25: in progress; MP0 to MP5b met. Cut from GB6 P2 to P5 and their tails in
`2026-10-28_graph_behaviors_plan.md`; P1 landed as `2026-11-24_command_packs_plan.md`. D1 to D28
settle building, admission, reload, `.cwasm`, the first host, fixtures, signing, network, the
authoring loop, the parser and viewer worlds, the browser table, grants, pins on reload, live
pages and traps. MP5 and MP5b (live pages) are met;
MP6 (the authoring loop) is next.

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
  section.

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

## Open questions

- MP7's world shapes: the parser's block and diagnostic types, and the viewer's scene or lens
  contribution, are read from graphshell-endpoint's existing types when MP7 starts.

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

## Progress

- 2026-11-25: cut from GB6 P2 to P5 and their tails; D1 to D11 ruled.
- 2026-11-25: MP0 and MP1 met (Findings 2, 3).
- 2026-11-25: MP2 to MP4 met (Findings 4 to 6); D12 and D13 ruled.
- 2026-11-25: MP5's forks ruled, D14 to D20 (Findings 7); live pages split out as MP5b.
- 2026-11-25: MP5 met on project documents (Findings 8); D21 ruled. MP5b is next.
- 2026-11-25: MP5b surveyed (Findings 9); D22 to D25 ruled.
- 2026-11-25: genet's identity targets pushed (`eea33cb1c71`); D26 to D28 ruled.
- 2026-11-25: MP5b met (Findings 10). MP6 is next.
