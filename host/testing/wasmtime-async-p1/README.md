# wasmtime-async-p1

Wasmtime 45 component-model-async maturity probe: can a host-fulfilled async
import suspend and resume a component call (the DocumentScript suspension
question). `guest/` builds to a component on `wasm32-wasip2`; `host/` runs it.

```sh
cd guest && cargo build --target wasm32-wasip2 --release && cd ..
cd host && cargo run
```

Origin: Mere's former `crates/probes/wasmtime-async-p1`, a local probe that was
never tracked. `guest/` and `host/` are each their own workspace root with their
own `Cargo.lock`, outside the host workspace.
