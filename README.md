# Graphshell

Graphshell is the host layer of the Mere family as a library: the contract between
an application that shows other applications (a host) and an application that can
be shown (a guest), and above it the toolkit a host is built from. Mere is the
engine underneath; applications such as Turnstone sit on top.

The repository has two Cargo workspace roots, each with its own lock.

## The contract root (`crates/`)

The bottom of the stack. Nothing in it depends on Mere, in normal, build or dev
dependencies; `grep -c 'merely-made/mere' Cargo.lock` prints 0.

| Crate | Role |
|---|---|
| `chirograph` | The projection contract: versioned, carrier-neutral messages by which an endpoint offers a scene, a resource or an action and a host asks for one |
| `sceno` | Scene vocabulary: source references, spaces, footprints, persisted scores, scene instances |
| `sceno-backdrops` | Backdrop scenes (metre, fretboard, terrain, templated sequences) compiled to `sceno` scenes |
| `scenotime` | Scene runtime: epochs, idempotent diffs, host-clocked transitions, picking |
| `hest` | Shared command menus, palette sessions and command ids |
| `graphshell-endpoint` | The traits an application implements beside its own truth, the `stdio`, `local` and `network` carriers, and the WIT projection session |
| `graphshell-client` | Transport-independent client state: snapshots, diffs, resume, resource cache |
| `script-wire` | wRPC value encoding and blocking transports for the WIT bindings |
| `script-witgen` | Generates Rust bindings for a WIT interface over `script-wire`; a build dependency |
| `tinct` | Perceptual (OKLCH) seed-to-palette derivation and a contrast-gated syntax palette |
| `vector-sheet` | The versioned vector sheet a guest block returns for its host to paint |

The WIT world is `crates/graphshell-endpoint/wit/world.wit` (package
`mere:script@0.1.0`). It is the only copy; `graphshell-endpoint` publishes its
directory to dependents' build scripts through Cargo `links` metadata
(`DEP_GRAPHSHELL_ENDPOINT_WIT`).

## The host root (`host/`)

The host toolkit and the resident assembly. The crates keep the subpaths they
had in Mere. The root names the contract crates by path (`../crates/<name>`), the
engine's five crates (`mere-kernel`, `mere-rdf`, `catchment`, `pandect`,
`mere-facts`) from mere.git, `pictograph` and `emblem` from emblem.git, and
Distillery's crates from distillery.git; nothing else comes from mere.git.

| Path | Crates |
|---|---|
| `crates/cambium/` | `cambium`, `meristem`, `sprigging`, `backstep`, `workbench`, `mandrake`, `cambium-rootstock`, `cambium-desktop`, `cambium-web`, `cambium-winit`, `cambium-a11y`, `mere-view`, `mere-overmap`; `scenes/scenograph` and `scenes/scenomise` |
| `crates/canvas/cartography` | `mere-cartography`, the canvas, with the backdrop studies it ships (`backdrops/`) |
| `crates/forme/uxtree` | `uxtree` |
| `crates/graphshell/` | `graphshell-door`, `graphshell-host`, `graphshell-client-tests` |
| `crates/incipit`, `crates/platen/platen` | `incipit`, `platen` |
| `crates/script/` | `app-host`, `authority-host`, `block-host`, `block-adapter`, `document-host`: the component hosts over the WIT world; their test guests build for `wasm32-wasip2` |
| `crates/system/` | `djinn-testkit`, `mere-fetch`, `mere-registry`, `settings-wit`, `mere-surface-api`, `tabard`, `ux-events` |
| `ports/mere-resident` | `mere-resident`, the resident assembly |
| `ports/moot` | `mere-moot` |
| `ports/pelt/core`, `ports/pelt/desktop`, `ports/tabard/workshop` | `pelt-core`, `pelt-desktop`, `tabard-workshop` |

Beside the members: `support/patches/` holds the patched `light-file-dialog` and
`cubek-matmul` that the root's `[patch.crates-io]` table points at,
`support/run_resident_*.ps1` drive resident receipt runs, and
`testing/moot-ocapn-coop` is a standalone check of `mere-moot`'s co-op sources
with its own lock.

For other repositories' tests, `block-host` exposes its fixture guest under its
`test-support` feature (`block_host::test_support::fixture_guest_wasm()`), and
`mere-resident` exports the identity-cards projection under `test-fixtures`.

## Depending on it

Family repositories name these crates by git, on `main`, with no revision pin:

```toml
cambium = { git = "https://github.com/merely-made/graphshell.git", branch = "main" }
```

Cargo finds a package by its name anywhere in the repository, in either root;
the consumer's lock records the revision.

## Building

The toolchain is pinned in `rust-toolchain.toml`, with the `wasm32-unknown-unknown`
and `wasm32-wasip2` targets.

```
cargo test --workspace
cargo check --workspace --all-targets --manifest-path host/Cargo.toml
```

The hooks in `.pre-commit-config.yaml` (run with prek) are line-ceiling, responsa,
clippy on both roots, clippy with `mere-view`'s query feature, and cargo deny's
bans check on `host/` with `deny.toml`.

## What is not here

The engine (Mere's five crates) is in mere.git. The reference application
(`ports/graphshell`, with `graphshell-mere`, `graphshell-blocks`,
`graphshell-dramatis-pane` and `graphshell-mcp`) is in Turnstone. `pictograph` is
in emblem, and `athanor` (`mere-athanor`) is in Distillery.

## History

The crates carry their Mere history: each was exported from Mere and merged in.
The toolkit under `host/` was imported from mere.git `325b0083b` with every file's
former paths, so `git log --follow` on a moved file reaches back past its renames
in Mere. The contract crates came earlier:

| Here | In Mere before the move |
|---|---|
| `crates/chirograph` | `crates/chirograph` |
| `crates/sceno` | `crates/cambium/scenes/sceno` |
| `crates/scenotime` | `crates/cambium/scenes/scenotime` |
| `crates/hest` | `crates/cambium/hest` |
| `crates/graphshell-endpoint` | `crates/graphshell/graphshell-endpoint` (without `src/door/`) |
| `crates/graphshell-endpoint/wit/world.wit` | `crates/script/wit/world.wit` |
| `crates/graphshell-client` | `crates/graphshell/graphshell-client` |
| `crates/script-wire` | `crates/script/wire` |
| `crates/script-witgen` | `crates/script/witgen` |
| `crates/tinct` | `crates/cambium/tinct` (copied byte for byte from Mere `9e790a152`, without history) |
| `crates/sceno-backdrops` | `scenomise::backdrops` and `scenomise::template` |

What this repository held before 2026-11-06 is recorded in
[the cut-over record](design_docs/graphshell_docs/technical_architecture/2026-11-06_cut_over.md);
the older content is in git history and on the repository's other branches.
Design documents are in `design_docs/`, indexed by `design_docs/DOC_README.md`.

## Licence

MPL-2.0, see [LICENSE](LICENSE). The sources carry the MPL-2.0 header and SPDX tag,
except `crates/hest`, whose sources came without headers and are covered by the
manifest's `license = "MPL-2.0"`; the golden file
`crates/script-wire/tests/generated/conformance.rs` is generated and compared
byte for byte with `script-witgen`'s output.
