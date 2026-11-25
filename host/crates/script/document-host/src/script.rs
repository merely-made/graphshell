// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Quota, turn outcome, and the DocumentScript run loop.

use super::*;
use crate::mod_loader::InstallRecord;

/// Default per-turn `net.fetch` budget (§A6). Generous for a real turn (a handful of
/// API calls), a hard ceiling on a fetch flood. Per *turn*, reset each `handle-event`,
/// so total egress is bounded by this times the host-paced turn rate.
pub(crate) const DEFAULT_MAX_FETCHES_PER_TURN: u32 = 32;

/// Per-turn resource quota for a [`DocumentScript`] (§11.7-6: fixed constants for
/// P2). `mem_bytes` caps the instance's linear memory; `epoch_deadline_ticks` is
/// how many 5ms watchdog ticks a single guest call may run before it is trapped;
/// `max_fetches_per_turn` caps `net.fetch` calls within one turn (§A6).
#[derive(Clone, Copy, Debug)]
pub struct Quota {
    pub mem_bytes: usize,
    pub epoch_deadline_ticks: u64,
    pub max_fetches_per_turn: u32,
}

impl Default for Quota {
    fn default() -> Self {
        // ~64 MiB and ~1s wall (200 * 5ms): generous for a turn, fatal to a runaway.
        Self {
            mem_bytes: 64 * 1024 * 1024,
            epoch_deadline_ticks: 200,
            max_fetches_per_turn: DEFAULT_MAX_FETCHES_PER_TURN,
        }
    }
}

/// The outcome of delivering one event to a [`DocumentScript`]. The script's batch
/// (if any) has already been applied to the live DOM; this reports what happened
/// without leaking the WIT types to the caller.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TurnOutcome {
    /// The batch applied (or was an empty no-op); carries the resulting revision.
    Applied(u64),
    /// Optimistic-concurrency conflict: the script cited a stale revision. Carries
    /// the current revision to rebase against.
    Conflict(u64),
    /// A mutation referenced an unknown / out-of-scope node id; nothing applied.
    UnknownNode(u64),
    /// The script declined the event, or the host refused the batch. Carries why.
    Refused(String),
}

/// Run `fut` to completion under an epoch watchdog: a thread bumps `engine`'s epoch
/// every 5ms so a guest call that exceeds its `set_epoch_deadline` is trapped. The
/// thread is stopped and joined before returning, so nothing spins between turns.
pub(crate) fn guarded_block_on<T>(
    engine: Engine,
    fut: impl std::future::Future<Output = wasmtime::Result<T>>,
) -> wasmtime::Result<T> {
    let stop = Arc::new(AtomicBool::new(false));
    let watch_stop = stop.clone();
    let watchdog = std::thread::spawn(move || {
        while !watch_stop.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(5));
            engine.increment_epoch();
        }
    });
    let result = pollster::block_on(fut);
    stop.store(true, Ordering::Relaxed);
    let _ = watchdog.join();
    result
}

/// Why [`DocumentScript::reload`] left the old instance attached.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReloadError {
    /// The new component does not match its grants or the world; each mismatch named.
    Check(CheckReport),
    /// The new component could not be read or is not a component.
    Refused(String),
    /// The new component failed to instantiate (an unlinked import is named).
    Instantiate(String),
    /// The new component's `activate` trapped.
    Activate(String),
}

impl ReloadError {
    pub fn words(&self) -> String {
        match self {
            Self::Check(report) => report.words(),
            Self::Refused(why) => why.clone(),
            Self::Instantiate(why) => format!("instantiate: {why}"),
            Self::Activate(why) => format!("activate: {why}"),
        }
    }
}

impl std::fmt::Display for ReloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.words())
    }
}

impl std::error::Error for ReloadError {}

/// A live DocumentScript attached to a caller-provided page [`ScriptedDom`] — the
/// P2.5 seam the content actor consumes. It owns the wasm instance plus the page
/// DOM (which the script mutates) for the attachment's lifetime; the caller renders
/// from [`dom`](Self::dom) and reads [`revision`](Self::revision) to gate
/// re-renders. Every guest call runs under the epoch + `StoreLimits` guards (§11.2),
/// so a runaway or memory-bomb script is contained and the host survives. A sync
/// surface (drives the async exports via `pollster`) for the sync content-actor
/// thread.
pub struct DocumentScript {
    store: Store<ScriptHost>,
    bindings: DocumentCore,
    epoch_deadline_ticks: u64,
}

