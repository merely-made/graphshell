// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! A plain wasm module run by wasmi: instantiate it with no imports and call
//! its `run` export, optionally under a fuel limit. The same code is the
//! native library and, through the exports at the bottom, a module a page
//! loads. See the README beside it.

use std::cell::RefCell;

use wasmi::{Config, Engine, Linker, Module, Store};

/// Run `run` from the module in `bytes`. With `fuel`, the run is cut off once
/// that much is spent, which is an error.
pub fn run(bytes: &[u8], fuel: Option<u64>) -> Result<i32, String> {
    let step = |what: &str, e: &dyn std::fmt::Display| format!("{what}: {e}");
    let mut config = Config::default();
    config.consume_fuel(fuel.is_some());
    let engine = Engine::new(&config);
    let module = Module::new(&engine, bytes).map_err(|e| step("compile", &e))?;
    let mut store = Store::new(&engine, ());
    if let Some(fuel) = fuel {
        store.set_fuel(fuel).map_err(|e| step("set fuel", &e))?;
    }
    let instance = Linker::<()>::new(&engine)
        .instantiate_and_start(&mut store, &module)
        .map_err(|e| step("instantiate", &e))?;
    let run = instance
        .get_typed_func::<(), i32>(&store, "run")
        .map_err(|e| step("find run", &e))?;
    run.call(&mut store, ()).map_err(|e| step("call run", &e))
}

thread_local! {
    /// Why the last exported call failed, for the caller to read back.
    static LAST_ERROR: RefCell<String> = const { RefCell::new(String::new()) };
}

/// The value the exported calls return when the module did not run to its end.
pub const FAILED: i32 = i32::MIN;

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

/// Runs the module at `ptr`. `fuel` of 0 means no limit. Returns `run`'s
/// result, or [`FAILED`] with the reason left for `last_error_ptr`.
///
/// # Safety
/// `ptr` and `len` must describe readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn run_module(ptr: *const u8, len: usize, fuel: u64) -> i32 {
    let bytes = unsafe { core::slice::from_raw_parts(ptr, len) };
    match run(bytes, (fuel != 0).then_some(fuel)) {
        Ok(value) => value,
        Err(message) => {
            LAST_ERROR.with(|e| *e.borrow_mut() = message);
            FAILED
        },
    }
}
