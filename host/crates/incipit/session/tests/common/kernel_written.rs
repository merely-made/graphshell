// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Stores mere-kernel wrote, frozen when Mere retired it (engine split lane 4).
//!
//! `tests/fixtures/kernel_written/` holds what pandect wrote over mere-kernel
//! 0.0.3: each `.bin` is one store's every key and value, the values
//! verbatim, as postcard behind [`STORE_FIXTURE`]; each `expected.json` holds
//! what the kernel read back from them, or the ids a test opens. The one-off
//! generators are `Code/testing/engine-split/lane4_fixture_gen.rs`, run in
//! this crate's `cross_read` test at Mere `2c1199652`, and
//! `lane4_fixture_gen_merge.rs` (`merge_source`), run at Mere `675fd8111`,
//! whose kernel and sessions are `2c1199652`'s.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use muniment::{Backend, MemoryBackend, WriteOp};
use serde_json::Value;

/// The header every frozen store carries; the rest is postcard.
pub const STORE_FIXTURE: &[u8] = b"incipit-session.kernel-store-fixture/v1\n";

fn path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/kernel_written")
        .join(name)
}

/// Every key and value a frozen store holds.
pub fn held(name: &str) -> BTreeMap<String, Vec<u8>> {
    let bytes = std::fs::read(path(name)).unwrap();
    let body = bytes
        .strip_prefix(STORE_FIXTURE)
        .unwrap_or_else(|| panic!("{name} lacks the store fixture header"));
    postcard::from_bytes(body).unwrap()
}

/// A memory store holding exactly what a frozen store holds.
pub async fn store(name: &str) -> MemoryBackend {
    let backend = MemoryBackend::new();
    let ops: Vec<WriteOp> = held(name)
        .into_iter()
        .map(|(key, value)| WriteOp::Put { key, value })
        .collect();
    backend.apply(&ops).await.unwrap();
    backend
}

/// What the kernel read back from a case's stores.
pub fn expected(case: &str) -> Value {
    serde_json::from_slice(&std::fs::read(path(&format!("{case}/expected.json"))).unwrap())
        .unwrap()
}

/// A uuid an expectation names.
pub fn uuid_of(value: &Value) -> uuid::Uuid {
    value.as_str().unwrap().parse().unwrap()
}
