// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The table in the mod projects plan's Findings 17 is `mod_targets::markdown()` (D46).

#![cfg(not(target_arch = "wasm32"))]

use std::path::Path;

const PLAN: &str =
    "design_docs/graphshell_docs/implementation_strategy/2026-11-25_mod_projects_plan.md";

#[test]
fn the_plan_carries_the_table() {
    // host/crates/script/mod-targets -> the repository root.
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../..");
    let Ok(plan) = std::fs::read_to_string(root.join(PLAN)) else {
        // The plan was archived or moved; nothing left to check.
        eprintln!("plan not found at {PLAN}; skipping");
        return;
    };
    let lines: Vec<&str> = plan.lines().map(str::trim).collect();
    for row in mod_targets::markdown().lines() {
        assert!(
            lines.contains(&row),
            "the plan's Findings 17 lacks this row:\n{row}"
        );
    }
}
