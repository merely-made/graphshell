# Receipts: guest runtime plan

Run evidence for `graphshell_docs/implementation_strategy/2026-11-21_guest_runtime_plan.md`.

## 2026-11-21: P0 baseline and P1 on the projection guest

Machine: the Windows laptop. Graphshell at `09b1deed` plus the uncommitted probe. Wasmtime 51.0.0,
`js-component-bindgen` 2.19.1, `wasm-bindgen` 0.2.129, Node 24.11.0.

**P0, both guests rebuilt from current sources, then on Wasmtime 51 with its compiler:**

- `cargo update graphshell-endpoint` in `guest-projection` moved it from `0fa13d15` to `09b1deed`; both
  guests built for `wasm32-wasip2 --release`.
- `cargo test -p app-host --test projection`: 2 passed.
- `cargo test -p document-host`: 56 passed, 0 failed.
- `tool reference`: 20 calls written to `web/reference.json`.

**Conversion (`tool convert`):**

| Guest | Inner module | Two small modules | Glue | Total |
| --- | --- | --- | --- | --- |
| projection | 1,763,573 B | 478 B, 391 B | 393,262 B | 2,157,703 B |
| document | 63,362 B | 338 B, 303 B | 326,104 B | 390,106 B |

Both import the WASI stubs module; projection also imports `mere:script/projection-session`, document
imports `mere:script/caps`, `document-host`, `log` and `net`.

**Browser-side host:** `guest_runtime_web_host.wasm` builds for `wasm32-unknown-unknown --release`
(1,404,697 B); after `wasm-bindgen --target web`, 905,641 B of module and 13,705 B of glue.

**P1 on the projection guest:**

| Where | Calls equal | Sessions closed / open | Control (`differsAt`) |
| --- | --- | --- | --- |
| Node 24.11.0 | 20 of 20 | 1 / 0 | 3, as altered |
| Built-in browser, `Chrome/156.0.8078.100` | 20 of 20 | 1 / 0 | 3, as altered |

No console errors in the browser. `guest.run()` took 102 ms under Node and 130 ms and 1,444 ms on two
browser loads; these are single unrepeated readings, not a speed measurement.

Not run: the document guest in a browser, and so none of its three guards.

## 2026-11-21: P1 on the document guest

Machine: the Windows laptop. Graphshell at `a1a47e24`, everything rebuilt in the main checkout. Node
24.11.0; the built-in browser, `Chrome/156.0.8078.100`. `web-host` now carries `FetchGate` and
`limitMemory`: 954,043 B of module and 18,914 B of glue after `wasm-bindgen`. The bomb guest converts to
53,382 B of module and 290,972 B of glue.

`node run-node-document.mjs` and `document.html`; both report `passed: true` and every check true.

| Case | Node | Browser | Native outcome matched |
| --- | --- | --- | --- |
| Time limit, 1,000 ms: runaway loop | terminated at 1,015 ms | terminated at 1,008 ms | trapped |
| Control: a noop turn, same limit | applied in 1.9 ms | applied in 1.2 ms | completed |
| Memory cap 32 MiB: allocation bomb | trapped at 26,279,936 B | trapped at 26,279,936 B | trapped |
| Memory cap 64 MiB (the control) | trapped at 59,834,368 B | trapped at 59,834,368 B | trapped |
| Origin gate: `https://attacker.test/leak` | refused, native text | refused, native text | Refused, text Hello |
| Control: an allowed origin | applied, text changed | applied, text changed | Applied |
| Fetch cap 1: two fetches in a turn | second refused, native text | second refused, native text | Refused |
| Fetch cap 2 over two turns (the control) | both applied | both applied | Applied(1), Applied(2) |

After the time-limit trap the script is detached, the document is unchanged and a fresh worker completes
a turn. The bomb's trap is the guest's own (`unreachable`, its allocator failing) once the engine
refuses to grow past the written maximum. No console errors in the browser. The projection page and its
control were rerun on the same build: 20 of 20, 1 closed and 0 open, control caught at call 3.

Not run: Genet's DOM, any real network route, and any browser other than Chromium.

## 2026-11-21: P2, Pulley

Machine: the Windows laptop. Graphshell at `bab12eb9` plus the uncommitted probe and timing tests.
Wasmtime 51.0.0, rustc 1.100.0, Node 24.11.0. Commands are in `host/testing/guest-runtime-p2/README.md`.

**The unchanged hosts on Pulley.** Debug build, `cargo test -p document-host` with the Pulley flags:
56 passed, 0 failed, the same count as P0. Release build with `--include-ignored`, both packages, run
once on each backend; every `timing:` line names the backend that ran.

