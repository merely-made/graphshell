// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! One runtime holding the engines and the loaded guests, and the JSON the
//! page and the native tool exchange with it (guest runtime plan, P7). The
//! native control and the wasm-built host run this same file, so a difference
//! in a result is a difference in where it ran.

use serde_json::{Value, json};
use wasmtime::component::{Component, Linker};
use wasmtime::{Config, Engine};

use crate::document_host::{self, Limits, Outcome, Script, ScriptHost, Turn};
use crate::projection_host;
use crate::stubs::StubOutput;

/// The guests a runtime can hold, by slot.
pub const PROJECTION: u32 = 0;
pub const DOCUMENT: u32 = 1;
pub const BOMB: u32 = 2;

pub struct Runtime {
    plain: Engine,
    guarded: Engine,
    projection: Option<Component>,
    document: Option<Component>,
    bomb: Option<Component>,
    linker: Linker<ScriptHost>,
    output: StubOutput,
    started: Option<projection_host::Started>,
    script: Option<Script>,
}

impl Runtime {
    /// `plain` is the engine config of the projection guest, `guarded` the one
    /// of the document guests (fuel and epochs on). The caller sets the target
    /// and `config::embedded`.
    pub fn new(plain: &Config, guarded: &Config) -> wasmtime::Result<Self> {
        let guarded_engine = Engine::new(guarded)?;
        let (linker, output) = document_host::linker(&guarded_engine)?;
        Ok(Self {
            plain: Engine::new(plain)?,
            guarded: guarded_engine,
            projection: None,
            document: None,
            bomb: None,
            linker,
            output,
            started: None,
            script: None,
        })
    }

    pub fn is_pulley(&self) -> bool {
        self.plain.is_pulley() && self.guarded.is_pulley()
    }

    /// Deserializes precompiled bytecode into a slot.
    ///
    /// # Safety
    /// `bytes` must come from `precompile_component` on a matching Wasmtime and
    /// settings: deserializing trusts them.
    pub unsafe fn load(&mut self, slot: u32, bytes: &[u8]) -> wasmtime::Result<()> {
        let engine = if slot == PROJECTION {
            &self.plain
        } else {
            &self.guarded
        };
        let component = unsafe { Component::deserialize(engine, bytes) }?;
        match slot {
            PROJECTION => self.projection = Some(component),
            DOCUMENT => self.document = Some(component),
            BOMB => self.bomb = Some(component),
            _ => wasmtime::bail!("no guest slot {slot}"),
        }
        Ok(())
    }

    /// Links and instantiates the projection guest, ready for `projection_run`.
    pub fn projection_start(&mut self) -> wasmtime::Result<()> {
        let component = self
            .projection
            .as_ref()
            .ok_or_else(|| wasmtime::format_err!("no projection guest loaded"))?;
        self.started = Some(projection_host::start(&self.plain, component)?);
        Ok(())
    }

    /// Runs the started guest's script and returns its transcript as JSON.
    pub fn projection_run(&mut self) -> wasmtime::Result<String> {
        let started = self
            .started
            .take()
            .ok_or_else(|| wasmtime::format_err!("not started"))?;
        let run = started.run()?;
        Ok(json!({
            "transcript": run.transcript,
            "closedSessions": run.closed_sessions,
            "openSessions": run.open_sessions,
            "stderr": run.stderr,
            "wasiCalls": run.wasi_calls,
        })
        .to_string())
    }

    /// One document turn from a JSON request: attach, deliver, detach.
    pub fn turn(&self, request: &str) -> String {
        let done = (|| {
            let request: Value = serde_json::from_str(request)?;
            let limits = limits_of(&request);
            let component = self.component(&request)?;
            let turn = document_host::run_turn(
                &self.guarded,
                &self.linker,
                component,
                &text(&request, "kind"),
                &text(&request, "payload"),
                &limits,
            );
            Ok::<_, wasmtime::Error>(self.result(&turn, limits.fuel))
        })();
        match done {
            Ok(result) => result.to_string(),
            Err(error) => {
                json!({ "outcome": "error", "message": format!("{error:?}") }).to_string()
            },
        }
    }

    /// Attaches a script that persists across `script_deliver` calls.
    pub fn script_attach(&mut self, request: &str) -> String {
        let attached = (|| -> wasmtime::Result<Script> {
            let request: Value = serde_json::from_str(request)?;
            let component = self.component(&request)?;
            Script::attach(&self.guarded, &self.linker, component, &limits_of(&request))
        })();
        match attached {
            Ok(script) => {
                self.script = Some(script);
                json!({ "outcome": "attached" }).to_string()
            },
            Err(error) => {
                json!({ "outcome": "trapped", "message": format!("{error:#}") }).to_string()
            },
        }
    }

    /// One turn on the attached script.
    pub fn script_deliver(&mut self, kind: &str, payload: &str) -> String {
        let Some(script) = self.script.as_mut() else {
            return json!({ "outcome": "error", "message": "no script is attached" }).to_string();
        };
        let fuel = script.fuel_limit();
        let turn = script.deliver(kind, payload);
        self.result(&turn, fuel).to_string()
    }

    pub fn script_detach(&mut self) {
        self.script = None;
    }

    fn component(&self, request: &Value) -> wasmtime::Result<&Component> {
        let component = match request.get("guest").and_then(Value::as_str) {
            Some("document") => &self.document,
            Some("bomb") => &self.bomb,
            other => wasmtime::bail!("unknown guest {other:?}"),
        };
        component
            .as_ref()
            .ok_or_else(|| wasmtime::format_err!("guest not loaded"))
    }

    fn result(&self, turn: &Turn, fuel: Option<u64>) -> Value {
        let (outcome, message, revision) = match &turn.outcome {
            Outcome::Applied(revision) => ("applied", String::new(), Some(*revision)),
            Outcome::Rejected(why) => ("rejected", format!("{why:?}"), None),
            Outcome::Refused(why) => ("refused", why.clone(), None),
            Outcome::Trapped(why) => ("trapped", why.clone(), None),
        };
        let result = json!({
            "outcome": outcome,
            "message": message,
            "appliedRevision": revision,
            "text": turn.text,
            "revision": turn.revision,
            "fuelUsed": fuel.zip(turn.fuel_left).map(|(start, left)| start - left),
            "memoryPeakBytes": turn.memory_peak_bytes,
            "growthsDenied": turn.growths_denied,
            "stderr": self.output.stderr(),
            "wasiCalls": self.output.calls(),
        });
        self.output.clear();
        result
    }
}

fn text(request: &Value, key: &str) -> String {
    request
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

/// The limits a request names; what it leaves out is `Limits::default()`'s.
fn limits_of(request: &Value) -> Limits {
    let number = |key: &str| request.get(key).and_then(Value::as_u64);
    let defaults = Limits::default();
    Limits {
        fuel: number("fuel"),
        epoch_deadline: number("epochDeadline"),
        tick_every_imports: number("tickEveryImports").unwrap_or(0),
        memory_cap: number("memoryCap").map(|n| n as usize),
        origins: request
            .get("origins")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(String::from)
                    .collect()
            })
            .unwrap_or(defaults.origins),
        max_fetches_per_turn: number("maxFetches")
            .map_or(defaults.max_fetches_per_turn, |n| n as u32),
    }
}
