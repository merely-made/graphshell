// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! An invited reader over iroh.
#![cfg(all(feature = "iroh", feature = "test-identity"))]

mod common;

#[test]
fn iroh_invitation_limits_access_across_repeated_queries() {
    common::invited_reader("iroh", &[], &[]);
}
