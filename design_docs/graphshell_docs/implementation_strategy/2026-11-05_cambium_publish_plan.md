# Cambium publish plan

**Status:** 2026-11-20: in progress. P0 is done. P1's manifest work is done as far as it can
go without a publish: the embeddings left rootstock (D6), the sparse crates carry their
one-hyphen names (D10), Genet's and Graphshell's chain crates are publishable with versions
on their git and path rows. What P1 still owes is listed under "Open before P2": four
questions for Mark and the dry runs, which can only run as a ladder. The API review P2 waited
on (D8) has its tiers 1 and 2 landed. P2's first rung is published: the vello fork's five
sparse crates and `netrender-vello` 0.11.1. Netrender's rung is dry-run and waits on Mark. The closure measured on 2026-11-05 is superseded: it is 54 crates,
and the Dramatis application this plan was opened for no longer exists.

**For:** publishing the Cambium desktop chain to crates.io so that applications outside Mere,
first the Dramatis application (source homes map, ownership cut 1), depend on registry
versions rather than `mere.git` pins. Opened by Mark's choice to publish the app's GUI
dependencies first and fall back to tracking `mere.git` main (D1).
Not here: the Dramatis application move itself (`dramatis/design_docs/`, the app plan after
its move); Genet's and Netrender's own release policy beyond the crates named below.

Receipts go to `archive_docs/receipts/2026-11-05_cambium_publish_plan_receipts.md`.

## Constraints

- Every published crate depends only on crates.io versions; a family that two repositories
  consume keeps one type identity (the cambium family resolves from one source per graph).
- Names are checked on crates.io at the time of the ruling and claimed by a real publish.
- Exact pins (`=0.1.2` and similar) move in every consumer manifest with each bump.
- No upstream contact without Mark's authorization (Netrender's vello notes: "upstream contact
  remains unauthorized").
- Security domains follow standards: nothing here touches crypto.

## Closure

Measured 2026-11-05 by `cargo metadata` over Mere, Genet and Armillary and the crates.io API
(subagent read, spot-checked). About 30 crates: the eight the app needs, their unpublished
dependencies, and roughly seventeen published crates whose manifests drifted since release.

- **New:** `apparatus-core` (Armillary); `genet-render`, `genet-presenter`, `genet-winit`,
  `genet-flow` (Genet); `cambium-rootstock`, `cambium-a11y`, `mandrake`, `cambium-desktop`,
  `persona-picker` (Mere); the renamed vello sparse crates (D5).
- **Republish (drifted):** `cambium` (chartulary ^0.3), `cambium-winit` (registry 0.3.0 wants
  `cambium ^0.3.3`), `buckram`, `livery`, `cadency`, `genet-livery`, `genet-parley`,
  `genet-fontique`, `genet-host-api`, `genet-scripted-dom`, `genet-static-dom`,
  `layout-dom-api`, `taproot`, `netrender`, `netrender_device`, `paint_list_render`.
- **Order:** Armillary → Netrender (with D5's crates) → Genet → Graphshell, after engine-split P3
  moves the Cambium family there (D7). Mere-resident crates in the closure publish from Mere.
- Source drift of crates "already published at matching deps" was not diffed.

## Phases

### P0. Rename `mesquite` to `mandrake` (D2, D3)
`host/crates/cambium/mandrake`, package and lib `mandrake`. The receipt schema id
`mesquite.commands/v1` is a wire name and stays.
- Done when: every Mere consumer checks on all targets (native and wasm32), `mandrake`'s tests
  pass, and Woodshed (root and its `host/ports` workspaces), Knot, Cleromancy, Isocosm, Turnstone and
  Murm's `apps/signalman-desktop` take the new name with their Mere pin bump. Knot, Isocosm,
  Cleromancy, Turnstone and Signalman also drop their `[patch.crates-io] accesskit` row, which
  pointed at Mere's removed patch, at the bump that reaches the remap.

### P1. Remove the blockers
- accesskit (D4): `cambium-a11y` remaps through accesskit's public `Node` API (8 vector and 7
  single NodeId properties, text selection, tree root, focus, action targets) with a fixture
  test setting every property and a serde key-set canary; drop `support/patches/accesskit` *(historical citation)* <!-- doc-audit: historical-path -->.
- vello (D5): publish the fork's sparse crates under Netrender names and point Netrender at them.
- iced/egui (D6): move `embedding-egui`/`embedding-iced` out of `cambium-rootstock` into a probe
  crate under `crates/probes` *(historical citation)* <!-- doc-audit: historical-path -->; Graphshell's `caller-owned-toolkit-bench` follows.
