// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Wasmtime as a wasm module, holding Graphshell's guests (guest runtime plan,
//! P7). It takes components already compiled to Pulley bytecode, serves the
//! projection guest from the fixture server and the document guests from a
//! fixture document, and answers in JSON. Built for `wasm32-unknown-unknown`;
//! see the README beside it. The page supplies only the two thread-local
//! functions below.

use std::cell::RefCell;

use wasmtime::Config;

#[path = "../../shared/config.rs"]
mod config;
#[path = "../../shared/document_host.rs"]
mod document_host;
#[path = "../../shared/driver.rs"]
mod driver;
#[path = "../../shared/projection_host.rs"]
mod projection_host;
#[path = "../../shared/stubs.rs"]
mod stubs;

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
    static RUNTIME: RefCell<Option<driver::Runtime>> = const { RefCell::new(None) };
    /// The last answer: JSON on success, the reason on failure.
    static ANSWER: RefCell<String> = const { RefCell::new(String::new()) };
}

/// Returned when a call could not be made; the reason is the answer.
const FAILED: i32 = -1;

fn answer(text: String) {
    ANSWER.with(|a| *a.borrow_mut() = text);
}

/// Runs `f` against the runtime and keeps its answer; `FAILED` with the
/// reason when it errors.
fn with_runtime(f: impl FnOnce(&mut driver::Runtime) -> wasmtime::Result<String>) -> i32 {
    RUNTIME.with(|r| {
        let mut r = r.borrow_mut();
        let Some(rt) = r.as_mut() else {
            answer("init was not called".into());
            return FAILED;
        };
        match f(rt) {
            Ok(text) => {
                answer(text);
                0
            },
            Err(error) => {
                answer(format!("{error:?}"));
                FAILED
            },
        }
    })
}

/// Space for the caller to write bytes into. Never freed: one set per page.
#[unsafe(no_mangle)]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    Vec::<u8>::with_capacity(len).leak().as_mut_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn answer_ptr() -> *const u8 {
    ANSWER.with(|a| a.borrow().as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn answer_len() -> usize {
    ANSWER.with(|a| a.borrow().len())
}

/// Builds the engines. Their settings are `config::embedded` and, for the
/// document guests, `config::guarded`: what the native tool compiled with.
#[unsafe(no_mangle)]
pub extern "C" fn init() -> i32 {
    let mut plain = Config::new();
    config::embedded(&mut plain);
    let mut guarded = Config::new();
    config::embedded(&mut guarded);
    config::guarded(&mut guarded);
    match driver::Runtime::new(&plain, &guarded) {
        Ok(rt) => {
            let pulley = rt.is_pulley();
            RUNTIME.with(|r| *r.borrow_mut() = Some(rt));
            answer(format!("{{\"pulley\":{pulley}}}"));
            0
        },
        Err(error) => {
            answer(format!("{error:?}"));
            FAILED
        },
    }
}

/// Deserializes a precompiled component into a guest slot (0 projection,
/// 1 document, 2 bomb).
///
/// # Safety
/// `ptr` and `len` must describe readable bytes written by `precompile_component`
/// on a matching Wasmtime and settings: deserializing trusts them.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn load(slot: u32, ptr: *const u8, len: usize) -> i32 {
    let bytes = unsafe { core::slice::from_raw_parts(ptr, len) };
    // SAFETY: the caller's contract.
    with_runtime(|rt| unsafe { rt.load(slot, bytes) }.map(|()| "{}".into()))
}

/// Links and instantiates the projection guest.
#[unsafe(no_mangle)]
pub extern "C" fn projection_start() -> i32 {
    with_runtime(|rt| rt.projection_start().map(|()| "{}".into()))
}

/// Runs the started projection guest; the answer is its transcript.
#[unsafe(no_mangle)]
pub extern "C" fn projection_run() -> i32 {
    with_runtime(|rt| rt.projection_run())
}

/// Runs one document turn from a JSON request.
///
/// # Safety
/// `ptr` and `len` must describe readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn turn(ptr: *const u8, len: usize) -> i32 {
    // SAFETY: the caller's contract.
    let bytes = unsafe { core::slice::from_raw_parts(ptr, len) };
    let request = String::from_utf8_lossy(bytes).into_owned();
    with_runtime(|rt| Ok(rt.turn(&request)))
}

/// Attaches a script that persists across `script_deliver` calls, from a JSON
/// request naming the guest and its limits.
///
/// # Safety
/// `ptr` and `len` must describe readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn script_attach(ptr: *const u8, len: usize) -> i32 {
    // SAFETY: the caller's contract.
    let bytes = unsafe { core::slice::from_raw_parts(ptr, len) };
    let request = String::from_utf8_lossy(bytes).into_owned();
    with_runtime(|rt| Ok(rt.script_attach(&request)))
}

/// One turn on the attached script; `kind` and `payload` are UTF-8 bytes.
///
/// # Safety
/// Both ranges must describe readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn script_deliver(
    kind: *const u8,
    kind_len: usize,
    payload: *const u8,
    payload_len: usize,
) -> i32 {
    // SAFETY: the caller's contract.
    let (kind, payload) = unsafe {
        (
            core::slice::from_raw_parts(kind, kind_len),
            core::slice::from_raw_parts(payload, payload_len),
        )
    };
    let (kind, payload) = (
        String::from_utf8_lossy(kind),
        String::from_utf8_lossy(payload),
    );
    with_runtime(|rt| Ok(rt.script_deliver(&kind, &payload)))
}

/// Drops the attached script.
#[unsafe(no_mangle)]
pub extern "C" fn script_detach() -> i32 {
    with_runtime(|rt| {
        rt.script_detach();
        Ok("{}".into())
    })
}
