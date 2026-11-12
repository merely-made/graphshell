//! Async host for the Wasmtime-45 component-model-async maturity probe.
//!
//! Determines empirically whether Wasmtime 45's "very incomplete" async actually
//! supports the one shape DocumentScript suspension needs: a host-fulfilled async
//! import whose future stays pending across a yield, suspending and resuming the
//! component call. If `RESULT:` prints with both fetches resolved and the host's
//! "pending → resolving" interleaving is visible, suspension works on 45.

use wasmtime::component::{Component, HasSelf, Linker, ResourceTable};
use wasmtime::{Engine, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

wasmtime::component::bindgen!({
    path: "../wit",
    world: "async-probe",
    imports: { default: async },
    exports: { default: async },
});

use crate::probe::suspend::host_io;

struct H {
    wasi: WasiCtx,
    table: ResourceTable,
}

impl WasiView for H {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView { ctx: &mut self.wasi, table: &mut self.table }
    }
}

impl host_io::Host for H {
    async fn fetch(&mut self, req: String) -> String {
        println!("  host: fetch('{req}') -> pending, yielding the executor...");
        // The suspension window: the future is not ready on first poll. If WT45's
        // async works, the component call parks here and the host stays live.
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        println!("  host: fetch('{req}') -> resolving");
        format!("resp:{req}")
    }
}

#[tokio::main]
async fn main() -> wasmtime::Result<()> {
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        "../guest/target/wasm32-wasip2/release/wt_async_guest.wasm".to_string()
    });
    println!("loading component: {path}");

    // Async is always available in wasmtime 45 (the old `Config::async_support`
    // is a deprecated no-op); component-model is on by default. No special config.
    let engine = Engine::default();
    let component = Component::from_file(&engine, &path)?;

    let mut linker = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
    host_io::add_to_linker::<H, HasSelf<H>>(&mut linker, |s| s)?;

    let mut store = Store::new(
        &engine,
        H { wasi: WasiCtxBuilder::new().build(), table: ResourceTable::new() },
    );

    let bindings = AsyncProbe::instantiate_async(&mut store, &component, &linker).await?;
    println!("instantiated; calling run(\"hello\") on the async store...");
    let out = bindings.call_run(&mut store, "hello").await?;
    println!("RESULT: {out}");
    Ok(())
}
