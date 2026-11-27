// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! pandect's product-codicil wire fixture (Mere 675fd8111,
//! `tests/pre_p4_wire_fixtures.rs`): the fixture holds a kernel snapshot, so it
//! decodes over incipit-graph (engine split D272). Written by the code before
//! the P4 sidecar moves, it decodes strictly and re-encodes to the same bytes.

use kernel::graph::Graph;

#[test]
fn a_product_codicil_written_before_the_change_re_encodes_unchanged() {
    let bytes: &[u8] = include_bytes!("fixtures/pre_p4_product_codicil_with_scene.json");
    let codicil = pandect::product_codicil::decode_profiled_codicil_with::<Graph>(bytes)
        .expect("the old codicil decodes and materializes");
    let scene = codicil
        .scene
        .as_ref()
        .expect("the codicil carries its scene");
    assert_eq!(scene.name, "Codicil scene");
    assert_eq!(
        serde_json::to_vec_pretty(&codicil).expect("re-encodes"),
        bytes,
        "the codicil bytes do not move"
    );
}
