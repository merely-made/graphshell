# Mod projects plan

**Status:** 2026-11-25: in progress; MP0 and MP1 met. Cut from GB6 P2 to P5 and their tails in
`2026-10-28_graph_behaviors_plan.md`; P1 landed as `2026-11-24_command_packs_plan.md`. D1 to D11
settle building, admission, reload, `.cwasm`, the first host, fixtures, signing, network, the
authoring loop, the parser and viewer worlds, and the browser table. MP2 is next.

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

## Progress

- 2026-11-25: cut from GB6 P2 to P5 and their tails; D1 to D11 ruled.
- 2026-11-25: MP0 and MP1 met (Findings 2, 3).
