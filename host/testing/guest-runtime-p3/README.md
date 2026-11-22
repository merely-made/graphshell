# guest-runtime-p3

Probe for the guest runtime plan
(`design_docs/graphshell_docs/implementation_strategy/2026-11-21_guest_runtime_plan.md`),
P3: wasmi, an interpreter written in Rust that runs plain wasm modules and has
no component model. Two parts: a written account of what one of our
components is underneath, and one plain module run on wasmi.

## What the projection component is, as plain modules

Read from the converted guest (`../guest-runtime-p1/web/projection/`), which
is the component taken apart.

**The inner module** (1.76 MB) is the guest's own code. It exports 8 things:
`memory`, `_initialize`, the three functions of its world (`run`,
`echo-failure`, `echo-notice`), a clean-up function for two of them
(`cabi_post_run`, `cabi_post_echo-failure`), and `cabi_realloc`, through which
a host asks the guest for memory to write a value into. It imports 35
functions:

- 11 from `mere:script/projection-session`: `open`, `discover`, eight
  `[method]session.*` functions and `[resource-drop]session`. This is the
  contract.
- 24 from WASI (`wasi:io`, `wasi:cli`, `wasi:clocks`): what the guest's
  standard library reaches for at start. Stubs that grant nothing satisfy all
  of them (P1).

Every one of those 35 has a core signature of integers and floats only. A
string is a pointer and a length into `memory`; a record is its fields laid
flat; a variant is a number saying which case, then its payload; a result too
wide to return is written where a pointer argument says; a `session` is a
32-bit handle.

**Two small modules** (478 B and 391 B) are plumbing: a table of 19 functions
the inner module's imports go through, filled in after the inner module
exists, because some imports need the inner module's own `memory` and
`cabi_realloc` to do their work.

**The wrapper** is everything that turns those integers back into values:
for each import, read the arguments out of guest memory, call the host, write
the result back (asking `cabi_realloc` for space); for each export, the
reverse, then call its `cabi_post_*`; and a table from 32-bit handles to the
host's sessions. On Wasmtime this is the component model. Converted for a
browser it is 393 KB of generated JavaScript.

## The layer a module-only runtime would need

wasmi (and Wasmer) run the inner module but know nothing of the wrapper. To
run a Graphshell guest on one, the host has to supply the wrapper itself.

`script-witgen` reads a WIT interface and writes two things today: a client
over a transport, and a `Handler` trait with a dispatcher that decodes a call
from wRPC's encoding and calls the handler. The module layer would be a third
output over the same parse and the same `Handler` trait:

- for each function of an imported interface, a host function with the core
  signature, which lifts its arguments from guest memory by the Component
  Model's layout rules, calls the `Handler`, and lowers the result into guest
  memory through `cabi_realloc`;
- for each exported function, the reverse, with the `cabi_post_*` call;
- a handle table per resource, and the `[resource-drop]` function;
- the names the inner module imports them under (`[method]session.fetch` and
  so on).

It would cover what `script-witgen` already covers (the synchronous model:
primitives, records, variants, enums, flags, options, results, tuples, lists,
resources) and refuse the same things (async, streams, futures). The layout
rules are the published Canonical ABI; the generator would have to follow
them exactly, and the test for that already exists in kind: the projection
transcript, byte for byte against a native run.

Beside it, the guest would need building as a plain module, not a component,
and the 24 WASI imports either stubbed, as in P1, or avoided by building the
guest for a target with no WASI.

None of this is built. It is a sketch of the work, not a measure of it.

## The probe

One plain module, the 50 million turn loop P2 timed, run by wasmi natively
and by wasmi built as a wasm module in a page.

- `wasmi-host/`: the library. `run(bytes, fuel)` instantiates the module and
  calls `run`; with fuel, a run that spends it all is cut off.
- `tool/`: native. `emit` writes the module; `time` runs it natively, then
  with fuel counted, then with too little fuel, which must be cut off;
  `expected` prints the value `run` must return.
- `web/`: the page, the shared runner and a Node runner. `module.wasm` and
  `wasmi_host.wasm` are build products.

```sh
cd testing/guest-runtime-p3/tool
cargo build --release --target-dir C:/t/p3-target
C:/t/p3-target/release/guest-runtime-p3-tool emit ../web/module.wasm
C:/t/p3-target/release/guest-runtime-p3-tool time ../web/module.wasm

cd ../wasmi-host
cargo build --release --target wasm32-unknown-unknown --target-dir C:/t/p3-target
cp C:/t/p3-target/wasm32-unknown-unknown/release/wasmi_host.wasm ../web/

cd ../web
node run-node.mjs            # exit 0 when run() is right and short fuel is cut off
python3 -m http.server 8769  # then open http://localhost:8769/
```

The page sets `window.__result`; `passed` is the verdict.

Building `wasmi-host` for wasm with `RUSTFLAGS="-C target-feature=+tail-call"`
makes wasmi choose its faster dispatch and the unlimited run finishes sooner,
but a run with fuel counted then overflows the engine's stack. The probe uses
the plain build.
