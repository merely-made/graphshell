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
- `web-host/`: the fixture server as a `wasm32-unknown-unknown` library.
- `web/`: the page, the shared runner, a Node runner and the WASI stubs.
  `projection/`, `document/`, `host/` and `reference.json` are build products.

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
