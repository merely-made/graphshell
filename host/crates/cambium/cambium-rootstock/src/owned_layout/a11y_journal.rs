// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! What the layout did between two accessibility publications (table plan
//! D76): the retained batches' mutations and restyled elements, or that a
//! full layout ran. Kept only while an accessibility session consumes it.
use std::collections::HashSet;

use layout_dom_api::DomMutation;

use super::OwnedLayout;
use crate::NodeId;

pub(crate) struct A11yJournal {
    /// DOM mutations drained before the journal began.
    start: u64,
    pub mutations: Vec<DomMutation<NodeId>>,
    /// The union of each batch's restyled set; `None` once any batch
    /// restyled the whole document.
    pub restyled: Option<HashSet<NodeId>>,
    /// A full layout ran, so nothing retained can be trusted.
    pub full: bool,
}

impl A11yJournal {
    /// Whether the journal holds every mutation drained since it began, and
    /// none are still pending.
    pub(crate) fn is_complete(&self, drained: u64, pending: usize) -> bool {
        pending == 0 && self.start.saturating_add(self.mutations.len() as u64) == drained
    }
}

impl OwnedLayout {
    /// Start recording, replacing any journal already running.
    pub(crate) fn begin_a11y_journal(&mut self, drained: u64) {
        self.a11y_journal = Some(A11yJournal {
            start: drained,
            mutations: Vec::new(),
            restyled: Some(HashSet::new()),
            full: false,
        });
    }

    pub(crate) fn take_a11y_journal(&mut self) -> Option<A11yJournal> {
        self.a11y_journal.take()
    }

    pub(crate) fn end_a11y_journal(&mut self) {
        self.a11y_journal = None;
    }

    /// A retained batch: its mutations and the style session's restyled set.
    pub(super) fn journal_batch(&mut self, mutations: &[DomMutation<NodeId>]) {
        let Some(journal) = self.a11y_journal.as_mut() else {
            return;
        };
        journal.mutations.extend_from_slice(mutations);
        match (journal.restyled.as_mut(), self.style.restyled_elements()) {
            (Some(all), Some(batch)) => all.extend(batch.iter().copied()),
            _ => journal.restyled = None,
        }
    }

    pub(crate) fn journal_full_layout(&mut self) {
        if let Some(journal) = self.a11y_journal.as_mut() {
            journal.full = true;
        }
    }
}
