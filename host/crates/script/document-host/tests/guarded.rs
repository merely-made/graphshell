// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! P2.2 verification: a misbehaving guest is contained. An infinite loop is
//! cancelled by the epoch deadline; an unbounded allocation is denied by
//! StoreLimits; a well-behaved turn completes. In every case the host thread
//! survives (the test continues).
//!
//! Build the bomb guest first:
//!   cd guest-bomb && cargo build --target wasm32-wasip2 --release

use std::path::PathBuf;

use document_host::Guarded;

fn bomb_wasm() -> PathBuf {
    let p = std::env::var("DOC_HOST_BOMB_WASM").unwrap_or_else(|_| {
        "guest-bomb/target/wasm32-wasip2/release/document_core_guest_bomb.wasm".to_string()
    });
    let path = PathBuf::from(&p);
    assert!(
        path.exists(),
        "bomb guest missing at {p}; build it: cd guest-bomb && cargo build --target wasm32-wasip2 --release"
    );
    path
}

#[tokio::test(flavor = "current_thread")]
async fn epoch_cancels_a_runaway_loop() {
    // Generous memory, a 1-tick epoch deadline → the infinite loop traps fast.
    let r = document_host::run_guarded(&bomb_wasm(), "loop", 256 * 1024 * 1024, 1)
        .await
        .expect("run_guarded");
    assert!(
        matches!(r, Guarded::Trapped(_)),
        "runaway loop should be trapped, got {r:?}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn store_limits_deny_a_memory_bomb() {
    // A 32 MiB cap, a far-off epoch deadline → the allocation bomb hits the cap first.
    let r = document_host::run_guarded(&bomb_wasm(), "grow", 32 * 1024 * 1024, 100_000)
        .await
        .expect("run_guarded");
    assert!(
        matches!(r, Guarded::Trapped(_)),
        "memory bomb should be denied/trapped, got {r:?}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn a_well_behaved_turn_completes() {
    let r = document_host::run_guarded(&bomb_wasm(), "noop", 256 * 1024 * 1024, 100_000)
        .await
        .expect("run_guarded");
    assert!(
        matches!(r, Guarded::Completed),
        "a benign turn should complete, got {r:?}"
    );
}

/// A reading for the guest runtime plan, not a check: which backend ran, and how long the
/// allocation bomb takes to reach a 256 MiB cap. Run it with `--ignored --nocapture`.
#[tokio::test(flavor = "current_thread")]
#[ignore = "a timing reading for the guest runtime plan"]
async fn timing_of_the_allocation_bomb() {
    const ROUNDS: usize = 7;
    const CAP_BYTES: usize = 256 * 1024 * 1024;

    let mut ms = Vec::new();
    for _ in 0..ROUNDS {
        let started = std::time::Instant::now();
        let r = document_host::run_guarded(&bomb_wasm(), "grow", CAP_BYTES, 100_000)
            .await
            .expect("run_guarded");
        ms.push(started.elapsed().as_secs_f64() * 1e3);
        assert!(
            matches!(r, Guarded::Trapped(_)),
            "the bomb should trap, got {r:?}"
        );
    }
    ms.sort_by(f64::total_cmp);
    println!(
        "bomb timing: backend={} rounds={ROUNDS} cap_mib=256 grow_to_trap_ms={:.2} (median, compile included)",
        if wasmtime::Engine::default().is_pulley() {
            "pulley"
        } else {
            "compiler"
        },
        ms[ROUNDS / 2],
    );
}
