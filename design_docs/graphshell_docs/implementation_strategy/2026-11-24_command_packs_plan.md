# Command packs plan

**Status:** 2026-11-24: in progress. Cut from GB6 P1 of `2026-10-28_graph_behaviors_plan.md`, whose
D1 to D6 place packs in the host toolkit's mod registry, run them through `lua-command` and set
their admission levels. D1 to D12 here settle the manifest, grants, context, first host, the
edges of admission, reloading, and Turnstone's targets and pack manager. CP1 and CP2 are met; CP3 is next.

**For:** Lua command packs: commands that install at runtime from a manifest, show in an app's
palette without a Rust enum variant, read a read-only context, and return action requests the host
checks against the pack's grants before it acts.

Not here: Wasm mod projects, the authoring pane and model-assisted authoring (GB6 P2 to P5, still
in the graph behaviors plan).

Receipts from this plan's runs go to `archive_docs/receipts/2026-11-24_command_packs_plan_receipts.md`,
never into this plan (DOC_POLICY §8).

## Constraints

- Home: the host toolkit's `mod_loader` registry (`host/crates/script/document-host`), not an app;
  every app's palette reads packs from it (`graph-behaviors D1`).
- Lane: a pack command is Lua run through `script-didge`'s `collect_invocations` over a host
  `CommandSet`; it returns hest `Invocation`s and never executes anything (`graph-behaviors D2`).
  The bounded run is didgeridoo's: fuel and a 16 MiB heap ceiling (`inker:lua-evaluator` D34, D36).
- Admission: conformant by default, dev mode by the user's choice per pack or folder; dev mode
  loosens admission and never grants; the level shows in the pack manager, not the palette
  (`graph-behaviors D3` to D6).
- Manifests are TOML, as the registry's `*.mod.toml` already are (data formats brief).

## Phases

### CP1. Manifest and admission

A `mod_type = "lua"` mod declares `command` contributions (id, label, effect class, the `.lua`
source file) and its grants (D1, D2). Conformant admission is all or nothing: an ABI number,
`deny_unknown_fields`, every file declared, no path escaping the pack, qualified unique ids, and a
named refusal for each failure (Mesocosm's rules, `graph-behaviors D3`). Dev mode loads what parses
and lists what conformant admission would refuse.

- Done when: a fixture pack admits conformant; each refusal kind has a test naming it; the same
  broken pack loads in dev mode with its diagnostics listed; the registry records each pack's level.

### CP2. Registration and run

Activation registers each declared command into the host's `CommandSet`, recorded with the
registry's `Action` extension record so a failed activation rolls back. Running a command builds the
`CommandContext` (D4), runs the source through `collect_invocations` under the pack's limits, and
checks every returned invocation: its id is granted, or its declared effect class is (D2, D3). A
refused invocation voids the batch, and the refusal names the id or class.

- Done when: headless tests in the host toolkit cover a granted run, a refusal for an ungranted id,
  a refusal for an ungranted class, a command the host adds later to a granted class being accepted
  (D3), an effect class that does not match the host's declaration being refused, and fuel and heap
  refusals.

### CP3. Lifecycle

Install from a directory at runtime, unload, and reload; a reload that fails admission or
activation leaves the last good version active.

- Done when: tests show a reload changing a command's behaviour without a restart, an unload
  removing its palette entries, and a failed reload rolling back.

### CP4. Turnstone

Turnstone's viewer command palette (`ports/graphshell/src/viewer_commands.rs`) lists pack commands
from the registry, and its pack manager, a Packs page in settings, shows each pack's level (D5,
D11, D12, `graph-behaviors D6`). Desktop only; the wasm32 build compiles without packs.

- Done when: a headed receipt installs a fixture pack, runs a command from the palette, shows a
  refused request with its reason, and reloads the pack to changed behaviour without a restart.

## Decisions

