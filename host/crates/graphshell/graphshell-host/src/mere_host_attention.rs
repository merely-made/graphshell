// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! A device's attention (camera, focus, mirror mode) for the views of a graph,
//! kept in one slot of the host's backend, keyed by the graph's root (engine
//! split D133, contract C7). It never enters the journal or the view log, so
//! it never replicates, and a read-only archive takes it without forking.
use super::*;
use incipit::view::{AttentionState, VIEW_INTENT_SCHEMA_V1};

/// A view state as a [`ViewIntent`]: the current schema, or the one before C7,
/// whose camera, focus and mirror keys are dropped.
pub(super) fn decode_view(state: &ViewState) -> Option<ViewIntent> {
    state
        .decode(VIEW_INTENT_SCHEMA)
        .or_else(|| state.decode(VIEW_INTENT_SCHEMA_V1))
}

fn slot_key(root_graph: uuid::Uuid) -> String {
    format!("graphs/{root_graph}/views/v1/attention")
}

fn entry(via: &str, view: &str) -> Result<String, MereHostError> {
    ViewKey::new(via, view)?;
    Ok(format!("{via}/{view}"))
}

impl<B: Backend> MereHost<B> {
    /// This device's attention for `via`'s view `view`.
    pub fn attention(&self, via: &str, view: &str) -> Option<AttentionState> {
        self.attention.get(&entry(via, view).ok()?).cloned()
    }

    /// `via`'s attention for the application's own views, by view name.
    pub(super) fn attention_of_app(&self) -> BTreeMap<String, AttentionState> {
        let prefix = format!("{}/", self.via);
        self.attention
            .iter()
            .filter_map(|(key, state)| Some((key.strip_prefix(&prefix)?.to_owned(), state.clone())))
            .collect()
    }
}

impl<B: Backend + Clone> MereHost<B> {
    fn attention_slot(&self) -> String {
        slot_key(*self.graph_session.manifest().root_graph_id.as_uuid())
    }

    /// Read this device's attention for the graph; none stored is none.
    pub async fn load_attention(&mut self) -> Result<(), MereHostError> {
        self.attention = self
            .slots
            .load(&self.attention_slot())
            .await?
            .unwrap_or_default();
        Ok(())
    }

    /// Keep this device's attention for one of `via`'s views, and store it.
    /// It writes no journal entry and no view entry, so it is allowed on a
    /// read-only session; the projection moves on so a reader sees it.
    pub async fn set_attention(
        &mut self,
        via: &str,
        view: &str,
        state: AttentionState,
    ) -> Result<(), MereHostError> {
        self.ensure_storing()?;
        let key = entry(via, view)?;
        let mut next = self.attention.clone();
        if state.is_empty() {
            next.remove(&key);
        } else {
            next.insert(key, state);
        }
        self.slots.save(&self.attention_slot(), &next).await?;
        self.attention = next;
        self.projection_revision = self.projection_revision.wrapping_add(1);
        Ok(())
    }
}