- Metadata: `publish = false` lifted where the closure requires it; Genet crates' inherited
  `repository` (servo/servo) corrected.
- Done when: each crate passes `cargo publish --dry-run` against registry-only dependencies
  in order, and the accesskit canary test fails when a property is left unmapped.

### P2. Publish in order
Waits on engine-split P3 (D7, met 2026-11-12): the Cambium family publishes once, from
Graphshell. Waits also on the composition plan's API review (D8). A dry run of a crate needs
its unpublished dependencies in the registry, so across repositories the dry runs can only run
as a ladder, each repository after the one below it has really published (Findings, 2026-11-20).
- Done when: every crate in the closure is on crates.io at the planned version, consumers' exact
  pins are updated, and the Dramatis application builds from registry versions alone.

### Open before P2

Found while finishing P1, 2026-11-20. None is decided.

- **`genet-presenter`'s `native-webgl` feature** takes optional dependencies on `genet-runtime`
  and `genet-webgl`, both unpublishable. Either they join the chain or the published crate
  goes without the feature.
- **`cambium-desktop` builds against a local patch of `light-file-dialog`**
  (`host/support/patches/light-file-dialog`: it "reuse[s] the Windows password class across
  cancellation and sequential prompts"). A published `cambium-desktop` resolves the registry's
  3.21.3. Whether that compiles and behaves is not determined.
- **`host/` patches `document-session-api` to Genet's git main**, to share one input vocabulary
  with Webcue. The published 0.1.2 may differ from main; if it does, it republishes first.
- **Version numbers.** About twenty published crates have drifted and need a bump, `cambium`
  0.6.1 and `cambium-winit` 0.3.0 among them. The numbers are chosen at the publish, and
  exact pins move in six consumer repositories with them.
- **The dry runs are a ladder.** `cargo package --list` passes for Genet's four new crates and
  Graphshell's six, but it resolves nothing. A real dry run of a crate needs its dependencies
  in the registry, so each repository's dry run follows the publish of the one below it: the
  vello fork, Netrender, Armillary, Genet, Graphshell's root, then `host/`.
- **Netrender's rung moves six repositories at once.** Genet, Graphshell's `host/`, Turnstone,
  Inker, Emblem and wgpu-cue take Netrender's crates from its main branch at `=0.1.2` and track
  no lock, so the push that carries 0.2.0 to Netrender's main breaks their resolve until their
  pins say `=0.2.0`. They keep the git row beside the version until the whole chain is on the
  registry; a graph that mixed a git Netrender with a registry one would hold two type
  identities.
- **The build wrapper refuses `cargo publish` and `cargo package`.** A dry run therefore runs
  as `publish --dry-run --no-verify` directly, which compiles nothing, followed by a wrapper
  build of each tarball. A real publish would run with `--no-verify` after that build, or the
  wrapper learns the two subcommands.
- **Inker and wgpu-cue** carry git-only rows (`webcue`, `grafting-frame`) that matter only if
  `uxtree` needs a newer `inker` than the registry's.

## Decisions

- **D1. Publish the app's GUI dependencies first (Mark, 2026-11-05):** "3, then 2", over
  tracking `mere.git` by rev or main from the start: publish, and track main for anything not
  yet publishable. Reaffirmed as "Full wave now" after the closure was shown at about eight
  crates, and pursued after it grew to about thirty with the blockers below examined.

  Links: summary The Dramatis app's GUI chain goes to crates.io; mere.git main is the interim.

- **D2. `mesquite` becomes `mandrake` (Mark, 2026-11-05):** "Mandrake", over `rhizotron`,
  `herbarium` and `burdock`; `mesquite` is taken on crates.io by an unrelated crate. Rhizotron
  is banked as a pane name. Outcome: new.

  Links: summary The scenario lane publishes as mandrake.

- **D3. Rename everywhere, consumers now (Mark, 2026-11-05):** "Rename everywhere", over a
  package-only rename; and "Now, in this wave" for Woodshed, Knot, Cleromancy and Isocosm,
  confirmed "Rename now regardless" when the wave's size was shown. Outcome: new.

  Links: summary Package, lib and imports take mandrake in Mere and its four consumer repositories now.

- **D4. Drop the accesskit patch (Mark, 2026-11-05):** "Drop patch, public-API remap", over a
  renamed fork and over keeping the patch. Put to Mark on finding every NodeId carrier public
  in accesskit 0.25.1 and no mapping API in 0.25.2. Outcome: new.

  Links: summary cambium-a11y remaps through accesskit's public API with an exhaustiveness canary; the local patch goes.

- **D5. Publish the vello sparse fork under Netrender names (Mark, 2026-11-05):** "Publish
  renamed sparse crates", over registry 0.3.0 with a pre-flight and over authorizing
  upstreaming. Put to Mark on finding Netrender's CPU path calling fork-only `FlattenError`
  refusal (`sessions.rs:345,352,553`, `webgl.rs:341`) and the fork's tolerance fix (`SQRT_TOL`
  0.5 → 1/16). Follows the `netrender-vello` precedent. Outcome: applies that precedent.

  Links: summary The fork's sparse vello crates publish under Netrender names, keeping exact curve refusal and the tolerance fix.