impl DocumentScript {
    /// Attach `component_path` to a live page `dom` under `grant` + `quota`, running
    /// `activate`. Fails if a required capability is denied (instantiation fails) or
    /// `activate` traps. The page DOM moves into the instance; recover the
    /// (script-mutated) DOM with [`detach`](Self::detach).
    pub fn attach(
        component_path: &Path,
        dom: ScriptedDom,
        grant: &Grant,
        quota: Quota,
        fetcher: Option<std::sync::Arc<dyn NetFetcher>>,
        net_origins: Vec<String>,
    ) -> wasmtime::Result<Self> {
        Self::attach_with(component_path, None, dom, grant, quota, fetcher, net_origins)
    }

    /// [`attach`](Self::attach) against an install record: the only way to attach a
    /// `.cwasm`, which must hash to the build the record names (mod projects D4).
    ///
    /// # Errors
    ///
    /// As `attach`, and a refusal when the file does not match the record.
    pub fn attach_pinned(
        component_path: &Path,
        record: &InstallRecord,
        dom: ScriptedDom,
        grant: &Grant,
        quota: Quota,
        fetcher: Option<std::sync::Arc<dyn NetFetcher>>,
        net_origins: Vec<String>,
    ) -> wasmtime::Result<Self> {
        Self::attach_with(
            component_path,
            Some(record),
            dom,
            grant,
            quota,
            fetcher,
            net_origins,
        )
    }

    /// Instantiate `component_path` over `dom` under the guards, before `activate`.
    fn instantiate(
        component_path: &Path,
        pin: Option<&InstallRecord>,
        dom: ScriptedDom,
        grant: &Grant,
        quota: Quota,
        fetcher: Option<std::sync::Arc<dyn NetFetcher>>,
        net_origins: Vec<String>,
    ) -> wasmtime::Result<(Store<ScriptHost>, DocumentCore)> {
        let engine = guarded_engine()?;
        let limits = StoreLimitsBuilder::new()
            .memory_size(quota.mem_bytes)
            .build();
        let (mut store, bindings) = pollster::block_on(build_instance(
            &engine,
            component_path,
            dom,
            grant,
            limits,
            Some(quota.epoch_deadline_ticks),
            pin,
        ))?;
        // The network backend for `net.fetch` (None = unconfigured -> fetch errors)
        // plus the origin allowlist gating which URLs it may reach (§E1) and the
        // per-turn fetch cap (§A6).
        store.data_mut().fetcher = fetcher;
        store.data_mut().net_origins = net_origins;
        store.data_mut().max_fetches_per_turn = quota.max_fetches_per_turn;
        store.set_epoch_deadline(quota.epoch_deadline_ticks);
        Ok((store, bindings))
    }

    fn attach_with(
        component_path: &Path,
        pin: Option<&InstallRecord>,
        dom: ScriptedDom,
        grant: &Grant,
        quota: Quota,
        fetcher: Option<std::sync::Arc<dyn NetFetcher>>,
        net_origins: Vec<String>,
    ) -> wasmtime::Result<Self> {
        let (mut store, bindings) =
            Self::instantiate(component_path, pin, dom, grant, quota, fetcher, net_origins)?;
        Self::activate(&mut store, &bindings)?;
        Ok(Self {
            store,
            bindings,
            epoch_deadline_ticks: quota.epoch_deadline_ticks,
        })
    }

    fn activate(store: &mut Store<ScriptHost>, bindings: &DocumentCore) -> wasmtime::Result<()> {
        let engine = store.engine().clone();
        let activate = bindings.call_activate(&mut *store);
        guarded_block_on(engine, activate)
    }