- **D1. A pack declares its commands in the existing `mod.toml` (Mark, 2026-11-24):** Evidence: the
  registry parses `*.mod.toml` with `toml::from_str` (`free_fns.rs:114`); it knows the Native and
  Wasm mod types and the Presentation, PaneAdapter, Pane and Servitor contributions, none a command.
  Offered: `mod_type = "lua"` and a `command` contribution in the existing manifest, activation and
  rollback reusing the `Action` record (recommended); a separate `*.pack.toml` read beside the
  registry. Answer: "In the existing mod.toml (Recommended)". What follows: one manifest format for
  every mod. Outcome: new.

  Links: area The stack · plans command-packs CP1 · summary A Lua command pack is a mod: lua mod type, command contributions, the existing mod.toml.

- **D2. Grants are classes or named commands, a ceiling and a floor (Mark, 2026-11-24):** Evidence:
  a pack's Lua returns hest `Invocation`s, requests and never permissions; hest's effect classes are
  Curation, DomainTruth and ExternalEffect; the registry's capabilities are coarse (Network,
  Filesystem, Identity, Clipboard, Exec). Offered: named host command ids approved at install
  (recommended); effect classes; both, ids naming and classes bounding. Answer: "Why can't ceilings
  also be floors? I can imagine enumerating every command (though an important and necessary
  capacity) will get annoying vs picking a class". What follows: a pack is granted classes, ids or
  both; a granted class admits every command in it and an id admits that command; anything outside
  is refused. Whether a command added later to a granted class rides the grant was put back as D3.
  Outcome: new.

  Links: area The stack · plans command-packs CP2 · summary A pack may be granted whole effect classes or single host commands; nothing outside them runs.

- **D3. A command added later to a granted class rides the grant (Mark, 2026-11-24):** Evidence: D2;
  the graph behaviors plan's lane constraint says a new capability never rides an existing
  preselected grant. Offered: a class grant is open-ended, the install dialog listing the class's
  current commands and saying later ones are included (recommended); a class grant is a snapshot,
  later commands prompting on first use; open-ended except ExternalEffect, granted by id only.
  Answer: "Rides the class grant (Recommended)". What follows: a class grant is explicitly
  open-ended and an id grant is not; the lane constraint is restated to say so. Outcome: refines
  D2.

  Links: area The stack · amends command-packs D2 · plans command-packs CP2 · summary A class grant covers whatever the host files under that class, now or later, and the install dialog says so.

- **D4. The context is selection and focus, versioned (Mark, 2026-11-24):** Evidence: GB6 P1 calls
  for a read-only `CommandContext` snapshot whose contents were unset. Offered: the selected node
  ids, the focused view's kind and target and the command's arguments, under a versioned schema
  hosts extend (recommended); a schema per app; decide while building. Answer: "Selection and
  focus, extensible (Recommended)". What follows: a pack written against one app's context runs in
  another that offers the same version. Outcome: new.

  Links: area The stack · plans command-packs CP2 · summary A pack command reads the selection, the focused view and its arguments, under a versioned schema.

- **D5. Turnstone proves it first (Mark, 2026-11-24):** Evidence: Turnstone's viewer commands
  already use hest's palette (`ports/graphshell/src/viewer_commands.rs`); Cambium's
  `command_surface` renders palettes for every app. Offered: Turnstone, with a headed receipt
  (recommended); a headless host first; Knot. Answer: "Turnstone (Recommended)". What follows: CP4.
  Outcome: new.

  Links: area The stack · plans command-packs CP4 · summary Command packs land first in Turnstone's viewer palette, proved by a headed receipt.

- **D6. An admitted Lua pack is Failed until CP2 registers its commands (Mark, 2026-11-24):**
  Evidence: CP1 admits a pack but cannot run it; `load_all` on one fails with "lua pack 'x' cannot
  activate before command registration". Offered: Failed with that reason (recommended); a status
  of its own between Discovered and Active; Active with no commands. Answer: "Failed with that
  reason (Recommended)". What follows: CP2 replaces the failure with registration. Outcome: new.

  Links: area The stack · plans command-packs CP1, command-packs CP2 · summary Until commands can register, an admitted Lua pack reports Failed with the reason.