- **D6. Bench embeddings leave rootstock (Mark, 2026-11-05):** "Move to a probe crate", over
  dropping them and over pinning iced 0.14. The iced/egui embeddings serve only Graphshell's
  `caller-owned-toolkit-bench`; iced is an unreleased master rev. Outcome: new.

  Links: summary Toolkit-embedding benches live in a probe crate; published rootstock carries no git dependency.

- **D7. Publish after the move, from Graphshell (Mark, 2026-11-07):** "After P3, from
  Graphshell", over publishing from Mere now and over holding the wave. Put to Mark on finding
  that engine-split P3 moves the Cambium family, `mandrake` and `persona-picker` included, to the
  Graphshell repository. Consumers track git main meanwhile and repoint `mere.git` to
  `graphshell.git` at P3. Outcome: refines D1 (registry first, git main as the interim) and
  reorders the closure.

  Links: plans engine-split P3 · summary The Cambium chain publishes once, from Graphshell, after engine-split P3; consumers track git main until then.

- **D8. P1 now, the publish after the API review (Mark, 2026-11-20):** put to Mark on a
  survey finding the closure at 54 crates in eight repositories, the Dramatis application
  gone, and P1 unfinished (Findings, 2026-11-20). Four cards. "P1 now, publish later
  (Recommended)": finish the blockers and take every crate through a publish dry run in order;
  the real publish waits for the composition plan's P6 API review, so the API is not fixed on
  crates.io before it is reviewed, and Mark sees the list before anything is published; over
  "Full wave now", "Close the plan unpublished" and leaving it. Outcome: refines D1 and D7.

  Links: draws on cambium-publish D1, cambium-publish D7 · summary P1 is finished now; the publish waits on the composition plan's API review and on Mark seeing the list.

- **D9. The vello sparse crates take one-hyphen names (Mark, 2026-11-20):** the names recorded
  on 2026-11-06 (`netrender-vello-common`, `-cpu`, `-gpu`, `-gpu-shaders`) carry two hyphens and
  were not yet claimed. "Shorten them (Recommended)", over keeping them as recorded and over
  deciding at publish time; then, of three sets all free on crates.io that day, "sparse, cpu,
  hybrid, shaders": `netrender-sparse` (the common crate), `netrender-cpu`, `netrender-hybrid`
  (upstream's `vello_hybrid`) and `netrender-shaders`. Library names stay (`vello_common`,
  `vello_cpu`, `vello_hybrid`, `vello_sparse_shaders`); `netrender-glifo` and the published
  `netrender-vello` are untouched. Outcome: refines D5.

  Links: draws on cambium-publish D5 · summary The fork's sparse crates publish as netrender-sparse, netrender-cpu, netrender-hybrid and netrender-shaders.

- **D10. The GPU and shader crates are `netrender-gpu` and `netrender-wesl` (Mark, 2026-11-20):**
  put back to Mark the same day on two errors in what D9 was offered on. The GPU crate's
  library is `vello_gpu` in the fork and upstream, and "hybrid" is only Netrender's dependency
  alias; and `netrender-vello-shaders`, the classic Vello shaders, is already published, so
  `netrender-shaders` would sit beside it. "sparse, cpu, gpu, wesl (Recommended)":
  `netrender-sparse`, `netrender-cpu`, `netrender-gpu` and `netrender-wesl` (the shader
  sources are WESL), all free that day; over keeping D9's four and over changing only one of
  the two. Library names are `vello_common`, `vello_cpu`, `vello_gpu` and `vello_gpu_shaders`.
  Outcome: amends D9.

  Links: amends cambium-publish D9 · summary The sparse crates publish as netrender-sparse, netrender-cpu, netrender-gpu and netrender-wesl.