    /// Replace the running component with the one at `component_path`, keeping the
    /// document (mod projects MP3, D3). The new component is checked against `grant`
    /// and the `document-core` world, instantiated, handed the document and
    /// activated; only then does the old instance deactivate and go. Instance-held
    /// state is lost; the document and its revision carry over. A new component that
    /// fails check, instantiation or activation leaves the old instance attached,
    /// the document untouched. With a `record`, the new component loads against it
    /// and a failure is kept as its `last_trap` (a failed check as its `last_check`).
    ///
    /// # Errors
    ///
    /// Why the new component was not attached.
    pub fn reload(
        &mut self,
        component_path: &Path,
        grant: &Grant,
        quota: Quota,
        fetcher: Option<std::sync::Arc<dyn NetFetcher>>,
        net_origins: Vec<String>,
        mut record: Option<&mut InstallRecord>,
    ) -> Result<(), ReloadError> {
        let fail = |record: &mut Option<&mut InstallRecord>, e: ReloadError| {
            if let Some(r) = record.as_deref_mut() {
                match &e {
                    ReloadError::Check(report) => r.note_check(report.words()),
                    other => r.note_trap(format!("reload: {}", other.words())),
                }
            }
            e
        };
        let report = crate::check(component_path, grant, &World::document_core())
            .map_err(|e| fail(&mut record, ReloadError::Refused(e.words())))?;
        if !report.ok() {
            return Err(fail(&mut record, ReloadError::Check(report)));
        }
        let pin = record.as_deref();
        let (mut store, bindings) = Self::instantiate(
            component_path,
            pin,
            ScriptedDom::new(),
            grant,
            quota,
            fetcher,
            net_origins,
        )
        .map_err(|e| fail(&mut record, ReloadError::Instantiate(format!("{e:#}"))))?;
        // Hand the document over, activate, and hand it back if activation fails.
        let doc = std::mem::replace(&mut self.store.data_mut().dom, ScriptedDom::new());
        let revision = self.store.data().revision;
        store.data_mut().dom = doc;
        store.data_mut().revision = revision;
        if let Err(e) = Self::activate(&mut store, &bindings) {
            self.store.data_mut().dom = std::mem::replace(&mut store.data_mut().dom, ScriptedDom::new());
            return Err(fail(&mut record, ReloadError::Activate(format!("{e:#}"))));
        }
        // The new instance owns the document; the old one deactivates and drops. Its
        // own teardown trap changes nothing: it is gone either way.
        let engine = self.store.engine().clone();
        self.store.set_epoch_deadline(self.epoch_deadline_ticks);
        let _ = guarded_block_on(engine, self.bindings.call_deactivate(&mut self.store));
        self.store = store;
        self.bindings = bindings;
        self.epoch_deadline_ticks = quota.epoch_deadline_ticks;
        Ok(())
    }

    /// What the guest has logged through `log` since this instance attached.
    pub fn guest_logs(&self) -> &[String] {
        &self.store.data().logs
    }

    /// Deliver one event under the guards; apply the returned batch to the live DOM;
    /// report the outcome. `Err` means the guest call **trapped** (epoch-cancelled
    /// runaway or memory-bomb) — the host survives and the DOM is unchanged; the
    /// caller should detach the script.
    pub fn deliver_event(&mut self, kind: &str, payload: &str) -> wasmtime::Result<TurnOutcome> {
        let ev = Event {
            kind: kind.to_string(),
            payload: payload.to_string(),
        };
        self.store.set_epoch_deadline(self.epoch_deadline_ticks);
        self.store.data_mut().fetches_this_turn = 0; // fresh per-turn fetch budget (§A6)
        let engine = self.store.engine().clone();
        let turn = self.bindings.call_handle_event(&mut self.store, &ev);
        let result = guarded_block_on(engine, turn)?;
        Ok(match result {
            Ok(batch) => {
                let h = self.store.data_mut();
                match dom_view::apply(&mut h.dom, &mut h.revision, batch) {
                    Ok(rev) => TurnOutcome::Applied(rev),
                    Err(TurnError::RevisionConflict(c)) => TurnOutcome::Conflict(c),
                    Err(TurnError::UnknownNode(id)) => TurnOutcome::UnknownNode(id),
                    Err(TurnError::Refused(w)) => TurnOutcome::Refused(w),
                }
            },
            Err(TurnError::RevisionConflict(c)) => TurnOutcome::Conflict(c),
            Err(TurnError::UnknownNode(id)) => TurnOutcome::UnknownNode(id),
            Err(TurnError::Refused(w)) => TurnOutcome::Refused(w),
        })
    }

    /// The live (possibly script-mutated) page DOM, for the caller to render.
    pub fn dom(&self) -> &ScriptedDom {
        &self.store.data().dom
    }

    /// The current host-side revision (bumped on every applied batch). The caller
    /// gates a re-render on this changing.
    pub fn revision(&self) -> u64 {
        self.store.data().revision
    }

    /// Run `deactivate` (guarded) and return the page DOM to the caller. A trap in
    /// `deactivate` is reported as `Err`, but the DOM is still recovered on success.
    pub fn detach(mut self) -> wasmtime::Result<ScriptedDom> {
        self.store.set_epoch_deadline(self.epoch_deadline_ticks);
        let engine = self.store.engine().clone();
        let deactivate = self.bindings.call_deactivate(&mut self.store);
        guarded_block_on(engine, deactivate)?;
        Ok(self.store.into_data().dom)
    }
}
