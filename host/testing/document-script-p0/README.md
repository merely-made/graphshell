# document-script-p0

Probe for Mere's DocumentScript substrate plan (2026-06-21, archived)
(§6 P0, plus the §10.3 transaction contract and §10.8). Drives the
`document-core` WIT world end to end across a real WebAssembly Component Model
boundary, synchronously, on released Wasmtime 45.

Origin: Mere's former `crates/probes/document-script-p0`, a local probe that was
never tracked. Standalone: `guest/` and `host/` are each their own workspace root
with their own `Cargo.lock`, outside the host workspace, and use the repository's
toolchain. Both depend only on crates.io (`wit-bindgen`, `wasmtime`,
`wasmtime-wasi`).

The library this probe led to is `host/crates/script/document-host`, which carries
the same world forward with a live genet `ScriptedDom`, capability grants,
quotas, and a `net` import.

## Layout

```text
wit/world.wit   the mere:script@0.1.0 contract (document-core world)
guest/          direct-Rust component, cdylib -> wasm32-wasip2
host/           wasmtime 45 runner: links WASI + log + document-host, owns the document
```

## The contract it exercises

- **Per turn**: the host delivers `handle-event(event) -> result<apply-batch,
  turn-error>`; the script pulls the snapshot it needs through the `inspect`
  host import, re-entrantly, scoped to `whole-document` or `subtree(node-id)`.
  `activate` and `deactivate` bracket the loop.
- **Node identity**: each `view-node` carries a host-assigned `node-id` plus a
  `content-hash` used for change detection, not addressing. Mutations are
  `set-text` / `remove` / `insert-before` / `append-child`, targeted by id with
  id-relative anchors.
- **Atomic apply**: an `apply-batch` cites `expected-revision`; the host applies
  all or nothing. `revision-conflict(u64)` carries the current revision,
  `unknown-node(node-id)` is reported before any change lands, `refused(string)`
  is the guest declining.
- **Capability linking**: the guest imports `log` and `document-host` from
  `mere:script`, and the host links exactly those two plus the WASI floor the
  `std` guest needs. Nothing else in `mere:script` is reachable.

## Run

```sh
cd guest && cargo build --target wasm32-wasip2 --release && cd ..
cd host && cargo run
```

The host loads `../guest/target/wasm32-wasip2/release/*.wasm` by default and
drives eight turns: `set`, `append`, `insert`, `subtree`, `remove`, `stale`,
`bad-id`, `frobnicate`. Four mutations apply (revision 0 to 4), the scoped
`subtree` inspect is a no-op, the stale revision and unknown id are rejected by
the host, and `frobnicate` is declined by the guest. The final document is a
root with three paragraphs.

## Findings

- **Toolchain**: wasmtime 45.0.2 requires rustc >= 1.93.0. This probe pinned
  1.93.0 locally when the workspace default was 1.92.0; the repository's own
  toolchain is now well past it, so the local pin is dropped.
- **Capability minimality costs a build mode**: a `std` guest on `wasm32-wasip2`
  imports the whole WASI world, so the host must grant WASI to instantiate at
  all. A component importing only the application capability needs a `no_std`
  build plus a hand-supplied allocator and mem / alloc-error intrinsics.
- **Instantiation fails when an imported interface is not linked**, observed
  with `wasi:io/error` before WASI was granted. Unimported means unreachable is
  a runtime mechanism, not an aspiration.

## Not covered here

- Async (`events: stream<event>`, `apply: async`).
- Host-side conflict detection in a real `apply()` seam; the guest emits only
  the conflict shape.
- The interpreter-component lane (Nova / Rune) and the `no_std` minimal-cap
  guest.
