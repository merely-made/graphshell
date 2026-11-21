# Guest runtime plan: where wasm guests run when Wasmtime's compiler cannot

**Status:** 2026-11-21. In progress. P0 is met, and P1 is met for the projection guest: converted to
modules, it reproduces the native transcript in a browser, all 20 calls. P1's document-guest half and
P2 to P5 are not started. D1 to D7 ruled the same day; D4 to D7 set how the document guest runs in a
browser, and that run is being built.

**For:** the web build. Graphshell's wasm hosts run guests on Wasmtime, which compiles them to machine
code, and a browser grants no page the right to do that. No ruling says how a component guest runs when
the host itself is in a browser. This plan proves the route Mark chose (the browser's own engine, D1)
and evaluates three others against the same guests: Wasmtime's interpreter Pulley, wasmi, and Wasmer.

**Not here:** changing the WIT contracts; replacing Wasmtime on native; merging `block-host` and
`authority-host` into `app-host` (`mere:engine-split D52`, ruled and not yet done); the script lanes
(Inker's `2026-11-21_lua_evaluator_plan.md`).

## Constraints

- Two test articles, both existing guests (D2). The projection guest with its 19-call script, whose
  transcript `app-host`'s test already compares byte for byte with a native run. The document guest,
  which also needs its three guards: the time limit, the memory cap and the fetch gate.
- A runtime passes a test article when it reproduces the native reference, not when it runs.
- Guests are rebuilt from current sources before any comparison (Findings 5).
- wasmi and Wasmer get a desk study and one small probe each, no port (D3).
- Browser runs use a real Chromium, driven by the harness, never synthetic OS input.
- A speed number names its workload and carries a positive control in the same run.
- Conclusions are scoped to what ran: the machine, the browser and the versions in the receipts.

## Phases

### P0. Baselines (gate)

**Done-conditions:** both guests rebuilt from current sources; the projection transcript and the
document guest's outcomes (a normal turn, a runaway turn cut off, an allocation past the cap denied, a
cross-origin fetch refused) recorded on Wasmtime 51 with its compiler; each later phase compares against
these records and no other.

### P1. The browser's own engine (D1)

Convert both components to modules a browser loads, with `jco`, and run them from a page.

**Done-conditions:** the projection transcript from Chromium equals the P0 reference byte for byte; for
each of the document guest's three guards, either a test shows it reproduced in the browser or the plan
names what replaces it there (a time limit has no epoch to trap on, so a worker that is terminated is
the candidate); the size of what ships beside the guest is recorded.

### P2. Pulley

The same hosts with Wasmtime's `pulley` feature and a Pulley target, on the native machine.

**Done-conditions:** both test articles reproduce P0, guards included; slowdown against the compiler is
measured on both articles; an attempt to build a minimal Wasmtime host for `wasm32-unknown-unknown` is
made and its outcome recorded, since upstream lists only `wasm32-wasip1` and that as untested.

### P3. wasmi (touches no product crate)

**Done-conditions:** a written account of what the projection component is as plain modules (its inner
module's imports and exports, and what the wrapper adds) and of the layer a module-only runtime would
need beneath WIT, sketched as a third output of `script-witgen`; one plain module run under wasmi on the
native machine and inside a `wasm32-unknown-unknown` build in Chromium, with the same result; its speed
beside P2's.

### P4. Wasmer

**Done-conditions:** a written account of its component-model status, its `js` backend and its stated
direction, each claim citing its source; the P3 module run under Wasmer natively and, through the `js`
backend, in Chromium, with the same result.

### P5. Readout

**Done-conditions:** one table, option by row: contracts unchanged or not, native, browser, guards,
speed, crates added, upstream's own maturity statement; the questions put to Mark, each with its
evidence (whether a native profile without a compiler matters, whether the module layer is worth
building, whether Wasmer earns a second look).

## Decisions

Each records the evidence put to Mark, what was offered, his answer in his words, what follows, and an
outcome. D1 to D7 are in answer order.

- **D1. The web build runs guests on the browser's own engine (Mark, 2026-11-21):** Evidence: Wasmtime
  cannot compile inside a browser (`mere/design_docs/archive_docs/2026-08-20_completed_plans/2026-06-03_actor_constellation_plan.md:388`);
  a browser already runs wasm, and `jco` converts a component into modules it loads, so the WIT
  contracts carry over. Offered as one of four routes to evaluate. Answer: "using a browser's own engine
  for web builds (which is a real no brainer)". What follows: P1 proves it on both test articles; the
  other three are evaluated beside it. Outcome: applies `mere:stack.4`.

  Links: area The stack · draws on mere:stack.4 · plans guest-runtime P1 · summary In a web build, component guests run on the browser's own wasm engine; the WIT contracts are unchanged.

- **D2. Two test articles (Mark, 2026-11-21):** Evidence: `app-host`'s projection test compares a
  guest's transcript with a native run of the same 19 calls
  (`host/crates/script/app-host/tests/projection.rs:189`); `document-host`'s guest adds a time limit,
  a memory cap and a fetch gate. Offered: the projection guest only (recommended); projection plus
  document guest; a new minimal guest. Answer: "Projection plus document guest". What follows: every
  option is held to both, so an interpreter must reproduce the guards, not only the calls. Outcome: a
  method choice for this plan; it bears on no doctrine statement.

  Links: area The stack · plans guest-runtime P0 · summary Every runtime option is held to the projection guest's transcript and the document guest's three guards.

- **D3. wasmi and Wasmer get a desk study, a small probe, and the module question (Mark, 2026-11-21):**
  Evidence: both run plain modules; wasmi's component model is an open issue and Wasmer's README names
  none (Findings 3, 4), so neither loads these guests as built. Offered: desk study plus one small probe
  (recommended); desk study only; port the projection contract. Answer: "1, but I wonder what those
  components would look like as wasm modules, or what that layer would look like." What follows: P3 and
  P4 each run one plain-module probe, and P3 writes down what a component is as modules and what the
  layer beneath WIT would be. Outcome: a method choice for this plan; it bears on no doctrine statement.

  Links: area The stack · plans guest-runtime P3, guest-runtime P4 · summary wasmi and Wasmer are studied on paper with one plain-module probe each; P3 documents components as modules and the layer a module runtime needs.

- **D4. The document stays with the host, outside the worker (Mark, 2026-11-21):** Evidence: a browser
  cannot interrupt wasm from the thread running it, so a time limit needs a worker a supervisor can
  terminate; the guest only reads a view through `inspect` and returns a batch for the host to apply;
  and natively a trapped turn already ends the script with "the host survives and the DOM is unchanged;
  the caller should detach the script" (`host/crates/script/document-host/src/script.rs:131`). Offered:
  the document with the host, a snapshot sent each turn (recommended); the document inside the worker.
  Answer: "With the host, outside the worker (Recommended)". What follows: each turn the supervisor
  sends the event and a view snapshot, the worker returns a batch and its log lines, the supervisor
  applies it; terminating a runaway leaves the document untouched. Outcome: applies `mere:stack.8`.

  Links: area The stack · draws on mere:stack.8 · plans guest-runtime P1 · summary In a browser the guest runs in a worker a supervisor can terminate; the document stays with the host and crosses as a snapshot per turn.

- **D5. Memory is capped when a script loads (Mark, 2026-11-21):** Evidence: the guest's core module
  defines its own memory with no maximum and the generated glue does not expose it (Findings 12);
  natively the cap is a per-script quota, 64 MiB by default. Offered: set a maximum when the script
  loads (recommended); set one when the guest is converted; no cap in the browser. Answer: "Set a
  maximum when the script loads (Recommended)". What follows: the host rewrites the module's memory
  limit just before compiling it, so the engine refuses growth itself. Outcome: applies `mere:stack.8`.

  Links: area The stack · draws on mere:stack.8 · plans guest-runtime P1 · summary The browser host caps a guest's memory by rewriting the module's memory maximum per script before compiling it.

- **D6. The probe reuses the fetch-rule file; the split waits (Mark, 2026-11-21):** Evidence: the
  per-turn count and the origin check are host code in one import
  (`host/crates/script/document-host/src/host.rs:66`); the origin rule is in `net.rs`, which uses only
  the standard library, but its crate depends on Wasmtime and does not build for a browser. Offered:
  compile that one file into the probe and take the split to the readout (recommended); split
  `document-host` first; leave fetch out. Answer: "Reuse the one file in the probe, split later
  (Recommended)". What follows: the probe's browser host includes `net.rs` by path; whether
  `document-host` becomes a runtime-free core and a Wasmtime shell is a P5 question. Outcome: a method
  choice for this plan; it bears on no doctrine statement.

  Links: area The stack · plans guest-runtime P1, guest-runtime P5 · summary The probe compiles document-host's net.rs into its browser host; splitting document-host is left to the readout.

- **D7. Fetching is faked in the probe (Mark, 2026-11-21):** Evidence: the guest's `fetch` is a
  synchronous call and a browser's network calls are not; the native tests use a fetcher that echoes.
  Offered: the echo fetcher only, with the real routes recorded (recommended); also a blocking request
  in the worker; also promise integration. Answer: "Echo fetcher only, record the routes
  (Recommended)". What follows: the guards are tested with the echo fetcher; the two real routes, a
  blocking request from inside the worker and the browser's promise integration for wasm, go to P5
  untested. Outcome: a method choice for this plan; it bears on no doctrine statement.

  Links: area The stack · plans guest-runtime P1, guest-runtime P5 · summary The probe tests the fetch rules with an echo fetcher; real network routes for a guest in a browser are recorded, not tried.

## Open questions

1. Answered 2026-11-21 (D4): the time limit is a supervisor terminating the guest's worker. Natively
   a trapped turn also ends the script, so the lost state matches; what the browser cannot do is trap
   and keep the instance.
2. Answered 2026-11-21: `js-component-bindgen` 2.19.1 converts both guests from Rust (Findings 7), so
   the web build needs no Node tool.
3. Whether any shipping profile needs native execution without a compiler. If none does, Pulley, wasmi
   and Wasmer answer a question nobody is asking yet.

## Findings

2026-11-21, from upstream sources and this repository's code; nothing was run.

1. `jco` "transpiles" components into modules that run in a browser, combining the browser's own wasm
   support with the component features, and ships a WASI 0.2 shim for browsers (its README).
2. Wasmtime 51 lists Pulley as a supported execution backend that needs "more time fuzzing/baking", and
   describes it as the default wherever the compiler has no backend (`docs/stability-tiers.md`,
   `docs/examples-pulley.md` on `release-51.0.0`). As a host target it lists `wasm32-wasip1`,
   "supported but not tested".
3. wasmi mirrors Wasmtime's API, has fuel metering and supports WASI `wasip1`; the component model is
   open issue 897.
4. Wasmer's Rust crate has native backends and a `js` backend built on `wasm-bindgen`
   (`lib/api/Cargo.toml`); its README leads with a sandbox SDK and names no component model.
5. A guest keeps its own untracked lock. On 2026-11-21 the projection guest held an older
   `graphshell-endpoint` and its transcript lacked a field; rebuilding it cleared the failure.
6. The hosts are on Wasmtime 51 since `47455b8b`; `document-host` sets its epoch deadline before
   instantiating.

7. The probe is `host/testing/guest-runtime-p1/`. `js-component-bindgen` converts a component from
   Rust into its inner module, two small adapter modules and glue; the projection guest is 1.76 MB of
   module and 393 KB of glue.
8. The endpoint's fixture server (`graphshell-endpoint`, feature `wit-testkit`) builds for
   `wasm32-unknown-unknown`, so the host half of a web build is the same Rust the native test serves
   from. Joining it to the converted guest took a `Session` class of nine methods and two functions in
   the page (`web/run-projection.js`).
9. The guest's standard library reaches for WASI at start: environment, exit, the three standard
   streams, terminals, two clocks, poll and stream errors. Stubs that grant nothing were enough
   (`web/wasi-stubs.js`).
10. `WitServer::session_wait_notice` waits by sleeping a thread
    (`crates/graphshell-endpoint/src/wit/server.rs:265`). The fixture script never reaches the sleep, so
    the transcript is unaffected, but a host on a browser's main thread cannot block that way.

11. Read 2026-11-21: the time and memory guards are tested natively with a separate bomb guest (an
    endless loop, an endless 8 MiB allocation) through `run_guarded`
    (`document-host/tests/guarded.rs:32`, `:44`); the fetch rules with the ordinary guest
    (`tests/document_script.rs:194`, `:224`). No test takes a turn after a trap.
12. The document guest's core module has one memory, 17 pages at start with no maximum, exported and
    not imported; the glue keeps it in a closure. Every import is called synchronously by the glue,
    including `fetch`, and a returned promise would be wrapped as a value, not awaited.
13. `ScriptHost` mixes the document and fetch state with Wasmtime's `WasiCtx`, `ResourceTable` and
    `StoreLimits`; `dom_view.rs` names the binding-generated types. So the document logic does not
    build for a browser as it stands, and the probe's document is a fixture, not Genet's DOM.

## Progress

- 2026-11-21: plan written; D1 to D3 ruled.
- 2026-11-21: P0 met (both guests rebuilt; projection 2 passed and document-host 56 passed on Wasmtime
  51). P1 met for the projection guest in Node and in a browser, each with a control that alters one
  reference outcome and is caught. The document guest's half of P1 is next.
- 2026-11-21: D4 to D7 ruled for the document guest; its browser run is being built in
  `host/testing/guest-runtime-p1/`.
