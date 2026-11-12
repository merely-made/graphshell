// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The attention intent: what a person is looking at, kept on the device
//! (engine split D133, contract C7). It writes no journal entry and no view
//! entry, so it is accepted on a read-only archive without forking it.

use incipit::view::AttentionState;
use serde::{Deserialize, Serialize};

/// Keep this device's attention for one of the application's views.
pub const SET_ATTENTION_INTENT: &str = "mere.session.attention";
/// Schema of [`SetAttentionV1`].
pub const SET_ATTENTION_SCHEMA: &str = "mere.session.attention/v1";

/// Payload of [`SET_ATTENTION_INTENT`]: one of the application's views, by
/// name.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetAttentionV1 {
    pub schema: String,
    pub view: String,
    pub state: AttentionState,
}

impl SetAttentionV1 {
    pub fn new(view: impl Into<String>, state: AttentionState) -> Self {
        Self {
            schema: SET_ATTENTION_SCHEMA.to_string(),
            view: view.into(),
            state,
        }
    }
}