- **D7. Dev mode still needs a TOML manifest that names its ABI and type (Mark, 2026-11-24):**
  Evidence: CP1's dev mode tolerates a wrong ABI, unknown keys, unqualified ids and undeclared
  files, and refuses a missing manifest, malformed TOML, or a missing `abi` or `mod_type` at both
  levels. Offered: refuse those at both levels (recommended); dev mode assumes the current ABI and
  lua, with diagnostics. Answer: "Refuse at both levels (Recommended)". What follows: a pack must
  say what it is, in TOML, whatever its level. Outcome: refines `graph-behaviors D5`.

  Links: area The stack · amends graph-behaviors D5 · plans command-packs CP1 · summary Dev mode loosens admission but still refuses a manifest that is not TOML or does not name its ABI and type.

- **D8. The runner is a crate of its own (Mark, 2026-11-24):** Evidence: `document-host`, home of the
  registry and CP1's admission, carries wasmtime 51, genet-scripted-dom, servitor and hest but not
  `script-didge`; in the host workspace only Scenograph takes `script-didge`, behind its optional
  `scripts` feature. Offered: a `command-packs` crate under `host/crates/script/` on the registry,
  hest and `script-didge` (recommended); a module beside `mod_loader`; the same behind a feature.
  Answer: "A new crate under host/crates/script/ (Recommended)". What follows: the registry stays
  engine-neutral and the Lua VM links only where packs run. Outcome: new.

  Links: area The stack · plans command-packs CP2 · summary The command pack runner is its own crate beside document-host, so only hosts that run packs link the Lua VM.

- **D9. A pack reloads when asked (Mark, 2026-11-24):** Evidence: CP2's runner reads a command's
  source at every run, so only a manifest change (commands, grants, labels) needs a reload; a
  failed reload keeps the last good version. Offered: an explicit reload of one pack or all, which
  hosts expose as a palette command and in the pack manager (recommended); watching pack folders;
  both, watching dev-mode packs only. Answer: "An explicit reload (Recommended)". What follows: no
  file watcher. Outcome: new.

  Links: area The stack · plans command-packs CP3 · summary A command pack is reloaded only when the host or the user asks.

- **D10. A conformant pack runs the sources it was admitted with (Mark, 2026-11-24):** Evidence:
  CP2 reads the source at each run, so an edited script ran without being checked again, even in a
  conformant pack. Offered: conformant packs pinned at admission, dev-mode packs live
  (recommended); both live; both pinned. Answer: "Pin conformant, live dev mode (Recommended)".
  What follows: a conformant pack's edit takes effect after a reload re-admits it; a dev-mode pack
  still reads its file at each run. Outcome: refines `graph-behaviors D3`.

  Links: area The stack · amends graph-behaviors D3 · plans command-packs CP3 · summary A conformant pack keeps the sources it was admitted with until reloaded; a dev-mode pack reads its files live.

- **D11. Packs run in native Turnstone first (Mark, 2026-11-24):** Evidence: Turnstone's viewer
  builds its palette from a hest `CommandSet` (`ports/graphshell/src/viewer_commands.rs:86`) and
  dispatches through the synchronous `command_invocation::invoke`, which a pack's invocations can
  feed; `document-host` links wasmtime and builds only natively, while the port also builds for
  wasm32. Offered: native first, the browser build compiling without packs behind a cfg
  (recommended); split admission and the runner from wasmtime and wire both now. Answer: "Native
  first (Recommended)". What follows: a wasm32 path is later work. Outcome: new.

  Links: area The stack · plans command-packs CP4 · summary Command packs run in desktop Turnstone first; the browser build compiles without them.

