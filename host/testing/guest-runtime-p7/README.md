# guest-runtime-p7

Probe for the guest runtime plan
(`design_docs/graphshell_docs/implementation_strategy/2026-11-21_guest_runtime_plan.md`),
P7: a guest on Wasmtime in a page. Wasmtime is built for
`wasm32-unknown-unknown` with its runtime, Pulley and the component model, no
compiler and no `std` feature, and runs the projection guest and the document
guests as Pulley bytecode compiled ahead of time. The page gives it bytes and
reads JSON.

- `tool/`: native. `precompile` writes a guest component as Pulley bytecode
  (`pulley32` for the wasm-built host, `pulley64` for the native control) with
  the same embedded settings the wasm host runs under; `native` is the
  control (the same runtime code, natively, on `pulley64`); `time` reads the
  same cases on the compiler and on `pulley64`; `reference` writes the native
  transcript; `imports` lists what a guest imports.
- `wasm-host/`: the Wasmtime build for the page. A cdylib; it has no imports.
- `shared/`: the Rust both crates include by path, so the control and the page
  run one body of code. `projection_host.rs` is `app-host`'s test host (the
  `bindgen!` bindings and the `Host` impl over the fixture server);
  `document_host.rs` is the document guest's host (a fixture document,
  `document-host`'s `net.rs` and fetch rules, a fuel limit, an epoch deadline
  and a `StoreLimits` cap); `stubs.rs` is the WASI the guests import, granting
  nothing; `config.rs` is the settings the tool and the host must share;
  `driver.rs` is the JSON the page and the tool speak.
- `cases.json`: the document cases and their expectations, read by the tool
  and the page alike.
- `web/`: `run-p7.js` (the checks), `run-node.mjs`, `index.html` (sets
  `window.__result`), `timing.js` with `timing-node.mjs` and `timing.html`,
  and `pulley-host.js` (the page's side of the host).

Each of `tool/` and `wasm-host/` is its own workspace root with its own
untracked lock. `build/` is the probe's build products.

## Build

```sh
# From host/. A short --target-dir: the linker fails on long paths on Windows.
# The guests' generate! reads crates/script/wit/world.wit, which the hosts'
# build scripts write from graphshell-endpoint (ignored by git); copy it first.
mkdir -p crates/script/wit && cp ../crates/graphshell-endpoint/wit/world.wit crates/script/wit/world.wit
(cd crates/script/app-host/guest-projection && cargo build --target wasm32-wasip2 --release --target-dir C:/t/p7-target/gp)
(cd crates/script/document-host/guest && cargo build --target wasm32-wasip2 --release --target-dir C:/t/p7-target/dg)
(cd crates/script/document-host/guest-bomb && cargo build --target wasm32-wasip2 --release --target-dir C:/t/p7-target/db)

cd testing/guest-runtime-p7
(cd tool && cargo build --release --target-dir C:/t/p7-target/tool)
(cd wasm-host && cargo build --release --target wasm32-unknown-unknown --target-dir C:/t/p7-target/wasm)

mkdir -p build && cd build
T=C:/t/p7-target/tool/release/guest-runtime-p7-tool.exe
GP=C:/t/p7-target/gp/wasm32-wasip2/release/projection_guest.wasm
DG=C:/t/p7-target/dg/wasm32-wasip2/release/document_core_guest.wasm
DB=C:/t/p7-target/db/wasm32-wasip2/release/document_core_guest_bomb.wasm
cp C:/t/p7-target/wasm/wasm32-unknown-unknown/release/guest_runtime_p7_wasm_host.wasm host.wasm
for target in pulley32 pulley64; do
  $T precompile projection $GP projection.$target.cwasm $target
  $T precompile document $DG document.$target.cwasm $target
  $T precompile bomb $DB bomb.$target.cwasm $target
done
# Step 1, the native control: pulley64, the same code and the same embedded settings.
$T native projection.pulley64.cwasm document.pulley64.cwasm bomb.pulley64.cwasm \
  reference.json ../cases.json native-pulley64.json
```

Rebuild the guests from current sources before any comparison (the plan's
Findings 5): `Cargo.lock` is untracked, so a guest resolves fresh.

## Run

```sh
cd web
node run-node.mjs --quiet    # step 2 and 3 in Node: projection transcript, its control, every case
node run-node.mjs            # the same as one JSON object; exit 0 when everything passes

# Chromium: serve host/testing/ (so P1's files are reachable for the timing page).
cd ../.. && python3 -m http.server 8777 --bind 127.0.0.1
# open http://127.0.0.1:8777/guest-runtime-p7/web/index.html   (window.__result.passed)
```

`index.html` and `run-node.mjs` pass when the transcript equals the native
reference in all 20 calls, the control (which alters call 3 of the reference)
is caught at call 3, the one session the guest opened was closed and none is
left open, every case in `cases.json` meets its expectation and agrees with
the native `pulley64` run on outcome, message, revision, text, fuel used,
memory peak and denied growths, and the instrument check (a wrong expectation
must fail) holds.

## Timing (step 4)

D1's route is P1's, so build P1's products first (its README has the full
commands; the short form, from `host/testing/guest-runtime-p1`):

```sh
cd tool && cargo build --target-dir C:/t/p7-target/p1tool
T1=C:/t/p7-target/p1tool/debug/guest-runtime-tool.exe
$T1 reference ../web/reference.json
$T1 convert $GP ../web/projection projection
$T1 convert $DG ../web/document document
$T1 convert $DB ../web/bomb bomb
cd ../web-host && cargo build --target wasm32-unknown-unknown --release --target-dir C:/t/p7-target/p1web
wasm-bindgen --target web --no-typescript --out-dir ../web/host \
  C:/t/p7-target/p1web/wasm32-unknown-unknown/release/guest_runtime_web_host.wasm
```

then

```sh
cd host/testing/guest-runtime-p7/web
node timing-node.mjs                 # JSON; D1 and Pulley in one process
# or http://127.0.0.1:8777/guest-runtime-p7/web/timing.html (window.__result)
# Native readings of the same cases (compiler and pulley64):
$T time $GP $DG $DB
```

Each reading is a median over the rounds the output names, and each timed run
is checked.

## What the page supplies, and what it does not

- The wasm host has no imports: `WebAssembly.Module.imports` is empty, so the
  page supplies nothing to Wasmtime. The two platform functions its embedded
  build asks for (`wasmtime_tls_get`, `wasmtime_tls_set`) are defined in the
  host's own Rust (`wasm-host/src/lib.rs`).
- The WASI the guests import is not the page's: `shared/stubs.rs` defines it
  inside the host, through the `Linker`, granting nothing.
- No clock: both WASI clocks answer zero, since `std::time` panics on
  `wasm32-unknown-unknown`. Nothing in these runs read one.

`wasm-host/Cargo.toml` carries two features that are attempts, kept so their
failures can be reproduced: `async` (compiles; fibers are unsupported on
wasm32, so `call_async` could not yield) and `wasi` (`wasmtime-wasi` does not
build for the target). `Component::deserialize` trusts its bytes, so a page
must load only bytecode its own build wrote.
