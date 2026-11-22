// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The smallest Wasmtime host: take a module already compiled to Pulley
//! bytecode, instantiate it and call its `run` export. It exists to be built
//! for `wasm32-unknown-unknown`; see the README beside it.

use std::cell::RefCell;

use wasmtime::{Engine, Instance, Module, Store};

/// Wasmtime's embedded build asks its host for two slots of thread-local
/// storage. A wasm module here has one thread, so statics serve.
mod platform {
    use core::sync::atomic::{AtomicPtr, Ordering};

    const SLOTS: usize = 2;
    static TLS: [AtomicPtr<u8>; SLOTS] = [
        AtomicPtr::new(core::ptr::null_mut()),
        AtomicPtr::new(core::ptr::null_mut()),
    ];

    #[unsafe(no_mangle)]
    pub extern "C" fn wasmtime_tls_get(slot: usize) -> *mut u8 {
        TLS[slot].load(Ordering::Relaxed)
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn wasmtime_tls_set(slot: usize, ptr: *mut u8) {
        TLS[slot].store(ptr, Ordering::Relaxed);
    }
}

thread_local! {
    /// Why the last `run_precompiled` failed, for the caller to read back.
    static LAST_ERROR: RefCell<String> = const { RefCell::new(String::new()) };
}

/// The value `run_precompiled` returns when it could not run the module.
const FAILED: i32 = i32::MIN;

/// Space for the caller to write a module into. Never freed: one module per page.
#[unsafe(no_mangle)]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    Vec::<u8>::with_capacity(len).leak().as_mut_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn last_error_ptr() -> *const u8 {
    LAST_ERROR.with(|e| e.borrow().as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn last_error_len() -> usize {
    LAST_ERROR.with(|e| e.borrow().len())
}

fn run(bytes: &[u8]) -> Result<i32, String> {
    // No compiler is built in, so the engine's only target is Pulley.
    let engine = Engine::default();
    if !engine.is_pulley() {
        return Err("the engine's target is not Pulley".into());
    }
    let step = |what: &str, e: wasmtime::Error| format!("{what}: {e:?}");
    // SAFETY: the caller's contract on `run_precompiled`.
    let module =
        unsafe { Module::deserialize(&engine, bytes) }.map_err(|e| step("deserialize", e))?;
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[]).map_err(|e| step("instantiate", e))?;
    let run = instance
        .get_typed_func::<(), i32>(&mut store, "run")
        .map_err(|e| step("find run", e))?;
    run.call(&mut store, ()).map_err(|e| step("call run", e))
}

/// Runs the `run` export of a precompiled Pulley module and returns its
/// result, or [`FAILED`] with the reason left for `last_error_ptr`.
///
/// # Safety
/// `ptr` and `len` must describe readable bytes, and the bytes must come from
/// `Engine::precompile_module` on a matching Wasmtime: deserializing trusts them.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn run_precompiled(ptr: *const u8, len: usize) -> i32 {
    let bytes = unsafe { core::slice::from_raw_parts(ptr, len) };
    match run(bytes) {
        Ok(value) => value,
        Err(message) => {
            LAST_ERROR.with(|e| *e.borrow_mut() = message);
            FAILED
        },
    }
}
