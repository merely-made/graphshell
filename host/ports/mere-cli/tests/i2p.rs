// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! An invited reader over I2P, through murm's scripted SAM bridge: no router
//! and no network, but the CLI's own SAM sessions and Noise streams.
#![cfg(all(feature = "i2p", feature = "test-identity"))]

mod common;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn i2p_invitation_limits_access_across_repeated_queries() {
    let fixture = transport::i2p::fixture::SamFixture::start().await;
    let sam = fixture.bridge().addr().to_string();
    tokio::task::spawn_blocking(move || {
        common::invited_reader("i2p", &["--sam", &sam], &["--sam", &sam]);
    })
    .await
    .unwrap();
    fixture.shutdown().await;
}
