// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! A plain wasm module run by Wasmer: instantiate it with no imports and call
//! its `run` export. The same code is the native library and, through
//! `run_module`, what a page calls. See the README beside it.

use wasmer::{Instance, Module, Store, imports};

/// Run `run` from the module in `bytes`.
pub fn run(bytes: &[u8]) -> Result<i32, String> {
    let step = |what: &str, e: &dyn std::fmt::Display| format!("{what}: {e}");
    let mut store = Store::default();
    let module = Module::new(&store, bytes).map_err(|e| step("compile", &e))?;
    let instance =
        Instance::new(&mut store, &module, &imports! {}).map_err(|e| step("instantiate", &e))?;
    let run = instance
        .exports
        .get_typed_function::<(), i32>(&store, "run")
        .map_err(|e| step("find run", &e))?;
    run.call(&mut store).map_err(|e| step("call run", &e))
}

/// What a page calls: the module's result, or the reason it did not run.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn run_module(bytes: &[u8]) -> Result<i32, String> {
    run(bytes)
}
