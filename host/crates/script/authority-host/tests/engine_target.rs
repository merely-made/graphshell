// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The `pulley` feature decides which backend this host's engines use, and nothing else does
//! (guest runtime plan D8).

#[test]
fn the_engine_is_pulley_exactly_when_the_feature_is_on() {
    let engine = wasmtime::Engine::new(&authority_host::engine_config()).unwrap();
    assert_eq!(engine.is_pulley(), cfg!(feature = "pulley"));
}