- **D11. Netrender's changed crates take minor bumps (Mark, 2026-11-20):** "Minor bumps", over
  patch bumps throughout. It was offered as 0.2.0 for `netrender`, `paint_list_render`,
  `paint_list_api` and `netrender_device`, with `netrender_text` at 0.1.2, on the size of the
  changes; the public APIs were not compared. `netrender_text` goes to 0.2.0 with the others:
  its functions take `netrender::Scene`, so a 0.1.x built on Netrender 0.2 would break callers
  under a compatible number. That correction is not Mark's; it goes to him with the publish
  list. The fork's `netrender-vello` goes to 0.11.1 first, because Netrender uses the
  diagnostics module that 0.11.0 was published without, and the change is additive.

  Links: summary Netrender's five crates publish at 0.2.0 after netrender-vello 0.11.1.

## Findings

- 2026-11-20, a survey of each repository's main (manifests read, nothing built; counts skip
  dev and optional dependencies, so feature-gated edges are approximate):
  - **The closure is 54 first-party crates** across Armillary, Netrender, the vello fork, Genet,
    Graphshell (root and `host/`), Inker, Eidetic, Mere and wgpu-cue. 29 are on crates.io, all
    owned by one account; 14 are new names, all free; about 20 published crates have a
    dependency set that differs from the registry's at the same version and need a bump.
  - **The first consumer is gone.** `dramatis/Cargo.toml` (lines 22 to 23): "There is no
    application here"; no Dramatis manifest names a cambium crate (cycle-breaks D26). Six
    repositories take the chain from `graphshell.git` main: Turnstone, Knot, Woodshed, Isocosm,
    Cleromancy and Murm's Signalman.
  - **P1 left undone.** D6: `embedding-egui` and `embedding-iced` are still features of
    `cambium-rootstock`, with `iced_wgpu` at a git rev. Genet's workspace still inherits
    `repository = servo/servo` and `publish = false` (`genet/Cargo.toml` 124, 129). Git-only
    rows with no version: `genet-render`, `genet-presenter`, `genet-winit` and
    `document-session-api` in `host/Cargo.toml`; `webcue` in Inker and `grafting-frame` in
    wgpu-cue; cambium's optional `tinct` is path-only.
  - **Local patches do not travel.** `cambium-desktop` builds against a `[patch.crates-io]`
    fork of `light-file-dialog`, and Genet's patch table redirects several crates to local
    copies; a published crate resolves the registry's versions instead.
  - **`persona-picker` is no longer a crate.** It is the cambium feature of that name.
  - **Forced order:** the vello fork, Netrender, Armillary (`apparatus-core`), Genet,
    wgpu-cue and Inker where a republish is needed, Graphshell's root, then `host/`
    (`cambium`, `cambium-winit`, `cambium-rootstock`, `cambium-a11y`, `mandrake`,
    `cambium-desktop`).
- 2026-11-05: `persona-picker` alone is publishable today: its name is free and `cambium`,
  `dramatis` and `personae` are published.

## Progress

- 2026-11-20: `netrender-vello` 0.11.1 published (Mark: "Publish 0.11.1"), from the fork at
  `8e76ca38`. Netrender's rung is dry-run and not pushed: with every vello row on the
  registry the workspace checks, the five crates package at 0.2.0, and each builds from its
  tarball. The pin moves are committed on local branches in the six repositories. Genet and
  Inker resolve against the local 0.2.0; Graphshell's `host/` and Turnstone cannot until Genet's
  main carries its move; Emblem and wgpu-cue track a lock, so theirs waits on the pushes and
  moves the lock with the pin.
- 2026-11-20: Netrender's rung prepared and not pushed (D11). The fork's `mark-ik/all-vellos`
  is `8e76ca38` with `netrender-vello` at 0.11.1; it packages and builds from its tarball, and
  is not published. On a Netrender branch the five crates say 0.2.0 and the sparse rows take
  registry versions; the workspace checks with `vello-all` and `serde`. Its own dry run waits
  on 0.11.1 reaching the registry.
- 2026-11-20: P2's first rung published (Mark: "Go ahead"). `netrender-sparse`,
  `netrender-cpu`, `netrender-wesl` and `netrender-gpu` 0.3.0 and `netrender-glifo` 0.4.0 are
  on crates.io from `mark-ik/all-vellos` at `b55d11f9`, and a crate outside the fork resolves
  all five from the registry. Netrender still takes them from the fork's branch; its rung
  moves those rows to registry versions.
