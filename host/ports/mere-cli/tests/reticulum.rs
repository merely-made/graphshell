// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! An invited reader over Reticulum, on a loopback TCP interface.
#![cfg(all(feature = "reticulum", feature = "test-identity"))]

mod common;

#[test]
fn reticulum_invitation_limits_access_across_repeated_queries() {
    let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let listen = format!("127.0.0.1:{port}");
    common::invited_reader("reticulum", &["--listen", &listen], &[]);
}