| Suite | Pulley | Compiler |
| --- | --- | --- |
| `app-host` (5, 3, 3) | all pass | all pass |
| `document-host` lib (36 run, 2 filtered) | pass | pass |
| `document-host` tests (1, 6, 1, 4, 4, 5) | all pass | all pass |
| two ignored doctests, forced by `--include-ignored` | fail to compile | fail to compile |

The skipped test (`discovers_native_mods_including_verso_and_nostrcore`) is ignored in the tree and
fails on both backends when forced.

**Timing readings**, medians within one run, one run per backend (a second projection run read 473 ms
and 40 ms):

| Reading | Pulley | Compiler | Ratio |
| --- | --- | --- | --- |
| Projection: compile the guest | 239.65 ms | 205.36 ms | 1.2 |
| Projection: instantiate | 0.258 ms | 0.232 ms | 1.1 |
| Projection: run the 20-call script (15 rounds) | 449.29 ms | 38.93 ms | 11.5 |
| Document: attach (10 rounds) | 19.47 ms | 19.21 ms | 1.0 |
| Document: one `set` turn (2,000 turns, 10 rounds) | 3,490.97 us | 110.42 us | 31.6 |
| Bomb: grow to a 256 MiB cap, compile included (7 rounds) | 55.80 ms | 55.46 ms | 1.0 |

Each timed run is checked: the projection transcript equals the native reference, each document turn
returns `Applied` with the next revision, the bomb traps.

**The arithmetic loop** (`tool time`, 50 million turns, the value checked against one worked out in
Rust, median of 5): compiler 44.0 ms, Pulley 336.3 ms, ratio 7.6. An earlier reading taken while other
builds ran was 67.9 ms and 736.0 ms, ratio 10.8, so these move with load.

**Wasmtime for `wasm32-unknown-unknown`** (`wasm-host`, release):

| Features | Outcome |
| --- | --- |
| `runtime`, `pulley` | compiles; the link wants `wasmtime_tls_get` and `wasmtime_tls_set`; with those supplied, 842,144 B, no imports |
| the same plus `component-model` | builds, 881,990 B, no imports |
| the same plus `std` | Wasmtime does not compile: 3 errors (`sys::mmap`, `Mmap`, `MmapVec::new_mmap`) |

**Pulley bytecode on that module in Node 24.11.0** (`run-node.mjs`; `tool precompile` wrote 2,440 B
for `pulley32` with signal traps, copy-on-write and memory reservations off): `run()` returned
1542256704, the expected value, on every call. Three runs of 9 calls each: medians 401.9, 466.5 and
421.2 ms, fastest 395 ms, slowest about 890 ms (the first call of a process). Each call also builds an
engine and deserializes the module.

Not run: a component or either guest on the wasm-built Wasmtime; that build in a browser page; async,
epochs or a memory cap there.

## 2026-11-22: P3, wasmi

Machine: the Windows laptop. Graphshell at `17e967bc` plus the uncommitted probe. wasmi 2.0.2, rustc
1.100.0, Node 24.11.0; the built-in browser, `Chrome/156.0.8078.100`. Commands are in
`host/testing/guest-runtime-p3/README.md`. The module is 100 bytes; `run()` must return 1542256704.

| Where | Result | Time for the loop, compile included | Short fuel (1,000,000) |
| --- | --- | --- | --- |
| Native | right | 121.4 and 114.6 ms (median of 5, two builds) | cut off |
| Native, fuel counted | right | 136.4 and 168.7 ms (one run each) | |
| Node, wasmi as a wasm module | right | 1,390.6 ms (median of 5; 1,292 to 1,407) | cut off |
| Chromium, the same | right | 686.7 ms (median of 5; 667 to 1,712) | cut off |

The cut-off message is the same everywhere: "all fuel consumed by WebAssembly". No console errors in
the browser. The browser reading was taken with wasmi's default features (2,912,278 B); without its
text-format parser the module is 1,957,577 B, which is what the Node row and the tree now use.

Beside P2, the same loop on the same machine: Wasmtime's compiler 44.0 ms, Pulley natively 336.3 ms,
Pulley on a wasm-built Wasmtime in Node about 400 to 470 ms.

**The `tail-call` build** (`RUSTFLAGS="-C target-feature=+tail-call"`, 2,863,408 B, Node): four
unlimited runs took 931.0, 912.2, 908.6 and 931.1 ms with the right value; the first run with fuel
counted threw "Maximum call stack size exceeded" inside wasmi's `consume_fuel`.

Not run: any Graphshell guest or any component on wasmi; the module layer is a written sketch only.

## 2026-11-22: P4, Wasmer

Machine: the Windows laptop. Graphshell at `1071722f` plus the uncommitted probe. Wasmer 7.5.0,
wasm-bindgen 0.2.130 (library and CLI), Node 24.11.0; the built-in browser, `Chrome/156.0.8078.100`.
The module is P3's; `run()` must return 1542256704.

