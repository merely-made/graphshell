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
