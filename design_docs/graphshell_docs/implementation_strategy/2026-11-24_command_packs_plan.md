# Command packs plan

**Status:** 2026-11-24: plan. Cut from GB6 P1 of `2026-10-28_graph_behaviors_plan.md`, whose D1 to D6
place packs in the host toolkit's mod registry, run them through `lua-command` and set their
admission levels. D1 to D5 here settle the manifest, grants, context and first host. Nothing is
built.

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
from the registry, and its pack manager shows each pack's level (D5, `graph-behaviors D6`).

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

## Open questions

- Where the runner lives: a module beside `mod_loader` in `document-host`, or a crate of its own
  under `host/crates/script/`. Decided when CP2 starts.
- The install dialog's wording for an open-ended class grant (D3).

## Findings

1. 2026-11-24, read: hest's `CommandSet::register` replaces a command with the same id, so commands
   can be registered at runtime (`crates/hest/src/menu/mod.rs:94-125`); `Invocation` is "a request
   to invoke a declared command, not permission to execute it" (`crates/hest/src/invocation.rs`);
   hest's `EffectClass` is Curation, DomainTruth or ExternalEffect (`crates/hest/src/lib.rs:35`).
   The registry's `ModExtensionRecord::Action` exists for rollback
   (`host/crates/script/document-host/src/mod_loader/loader/types.rs:93`). The host workspace already
   takes `script-didge` (`host/Cargo.toml:390`).

## Progress

- 2026-11-24: cut from GB6 P1; D1 to D5 ruled.
