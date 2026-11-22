# guest-runtime-p4

Probe for the guest runtime plan
(`design_docs/graphshell_docs/implementation_strategy/2026-11-21_guest_runtime_plan.md`),
P4: Wasmer. A desk study, each claim with its source, and the P3 module run
through Wasmer.

## Desk study

Read 2026-11-22. Wasmer 7.5.0 was released 2026-10-01 and is MIT licensed
(`Cargo.toml` on `main`; the release page).

- **No component model, and none planned.** Issue 4439 ("WASI Preview 2
  Support", closed as not planned on 2024-08-20) carries the maintainers' only
  statement: "we don't plan to support the Component Model", because only one
  server-side runtime had committed to it and Wasmer "follows strictly the
  features supported by browsers". No other tracking issue was found. So
  Wasmer runs a guest's inner module and needs the same wrapper wasmi does
  (`../guest-runtime-p3/README.md`).
- **Three flavours of the Rust crate** (`lib/api/Cargo.toml`): `sys`, with its
  own compilers (`cranelift`, `singlepass`, `llvm`), the default; `v8`, which
  links a V8 engine; and `js`, "where `wasmer` will be compiled to WebAssembly
  to run in a JavaScript host", built on `wasm-bindgen` and `js-sys`. Its
  README says of `js` that "there is no engines or compilers available": the
  browser's engine runs the module. The wamr and wasmi backends were dropped
  in 7.2.0.
- **Windows.** Release 7.3.0: "Windows support has been removed from the Sys
  compiler backends". On this machine the default build stops with "The
  Cranelift compiler backend is not supported on Windows. Use the V8 backend
  instead." The `v8` feature's build script downloads a prebuilt V8 archive
  from `wasmerio/wee8-custom-builds` (`build.rs`); the probe did not take it.
- **Guards under `js`.** Nothing documents fuel, a memory limit or
  interruption for the `js` flavour. The metering and memory-limit examples
  require `cranelift`, and the interrupt feature depends on the `sys` runtime.
  Treat them as absent there until shown otherwise.
- **Direction.** The README leads with "Lightweight sandboxes for your apps
  and AI agents": a CLI, an SDK it calls alpha, and running in a browser.

Not read: the changelog, and the `js` backend's source.

## The probe

`wasmer-host/` is one library: `run(bytes)` instantiates a module with no
imports and calls `run`. Natively it takes Wasmer's default flavour; for
`wasm32` it takes `js-default` and is wrapped with `wasm-bindgen`. `web/` has
the page, the shared runner and a Node runner; `module.wasm` (P3's, copied)
and `pkg/` are build products.

```sh
cp ../guest-runtime-p3/web/module.wasm web/   # after P3's `tool emit`

cd wasmer-host
cargo build --release --lib --target wasm32-unknown-unknown --target-dir C:/t/p4-target
# The wasm-bindgen CLI must match the wasm-bindgen the build resolved (0.2.130 here).
wasm-bindgen --target web --no-typescript --out-dir ../web/pkg \
  C:/t/p4-target/wasm32-unknown-unknown/release/wasmer_host.wasm

cd ../web
node run-node.mjs            # exit 0 when run() returned the expected value
python3 -m http.server 8770  # then open http://localhost:8770/

# Natively, where Wasmer's compilers build (not Windows):
cd ../wasmer-host && cargo run --release --bin time-native -- ../web/module.wasm 1542256704
```

The native run has not been made: this machine is Windows.
