# guest-runtime-p1

Probe for the guest runtime plan
(`design_docs/graphshell_docs/implementation_strategy/2026-11-21_guest_runtime_plan.md`),
P0 and P1: does a component guest run on a browser's own wasm engine and
reproduce what it does on Wasmtime?

The test article is `app-host`'s projection guest. Natively it runs a shared
script of calls against a fixture server and its transcript must equal a
native run of the same script. Here the same guest is converted to plain
modules, the same fixture server is compiled to wasm, and a page joins them.

- `tool/`: native. `reference` writes the native transcript; `convert` turns a
  component into modules and glue with the converter inside `jco`, used from
  Rust.
- `web-host/`: the fixture server as a `wasm32-unknown-unknown` library, plus
  the two pieces the document guest needs: `FetchGate` (the native
  `net.rs` origin rules compiled from the same file, with the count and origin
  order of `document-host`'s `src/host.rs`) and `limitMemory` (writes a memory
  maximum into a core module). Both have native unit tests.
- `web/`: the page, the shared runner, a Node runner and the WASI stubs.
  `projection/`, `document/`, `bomb/`, `host/` and `reference.json` are build
  products.

Each of `tool/` and `web-host/` is its own workspace root with its own lock.

```sh
# From host/: build the guest from current sources.
(cd crates/script/app-host/guest-projection && cargo update graphshell-endpoint \
  && cargo build --target wasm32-wasip2 --release)

cd testing/guest-runtime-p1/tool
cargo run -- reference ../web/reference.json
cargo run -- convert \
  ../../../crates/script/app-host/guest-projection/target/wasm32-wasip2/release/projection_guest.wasm \
  ../web/projection projection

cd ../web-host
cargo build --target wasm32-unknown-unknown --release
wasm-bindgen --target web --no-typescript --out-dir ../web/host \
  target/wasm32-unknown-unknown/release/guest_runtime_web_host.wasm

cd ../web
node run-node.mjs            # passes when the transcript matches
node run-node.mjs --control  # passes when an altered reference is caught
python3 -m http.server 8767  # then open http://localhost:8767/ and /?control
```

The page sets `window.__result`; `passed` is the verdict. A run passes when all
calls match the reference, the one session the guest opened was closed, and
none is left open.

## The document guest and its guards

The second guest is `document-host`'s document guest, which natively runs
under a time limit, a memory cap, an origin gate and a per-turn fetch cap.
Here it runs on the engine's own wasm support with the same four guards, each
beside a control that must come out the other way (`web/run-document.js`):

- Time limit: the guest runs in a worker (`guest-worker.js`) and a supervisor
  (`supervisor.js`) terminates the worker when a turn passes its limit, because
  an engine cannot interrupt wasm from its own thread. A trapped turn ends the
  script, as natively; attaching again gives a fresh worker.
- The document is the supervisor's, not the worker's. Each turn the worker is
  sent the event and a snapshot of the document view, serves `inspect` from it
  and returns a batch, which the supervisor applies. A terminated worker leaves
  the document untouched. The document is a fixture (`document-fixture.js`):
  the native test page held as a plain tree, not Genet's DOM and not under
  test.
- Memory cap: `limitMemory` rewrites the guest's core module before it is
  compiled so its memory has a maximum, and the engine refuses growth itself.
- Origin gate and fetch cap: `FetchGate`, with fetching faked as the native
  tests fake it. No network is reached.
- `env.js` hides what differs between Node and a page (reading files, starting
  a worker), so the supervisor is the same code in both.

```sh
# The guests, built from current sources. The guests' `generate!` reads
# crates/script/wit/world.wit, which document-host's build.rs writes from
# graphshell-endpoint's wit/world.wit; run that first (or copy the file).
(cd crates/script/document-host/guest && cargo build --target wasm32-wasip2 --release)
(cd crates/script/document-host/guest-bomb && cargo build --target wasm32-wasip2 --release)

cd testing/guest-runtime-p1/tool
cargo run -- convert \
  ../../../crates/script/document-host/guest/target/wasm32-wasip2/release/document_core_guest.wasm \
  ../web/document document
cargo run -- convert \
  ../../../crates/script/document-host/guest-bomb/target/wasm32-wasip2/release/document_core_guest_bomb.wasm \
  ../web/bomb bomb

cd ../web-host
cargo test                   # FetchGate and limitMemory, natively
# then build the host module as above (it now includes both)

cd ../web
node run-node-document.mjs   # one JSON object; exit 0 when every case and control passes
python3 -m http.server 8767  # then open http://localhost:8767/document.html
```

`document.html` sets `window.__result` with `passed` and one entry per case.
