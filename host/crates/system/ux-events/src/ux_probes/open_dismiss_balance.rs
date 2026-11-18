// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Open/Dismiss balance probe — every Opened eventually gets Dismissed.

use std::collections::{HashMap, hash_map::Entry};
use std::sync::Mutex;

use crate::ux_observability::{EventAddress, SurfaceId, UxEvent};

use super::{ProbeFailure, UxProbe};

/// Asserts that every `SurfaceOpened` for a given surface gets
/// matched by a `SurfaceDismissed` before the same surface is
/// re-opened. The surface kind and optional captured address form the key.
/// Keep the address used at open until dismiss. Address-less events use the
/// surface kind on its own.
///
/// This probe *flags on re-open*, not on stream end (the stream
/// never explicitly ends). To check terminal balance, call
/// [`Self::pending_opens`] after running messages: any non-zero
/// count means a surface is still open.
pub struct OpenDismissBalanceProbe {
    open_counts: Mutex<HashMap<(SurfaceId, Option<EventAddress>), u32>>,
    failures: Mutex<Vec<ProbeFailure>>,
}

impl OpenDismissBalanceProbe {
    pub fn new() -> Self {
        Self {
            open_counts: Mutex::new(HashMap::new()),
            failures: Mutex::new(Vec::new()),
        }
    }

    /// Snapshot the current per-surface open count (Opens minus
    /// Dismisses). A non-zero entry means that surface is currently
    /// open — useful for terminal-balance assertions in tests.
    pub fn pending_opens(&self) -> HashMap<SurfaceId, u32> {
        let mut pending = HashMap::new();
        for (&(surface, _), &count) in self.open_counts.lock().unwrap().iter() {
            *pending.entry(surface).or_insert(0) += count;
        }
        pending
    }
}

impl Default for OpenDismissBalanceProbe {
    fn default() -> Self {
        Self::new()
    }
}

impl UxProbe for OpenDismissBalanceProbe {
    fn name(&self) -> &'static str {
        "open_dismiss_balance"
    }

    fn observe(&self, event: &UxEvent) {
        match event {
            UxEvent::SurfaceOpened { surface, address } => {
                let mut counts = self.open_counts.lock().unwrap();
                let entry = counts.entry((*surface, *address)).or_insert(0);
                if *entry > 0 {
                    self.failures.lock().unwrap().push(ProbeFailure {
                        probe_name: self.name(),
                        description: format!(
                            "{:?} opened again while previous open is unmatched",
                            surface
                        ),
                        triggering_event: event.clone(),
                    });
                }
                *entry += 1;
            },
            UxEvent::SurfaceDismissed {
                surface, address, ..
            } => {
                let mut counts = self.open_counts.lock().unwrap();
                match counts.entry((*surface, *address)) {
                    Entry::Occupied(mut entry) if *entry.get() > 1 => *entry.get_mut() -= 1,
                    Entry::Occupied(entry) => {
                        entry.remove();
                    },
                    Entry::Vacant(_) => {
                        self.failures.lock().unwrap().push(ProbeFailure {
                            probe_name: self.name(),
                            description: format!("{:?} dismissed without a matching open", surface),
                            triggering_event: event.clone(),
                        });
                    },
                }
            },
            _ => {},
        }
    }

    fn drain_failures(&self) -> Vec<ProbeFailure> {
        std::mem::take(&mut *self.failures.lock().unwrap())
    }
}
