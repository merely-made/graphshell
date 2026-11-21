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