- **D12. Turnstone's pack manager is a settings page (Mark, 2026-11-24):** Evidence:
  `graph-behaviors D6` shows a pack's level where packs are managed, not in the palette; Turnstone
  had no pack manager. Offered: a pane listing packs, opened from the palette (recommended); a
  palette section; a settings page. Answer: "Settings page". What follows: a Packs page in
  Turnstone's settings lists each pack's name, level, commands, grants and last failed reload, with
  install, reload, unload and level actions. Outcome: new.

  Links: area The stack · plans command-packs CP4 · summary Turnstone manages command packs on a Packs page in its settings.

## Open questions

- The install dialog's wording for an open-ended class grant (D3).

## Findings

1. 2026-11-24, read: hest's `CommandSet::register` replaces a command with the same id, so commands
   can be registered at runtime (`crates/hest/src/menu/mod.rs:94-125`); `Invocation` is "a request
   to invoke a declared command, not permission to execute it" (`crates/hest/src/invocation.rs`);
   hest's `EffectClass` is Curation, DomainTruth or ExternalEffect (`crates/hest/src/lib.rs:35`).
   The registry's `ModExtensionRecord::Action` exists for rollback
   (`host/crates/script/document-host/src/mod_loader/loader/types.rs:93`). The host workspace already
   takes `script-didge` (`host/Cargo.toml:390`).
2. CP1, 2026-11-24 (`host/crates/script/document-host/src/mod_loader/loader/lua_pack.rs`): a pack is
   a directory with `mod.toml` (`abi = 1`, `mod_type = "lua"`, `files`, `[grants] classes/commands`,
   `[[contributes]] kind = "command"` with `id`, `label`, `effect`, `source`); a command id is
   `<mod_id>:<name>`. `lua_pack::admit(root, AdmissionLevel)` returns an `AdmittedPack` or one of
   fourteen named `PackRefusal`s; conformant stops at the first in a fixed order, dev mode loads
   what parses and lists the rest. Unknown keys are found by a key list checked against the parsed
   TOML, since one set of structs serves both levels. Paths are compared by components and
   canonical forms, so `..`, roots, drive prefixes and links out are refused on every platform.
   `ModRegistry::admit_lua_pack` records the level. `document-host`: 57 lib tests (19 new) and 19
   integration tests pass; clippy adds no warning.
3. 2026-11-24, read: `script-didge`'s `collect_invocations(source, offered, schema, budget, limits)`
   binds no globals, so a script cannot read a context; the effect class of each invocation comes
   from the host's `CommandSet`, never the script (`crates/script/didge/src/command.rs`, Inker
   `2ba0e45`). CP2 needs a variant that binds the context.
4. CP2, 2026-11-24. Inker `40a2d10` adds `collect_invocations_with_context`, which converts a
   `ScriptValue` into a `context` global in each call's own state, so a script's writes never reach
   the host. `host/crates/script/command-packs`: `PackRunner` (`run(command_id, &CommandContext)`,
   `PackLimits`), `CommandContext` (version 1: selection, focus, arguments, `extensions`) and
   `Refusal` (`UnknownCommand`, `SourceUnreadable`, `Script`, `NotGranted`). The class checked is
   the host's declaration, and an id the host no longer offers is refused. The registry starts Lua
   packs through an injected `LuaPackRuntime` (`ModRegistry::with_lua_runtime`) and records an
   `Action` per command for rollback, which uses hest's new `CommandSet::unregister`. A pack's
   limits are the runner's, not the manifest's, and its heap ceiling is `script-didge`'s 16 MiB.
   `command-packs` 13 tests, `script-didge` 70 and `document-host` 57 pass.

## Progress

- 2026-11-24: cut from GB6 P1; D1 to D5 ruled.
- 2026-11-24: CP1 met (Findings 2); D6 and D7 ruled.
- 2026-11-24: D8 ruled; CP2 started.
- 2026-11-24: D9 and D10 ruled; CP3 started.
- 2026-11-24: D11 and D12 ruled for CP4.
- 2026-11-24: CP2 met (Findings 4; graphshell `6a1b2f54`, Inker `40a2d10`).
