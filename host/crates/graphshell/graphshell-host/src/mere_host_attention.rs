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
use muniment::{Codec, JsonCodec, WriteOp};

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

const STANDALONE: (&str, &str) = ("graphshell", "graphshell.mere");
const TURNSTONE: (&str, &str) = ("turnstone", "turnstone.mere");

impl<B: LegacyTransactionBackend + Clone> MereHost<B> {
    /// Carry the standalone viewer's saved layout and this device's focus
    /// into missing Turnstone view records. Only the resident calls this, on
    /// first open of an admitted local view under the owner's opt-in.
    pub async fn handoff_turnstone_view(&mut self) -> Result<(), MereHostError> {
        if self.is_observing()
            || self.graph_session.ensure_writable().is_err()
            || self.graph_session.manifest().trashed.is_some()
        {
            return Ok(());
        }

        let backend = self.slots.backend();
        let slot = self.attention_slot();
        let layout_marker = format!(
            "{slot}-turnstone/layout/{}",
            self.graph_session.id().as_uuid()
        );
        // Focus follows its device-local root, even across sibling sessions.
        let focus_marker = format!("{slot}-turnstone/focus");
        let prior = backend.get(&slot).await?;
        let mut attention: BTreeMap<String, AttentionState> = prior
            .as_deref()
            .map(JsonCodec::decode)
            .transpose()?
            .unwrap_or_default();
        let layout = self
            .view(STANDALONE.0, STANDALONE.1)
            .and_then(|view| view.strategy);
        let layout = match layout {
            Some(layout) if backend.get(&layout_marker).await?.is_none() => Some(layout),
            _ => None,
        };
        let focus = attention
            .get(&entry(STANDALONE.0, STANDALONE.1)?)
            .and_then(|state| state.focus.clone());
        let focus = match focus {
            Some(focus) if backend.get(&focus_marker).await?.is_none() => Some(focus),
            _ => None,
        };
        if layout.is_none() && focus.is_none() {
            self.bump_if(self.attention != attention);
            self.attention = attention;
            return Ok(());
        }

        // Compare the bytes read above in the same commit as the view entry.
        // A refused write cannot install half the handoff or mark it done.
        let mut extra = CommitExtra {
            expect: vec![(slot.clone(), prior)],
            ..CommitExtra::default()
        };
        if layout.is_some() {
            record_handoff(&mut extra, layout_marker);
        }
        if focus.is_some() {
            record_handoff(&mut extra, focus_marker);
        }
        let destination = ViewKey::new(TURNSTONE.0, TURNSTONE.1)?;
        let layout = layout.filter(|_| self.graph_session.view(&destination).is_none());
        let key = entry(TURNSTONE.0, TURNSTONE.1)?;
        if let std::collections::btree_map::Entry::Vacant(target) = attention.entry(key)
            && let Some(focus) = focus
        {
            target.insert(AttentionState {
                focus: Some(focus),
                ..Default::default()
            });
            extra.ops.push(WriteOp::Put {
                key: slot,
                value: JsonCodec::encode(&attention)?,
            });
        }

        let changed = layout.is_some() || self.attention != attention;
        let author = author_of(&self.selected_persona, TURNSTONE.0);
        self.graph_session
            .transact_change_with(
                wall_clock_now(),
                false,
                |_| extra,
                |candidate| {
                    if let Some(strategy) = layout {
                        let view = ViewIntent {
                            strategy: Some(strategy),
                            ..Default::default()
                        };
                        candidate.set_view_now(author, destination, encode_view(&view))?;
                    }
                    Ok(())
                },
            )
            .await?;
        self.attention = attention;
        self.bump_if(changed);
        Ok(())
    }
}

fn record_handoff(extra: &mut CommitExtra, marker: String) {
    extra.expect.push((marker.clone(), None));
    extra.ops.push(WriteOp::Put {
        key: marker,
        value: b"true".to_vec(),
    });
}