| Where | Result | Time for the loop, compile included |
| --- | --- | --- |
| Native, default flavour | did not build: "The Cranelift compiler backend is not supported on Windows. Use the V8 backend instead." | |
| Node, `js` flavour | right | 79.6 ms (median of 5; 65.6 to 125.6) |
| Chromium, `js` flavour | right | 59.1 ms (median of 5; 58.3 to 84.6) |

`wasmer_host_bg.wasm` is 604,811 B and its glue 19,261 B. No console errors in the browser. The first
`wasm-bindgen` attempt failed because the installed CLI was 0.2.129 and Wasmer's `js-sys` requires
0.2.130; a matching CLI was built into a scratch directory.

Not run: Wasmer natively; its `v8` flavour (the build downloads a prebuilt V8); any guard under `js`;
any Graphshell guest.

## 2026-11-23: P7, a guest on Wasmtime in a page

Machine: the Windows laptop. Graphshell at `39bd50c7` plus the probe. Wasmtime 51.0.0 with `runtime`,
`pulley` and `component-model`, no `std`; Node 24.11.0; the built-in browser, `Chrome/156.0.8078.100`.
Commands are in `host/testing/guest-runtime-p7/README.md`. The lane ran everything; `node run-node.mjs`
and the page were rerun afterwards on the same build and agree.

**Artefacts:** the wasm-built host 2,789,715 B, no imports. Bytecode for `pulley32`: projection
4,146,888 B, document 250,144 B, bomb 214,104 B. The guest components are 1,779,273 B, 74,111 B and
62,321 B.

**Projection guest:** equal to the native reference in 20 of 20 calls, 1 session closed and 0 open; a
reference altered at call 3 is caught at call 3. The same on native `pulley64` with the same settings
(the control that the bindings and settings are right), in Node and in Chromium. No console errors.

**Document guest, 13 cases and the persistence checks, all pass in the three places:**

| Guard | Case | Control |
| --- | --- | --- |
| Fuel | a runaway loop at 2,000,000 fuel traps, "all fuel consumed by WebAssembly" | a noop turn under the same fuel completes on 7,138 |
| Epoch | a `set` turn with deadline 1 and a tick on every import traps, "interrupt" | the same turn with nothing ticking applies |
| Epoch, its limit | a loop that calls no import is not stopped by the epoch; fuel ends it | |
| Store limit | the bomb at a 32 MiB cap traps at 26,279,936 B, 2 growths denied | at 64 MiB it reaches 59,834,368 B; a noop under 32 MiB has none denied |
| Fetch gate | a foreign origin is refused with the native text | an allowed origin applies |
| Fetch cap | a second fetch past a cap of 1 is refused | with a cap of 2 it applies |
| Script | turns persist the document and instance; a cut-off turn ends the script and a fresh attach completes a turn | a wrong expectation is reported as failing |

**Timing beside D1's route** (medians; Node shows two runs; the browser's timer is coarsened to
0.1 ms; D1's per-turn figure carries a listener added each turn in P1's supervisor):

| Workload | D1, the engine's own wasm | Wasmtime on Pulley, in wasm |
| --- | --- | --- |
| Projection, 20 calls, run only (7 rounds), Node | 60.8 / 53.3 ms | 578.2 / 556.8 ms |
| the same, Chromium | 49.3 ms | 517.5 ms |
| Projection, load to result, Chromium | 63.8 ms | 525.7 ms |
| Document `set` turn (200 turns, 5 rounds), Node | 0.275 / 0.251 ms | 0.038 / 0.044 ms |
| the same, Chromium | 0.253 ms | 0.037 ms |
| Document attach, Node | 82.8 / 76.5 ms | 0.47 / 0.32 ms |
| the same, Chromium | 22.5 ms | 0.3 ms |
| Bomb to a 256 MiB cap, Chromium | 51.3 ms | 27.0 ms |
| Runaway loop cut off | at its time limit (P1: 1,008 ms of 1,000) | by fuel: 2,000,000 in 23.7 ms; 50,000,000 in about 520 ms |

Deserializing the 4.1 MB projection bytecode took 1.9 ms in Node. Natively on the same cases, release,
15 rounds: the projection run 40.6 ms on the compiler and 482.3 ms on `pulley64`.

**What the page supplied:** the host module and the bytecode, copied in; nothing else. Not built for
the target: `wasmtime-wasi` ("This wasm target is unsupported by mio", then getrandom's "not supported
by default"). Not available: async (no stack switching on wasm32), a clock.

Not run: any browser but Chromium 156; Genet's DOM; a real network route; an epoch ticked from another
thread; size work on the host module.