- 2026-11-20: the ladder's first rung is dry-run and not published (Mark: "Dry run first"). The
  fork's five sparse crates package from `mark-ik/all-vellos` at `b55d11f9`, and each builds
  alone from its packaged tarball with its siblings patched in. All five names are free on
  crates.io under both spellings. The run found the four READMEs still linking upstream's
  crate pages (badges and documentation links); `b55d11f9` points them at the netrender names.
  Netrender's lock stays on `551cd5da`, which differs only in those READMEs.
- 2026-11-20: D10's rename landed. The fork's `mark-ik/all-vellos` is `551cd5da` and Netrender's
  main `1c375bd5d`, moved together; Netrender's workspace checked against the renamed crates
  first, with `vello-all` and `serde` (`--all-features` turns on a wasm-only feature that
  refuses a native build). Turnstone's deny reasons name `netrender-wesl` (`c8b749be`).
- 2026-11-20: Graphshell's host manifests (`aa1f31a4`): `cambium-rootstock`, `cambium-a11y`,
  `mandrake` and `cambium-desktop` are publishable; `genet-render`, `genet-presenter`,
  `genet-winit`, `document-session-api` and `tinct` carry versions on their rows; the `host`
  workspace still resolves. What remains is under "Open before P2".
- 2026-11-20: the D10 rename is prepared on the vello fork and not pushed (`551cd5da` on a
  local branch off `mark-ik/all-vellos`: nine files, package names and both lock files only;
  `cargo metadata --locked` passes in the root and in `fuzz/`). The fork's branch and
  Netrender's manifest have to move together, since Netrender takes the fork by branch and
  consumers track no lock; Netrender's side is not yet built.
- 2026-11-20: Genet's P1 metadata landed (Genet `e75a72ce34b`, manifests only, nothing built):
  the chain's crates name `merely-made/genet`, with the workspace default left at servo/servo
  for the vendored media crates; `genet-render`, `genet-presenter` and `genet-winit` are
  publishable; path dependencies between the chain's crates carry versions. Open for the
  publish list: `genet-presenter`'s `native-webgl` feature takes optional dependencies on
  `genet-runtime` and `genet-webgl`, both unpublishable; and Genet takes `netrender`,
  `paint_list_api` and `paint_list_render` by git with a version, so Netrender publishes first.
- 2026-11-20: survey taken and D8, D9 ruled. P1's remainder started: the embeddings move (D6)
  first, on Graphshell.
- 2026-11-20: D6 done (Graphshell `9163ad09`). The embedding module is the probe crate
  `cambium-embed` at `host/testing/cambium-embed`, a member of the `host` workspace; rootstock
  has no egui or iced dependency and no `embedding` features, and `cambium_rootstock::embedding`
  is gone. Nothing else used it; no `caller-owned-toolkit-bench` exists in any manifest. The API
  review the publish waits on will include its tier 2 (`cambium-composition D17`).
- 2026-11-06: P1's Vello item done (D5). The fork's sparse crates are `netrender-vello-common`,
  `-cpu`, `-gpu`, `-gpu-shaders` (0.3.0) and `netrender-glifo` (0.4.0; Mark, 2026-11-06,
  "netrender-glifo"), with library names unchanged, on `mark-ik/all-vellos` at `7784613` on top
  of `dfb9b53b` (Mark: "Branch tip dfb9b53b"). Netrender `ad4be200e` takes them; its lock is a
  pure rename. A combined `cargo publish --dry-run` packages and verifies all five against
  cargo's temporary registry. Netrender's `vello-all` tests pass except three Classic-path GPU
  tests that fail identically on the parent (`compose_into_01_identity_matches_render`, two
  `cast_slice` alignment panics); no Netrender test triggers `CurveFlattening`, which only the
  fork tests.
- 2026-11-06: P0 done. The rename reached every consumer in the same round that restored the
  family source convention (family brief, "Family source convention", upheld 2026-11-06):
  Woodshed 17ff090d, Knot 40ac979, Cleromancy 4df159b, Turnstone 5be7608c, Murm/Signalman
  1aebf9c1542, Isocosm 26306b02. Each also dropped its accesskit-from-Mere patch row. The
  `mesquite.commands/v1` wire id and Isocosm's `mesquite_byte_scope` receipt key stay.

- 2026-11-05: closure mapped; blockers examined; D1 to D6; P0 rename landed in Mere.
- 2026-11-05: accesskit remap (`cambium-winit-a11y/src/remap.rs`) replaces the patch: 28
  `cambium-a11y` tests pass, including a fixture over all 16 NodeId properties and a canary that
  reads AccessKit's own `Properties` schema; Mere and the standalone `native-id-consumer` probe
  resolve registry accesskit. The probe's dependency also had a stale package name, fixed.
