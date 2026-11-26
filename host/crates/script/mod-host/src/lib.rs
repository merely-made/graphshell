// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The parser and viewer component hosts (mod projects MP7).
//!
//! A parser mod takes the fetched bytes and their media type and returns inker's
//! `EngineDocument`; a viewer mod takes the graph and returns a scene and a vector sheet.
//! Both worlds are pure (`log` and `caps` only). Containment is block-host's: epoch
//! interruption under a [`Watchdog`], `StoreLimits`, a component cap, payload caps, and a
//! trap that drops the instance (D20): a [`ParserHost`] or [`ViewerHost`] that returned a
//! trap says [`ParseFailure::Trapped`] or [`ViewFailure::Trapped`] and the caller discards it.
//!
//! [`ModKind`] names the three kinds of project (a project's `kind` in `mod.toml`), and
//! [`fixture`] holds the TOML fixture forms and runners for the parser and viewer worlds.

pub use block_host::{Watchdog, engine_config, guarded_engine};
pub use wasmtime;
use wasmtime::component::{Linker, ResourceTable};
use wasmtime::{Engine, Store, StoreLimits};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

pub mod fixture;
mod kind;
pub mod parser;
#[cfg(feature = "test-support")]
pub mod test_support;
pub mod viewer;

pub use kind::{ModKind, normalize_media_type};
pub use parser::{Diagnostic, ParseContext, ParseFailure, ParsedDocument, ParserHost, Severity};
pub use viewer::{ViewFailure, ViewedScene, ViewerHost};

/// Most bytes a guest component may be (the capsule cap).
pub const MAX_COMPONENT_BYTES: usize = block_host::MAX_COMPONENT_BYTES;
/// Most bytes of one viewer payload, the scene and the sheet each (the 1 MiB session cap).
pub const MAX_PAYLOAD_BYTES: usize = 1024 * 1024;
/// Most bytes a parser is handed, and most bytes of the document it returns (the smolweb
/// packs' reply cap).
pub const MAX_DOCUMENT_BYTES: usize = 16 * 1024 * 1024;
/// Most diagnostics kept from one parse; a guest that emits more is told so in the last one.
pub const MAX_DIAGNOSTICS: usize = 256;
/// Most guest `log` lines kept, and most characters of each.
const MAX_LOG_LINES: usize = 256;
const MAX_LOG_CHARS: usize = 1024;

/// What a guest instance may use: a memory ceiling and a wall-clock deadline in watchdog
/// ticks. The same shape as the document world's [`document_host::Quota`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub mem_bytes: usize,
    pub epoch_deadline_ticks: u64,
}

impl Default for Limits {
    fn default() -> Self {
        let q = document_host::Quota::default();
        Self {
            mem_bytes: q.mem_bytes,
            epoch_deadline_ticks: q.epoch_deadline_ticks,
        }
    }
}

impl From<document_host::Quota> for Limits {
    fn from(q: document_host::Quota) -> Self {
        Self {
            mem_bytes: q.mem_bytes,
            epoch_deadline_ticks: q.epoch_deadline_ticks,
        }
    }
}

/// The tick the watchdog bumps the epoch on; [`Limits::epoch_deadline_ticks`] counts these.
pub const WATCHDOG_TICK: std::time::Duration = std::time::Duration::from_millis(5);

/// Start a watchdog over `engine` on [`WATCHDOG_TICK`].
pub fn watchdog(engine: &Engine) -> Watchdog {
    Watchdog::start(engine.clone(), WATCHDOG_TICK)
}

/// One instance's store data: the log, the grant report, WASI and the limits.
pub(crate) struct GuestState {
    logs: Vec<String>,
    granted: Vec<String>,
    wasi: WasiCtx,
    table: ResourceTable,
    limits: StoreLimits,
}

impl GuestState {
    pub(crate) fn log(&mut self, message: String) {
        if self.logs.len() < MAX_LOG_LINES {
            self.logs.push(message.chars().take(MAX_LOG_CHARS).collect());
        }
    }

    pub(crate) fn granted(&self) -> Vec<String> {
        self.granted.clone()
    }
}

impl WasiView for GuestState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

/// A linker with the WASI floor; the world's own `log` and `caps` are added by its module.
pub(crate) fn base_linker(engine: &Engine) -> wasmtime::Result<Linker<GuestState>> {
    let mut linker: Linker<GuestState> = Linker::new(engine);
    wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
    Ok(linker)
}

/// A store for one instance, limits and first deadline armed.
pub(crate) fn new_store(engine: &Engine, granted: Vec<String>, limits: Limits) -> Store<GuestState> {
    let mut store = Store::new(
        engine,
        GuestState {
            logs: Vec::new(),
            granted,
            wasi: WasiCtxBuilder::new().build(),
            table: ResourceTable::new(),
            limits: wasmtime::StoreLimitsBuilder::new()
                .memory_size(limits.mem_bytes)
                .build(),
        },
    );
    store.limiter(|s| &mut s.limits);
    store.set_epoch_deadline(limits.epoch_deadline_ticks);
    store
}

/// Check the component's size, then pin it: a record that names a different hash refuses
/// it (D4); no record pins nothing.
pub(crate) fn admit_bytes(
    bytes: &[u8],
    record: Option<&document_host::mod_loader::InstallRecord>,
) -> wasmtime::Result<()> {
    if bytes.len() > MAX_COMPONENT_BYTES {
        wasmtime::bail!("component exceeds {MAX_COMPONENT_BYTES} bytes");
    }
    if let Some(record) = record {
        record
            .verify_component(bytes)
            .map_err(|refusal| wasmtime::Error::msg(refusal.to_string()))?;
    }
    Ok(())
}

/// The first line of a trap, which names the cause; the backtrace stays out of the status.
pub(crate) fn first_line(trap: &wasmtime::Error) -> String {
    let full = format!("{trap:#}");
    full.lines().next().unwrap_or("trap").to_string()
}
