// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The browser build's refusals, run on wasm32 under Node (D46):
//! `cargo test -p mod-targets --target wasm32-unknown-unknown --test wasm`.
//! Natively this file compiles to no tests; the unit tests in `src/lib.rs` cover desktop.

use mod_targets::{Admission, Kind, Target, admit, refusal};
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
fn this_build_is_the_browser() {
    // A positive control: this file only runs on wasm32, so the build must say browser.
    assert_eq!(Target::current(), Target::Browser);
}

#[wasm_bindgen_test]
fn browser_refuses_native_and_cwasm_with_a_reason() {
    for (kind, word) in [
        (Kind::Cwasm, "CPU"),
        (Kind::Native, "compiled into the binary"),
    ] {
        assert!(
            matches!(admit(kind, Target::Browser), Admission::Refused(_)),
            "{kind:?}"
        );
        let msg = refusal(kind, Target::Browser).expect("refused");
        assert!(msg.contains(word), "{msg}");
    }
}

#[wasm_bindgen_test]
fn browser_does_not_load_wasm_components_yet() {
    for kind in [Kind::DocumentScript, Kind::Parser, Kind::Viewer] {
        let a = admit(kind, Target::Browser);
        assert!(matches!(a, Admission::Planned(_)), "{kind:?}");
        assert!(!a.loads());
        let msg = refusal(kind, Target::Browser).expect("not loaded");
        assert!(
            msg.contains("no Wasm engine in the browser build yet"),
            "{msg}"
        );
    }
}

#[wasm_bindgen_test]
fn browser_allows_a_lua_pack() {
    assert!(admit(Kind::LuaPack, Target::Browser).loads());
}

#[wasm_bindgen_test]
fn desktop_loads_every_kind() {
    for kind in Kind::ALL {
        assert!(admit(kind, Target::Desktop).loads(), "{kind:?}");
    }
}
