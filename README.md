# Graphshell

Graphshell is the protocol layer of the Mere family: the contract between an
application that shows other applications (a host) and an application that can be
shown (a guest), and the kit a host-role application needs to speak it. It is the
bottom of the stack. Nothing in this repository depends on Mere, and Mere, Murm,
Knot, Cleromancy, Turnstone and the other apps take it from here.

The rulings are Mere's cycle breaks plan, D7 (extract the contract leaves first),
D8 (the bottom layer is this repository, named for the protocol) and D9 (the
contract plus the host-role kit): `mere/design_docs/mere_docs/implementation_strategy/2026-11-06_cycle_breaks_plan.md`,
with the research behind them in
`mere/design_docs/mere_docs/research/2026-11-06_host_guest_contract_brief.md`.

## What is here

| Crate | Role |
|---|---|
| `chirograph` | The projection contract: versioned, carrier-neutral messages by which an endpoint offers a scene, a resource or an action and a host asks for one; the `Carrier` trait; the NDJSON line codec |
| `sceno` | Scene vocabulary: source references, spaces, footprints, persisted scores, scene instances |
| `scenotime` | Scene runtime: epochs, idempotent diffs, host-clocked transitions, traces and picking |
| `hest` | Shared command menus, palette sessions and command ids |
| `graphshell-endpoint` | The traits an application implements beside its own truth (`ProjectionCatalog`, `ProjectionSource`, `PresentationSource`, `IntentSink`), the `stdio`, `local` and `network` carriers, and the WIT projection session (`wit`, `wit-ipc`, `wit-testkit`) |
| `graphshell-client` | Transport-independent client state: snapshots, diffs, resume, resource cache, the frozen realization |
| `script-wire` | wRPC value encoding and blocking transports (OS pipes, `ipc-channel`) for the WIT bindings |
| `script-witgen` | Generates Rust bindings for a WIT interface over `script-wire`; a build dependency |
| `tinct` | Perceptual (OKLCH) seed-to-palette derivation and a contrast-gated syntax-highlight palette, for every host and the reading stack |

The WIT world is `crates/graphshell-endpoint/wit/world.wit` (package
`mere:script@0.1.0`, the name is a wire name and is kept). It is the only copy.
`graphshell-endpoint` publishes its directory to dependents' build scripts through
Cargo `links` metadata (`DEP_GRAPHSHELL_ENDPOINT_WIT`).

## What is not here

The Graphshell reference host (`ports/graphshell`, `graphshell-mere`,
`graphshell-host`) stays in Mere, above this layer. So do the parts of the old
endpoint that need Mere's custody and identity stack: the application-door client,
the identity profile and the projection provider and offer routes, now the
`graphshell-door` crate in Mere. `scenomise` (the choreography compiler),
`scenograph` and `mere-curation` also stay in Mere: nothing here uses them.

## The rule

Each crate here depends on nothing in Mere, in normal, build or dev dependencies.
A test that needs Mere code is a consumer test and lives in Mere. Check with
`grep -c 'merely-made/mere' Cargo.lock`, which must print 0.

## Building

```
cargo +1.97.1 test --workspace --locked
```

## History

The crates carry their Mere history: each was exported from Mere with the
paths below and merged in. What this repository held before is recorded in
[docs/2026-11-06_cut_over.md](docs/2026-11-06_cut_over.md). The older content is
in git history and on the repository's other branches.

| Here | In Mere before the move |
|---|---|
| `crates/chirograph` | `crates/chirograph` |
| `crates/sceno` | `crates/cambium/scenes/sceno` |
| `crates/scenotime` | `crates/cambium/scenes/scenotime` |
| `crates/hest` | `crates/cambium/hest` |
| `crates/graphshell-endpoint` | `crates/graphshell/graphshell-endpoint` (without `src/door/`) |
| `crates/graphshell-endpoint/wit/world.wit` | `crates/script/wit/world.wit` (its identical copy under the endpoint was dropped) |
| `crates/graphshell-client` | `crates/graphshell/graphshell-client` |
| `crates/script-wire` | `crates/script/wire` |
| `crates/script-witgen` | `crates/script/witgen` |
| `crates/tinct` | `crates/cambium/tinct` (copied, without history; see below) |

2026-11-07: `tinct` joined this root under Mere's
`design_docs/mere_docs/implementation_strategy/2026-11-07_engine_split_plan.md`,
D19 ("tinct joins the bottom root, Inker may name it"). Its files were copied
byte for byte from Mere at `9e790a152b369c1c0bf3a69c0185dd6c7bf8074c`; only the
manifest changed (workspace `serde`, the repository comment). Its history before
that is in Mere, under `crates/cambium/tinct`.

## Licence

MPL-2.0, see [LICENSE](LICENSE). The sources carry the MPL-2.0 header and SPDX tag,
except `crates/hest`, whose sources came without headers and are covered by the
manifest's `license = "MPL-2.0"`; the golden file
`crates/script-wire/tests/generated/conformance.rs` is generated and compared
byte for byte with `script-witgen`'s output.
