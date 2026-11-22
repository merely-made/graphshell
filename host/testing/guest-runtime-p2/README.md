# guest-runtime-p2

Probe for the guest runtime plan
(`design_docs/graphshell_docs/implementation_strategy/2026-11-21_guest_runtime_plan.md`),
P2: Wasmtime's interpreter, Pulley, in place of its compiler.

Two questions, two halves.

## The same hosts on Pulley, natively

Nothing here: the hosts and their tests run unchanged. Wasmtime's build script
sets a `default_target_pulley` cfg on machines its compiler has no backend for,
and every `Config::new()` then targets Pulley. Passing that cfg by hand, with
the `pulley` feature on, gives the same build on this machine:

```sh
# From host/. A short --target-dir: the linker fails on long paths on Windows.
RUSTFLAGS="--cfg default_target_pulley" cargo test --release --no-fail-fast \
  -p app-host -p document-host --features wasmtime/pulley --target-dir C:/t/pulley-target \
  -- --include-ignored --nocapture --test-threads=1 --skip discovers_native_mods
# The compiler, for comparison: the same line without RUSTFLAGS, --features and --target-dir.
```

`--include-ignored` runs three timing readings (`timing_of_...` in
`app-host/tests/projection.rs`, `document-host/tests/document_script.rs` and
`tests/guarded.rs`). Each prints one `timing:` line that starts with the
backend that ran, `pulley` or `compiler`, which is the check that the flags
took. Two ignored doctests and the skipped ignored test fail the same way on
both backends and are not part of this.

This is how the probe selects Pulley, not how a product would: a build that
wants Pulley calls `Config::target`.

## Wasmtime itself as a wasm module

- `wasm-host/`: the smallest Wasmtime host, built for `wasm32-unknown-unknown`
  with the runtime and Pulley and no compiler. It takes a module already
  compiled to Pulley bytecode and calls its `run` export. It supplies the two
  thread-local functions Wasmtime's embedded build asks its host for.
- `tool/`: native. `precompile` writes the probe module (a 50 million turn
  loop) as Pulley bytecode for that host; `time` runs the same module natively
  on the compiler and on Pulley; `expected` prints the value `run` must return.
- `run-node.mjs`: loads the host in Node, hands it the bytecode, checks the
  value and times the call.

Each of `wasm-host/` and `tool/` is its own workspace root.

```sh
cd testing/guest-runtime-p2/wasm-host
cargo build --release --target wasm32-unknown-unknown --target-dir C:/t/p2-wasm-target
# Also builds with --features component-model. --features std does not compile.

cd ../tool
cargo build --release --target-dir C:/t/p2-tool-target
C:/t/p2-tool-target/release/guest-runtime-p2-tool precompile ../module.cwasm
C:/t/p2-tool-target/release/guest-runtime-p2-tool time

cd ..
node run-node.mjs C:/t/p2-wasm-target/wasm32-unknown-unknown/release/guest_runtime_p2_wasm_host.wasm \
  module.cwasm "$(C:/t/p2-tool-target/release/guest-runtime-p2-tool expected)"
```

`run-node.mjs` passes when `run` returned the expected value. The module it
runs is a core module, not a component: no Graphshell guest has been run this
way.
