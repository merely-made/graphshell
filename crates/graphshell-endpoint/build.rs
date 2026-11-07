// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The `mere:script` WIT world lives once, at `wit/world.wit` in this package.
//!
//! This script publishes the directory holding it as Cargo `links` metadata
//! (`DEP_GRAPHSHELL_ENDPOINT_WIT` in a dependent's build script), so a host that
//! binds other worlds of the same package can read the one copy instead of
//! keeping its own. It also binds `projection-session` when the `wit` feature
//! is on (capability door D18).

fn main() {
    let wit_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("wit");
    println!("cargo::metadata=wit={}", wit_dir.display());

    #[cfg(feature = "wit")]
    {
        let wit = wit_dir.join("world.wit");
        println!("cargo::rerun-if-changed={}", wit.display());
        let src =
            script_witgen::generate(&wit, "projection-session").expect("projection-session binds");
        let out =
            std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("projection_session.rs");
        std::fs::write(out, src).unwrap();
    }
}
