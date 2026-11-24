// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Lua command packs, registered and run (command packs CP2).
//!
//! [`PackRunner`] is the registry's [`document_host::LuaPackRuntime`]: it puts
//! an admitted pack's commands in the host's `CommandSet` and runs one through
//! `script-didge`. A run returns hest `Invocation`s, requests and never
//! execution, after each is checked against the pack's grants (D2, D3).

mod context;
mod runner;

pub use context::{CONTEXT_VERSION, CommandContext, Focus};
pub use runner::{PackLimits, PackRunner, Refusal};

#[cfg(test)]
mod tests;
