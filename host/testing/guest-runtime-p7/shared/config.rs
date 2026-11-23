// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The engine settings the native tool that writes bytecode and the wasm host
//! that reads it must share, so that a deserialize finds them compatible
//! (guest runtime plan, P7).

use wasmtime::Config;

/// The settings of an engine with no virtual memory and no signal handlers,
/// which is what Wasmtime is when it is itself built for wasm32 (as P2).
pub fn embedded(config: &mut Config) {
    config.signals_based_traps(false);
    config.memory_init_cow(false);
    config.memory_reservation(0);
    config.memory_guard_size(0);
    config.memory_reservation_for_growth(0);
}

/// Fuel and epoch interruption change the code a guest compiles to, so the
/// document guests are compiled with both and a run turns each up or down by
/// its budget.
pub fn guarded(config: &mut Config) {
    config.consume_fuel(true);
    config.epoch_interruption(true);
}
