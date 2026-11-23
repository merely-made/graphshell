// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The WASI a wasip2 guest's standard library reaches for, as little of it as
//! the test guests import (guest runtime plan, P7). `wasmtime-wasi` does not
//! build for `wasm32-unknown-unknown`, so the page's host supplies these
//! itself through the `Linker`, as `wasi-stubs.js` does for D1's route. Nothing
//! here grants the guest anything: no files, no network, no environment, no
//! clock (both clocks answer zero), and the standard streams are a buffer the
//! host reads. Each function counts its calls so a run can say which of them
//! the guest used.
//!
//! The functions are defined with dynamic values (`Linker::func_new`), so one
//! table serves every signature and no WASI bindings are generated.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use wasmtime::component::{Linker, Resource, ResourceType, Val};
use wasmtime::{Result, StoreContextMut, format_err};

/// The version of WASI the guests were built against (Rust 1.100's wasip2).
const WASI: &str = "0.2.12";

/// Standard stream numbers, carried as the resource representation.
const STDIN: u32 = 0;
const STDOUT: u32 = 1;
const STDERR: u32 = 2;
/// What `check-write` offers: the room the stub buffer always has.
const WRITE_ROOM: u64 = 4096;

// Markers that tell the engine which resource a handle is.
struct PollableRep;
struct ErrorRep;
struct InputRep;
struct OutputRep;
struct TerminalInputRep;
struct TerminalOutputRep;

/// What the stubs gathered: the guest's output and which functions it called.
#[derive(Clone, Default)]
pub struct StubOutput {
    streams: Arc<Mutex<[String; 3]>>,
    calls: Arc<Mutex<BTreeMap<&'static str, u32>>>,
}

impl StubOutput {
    pub fn stderr(&self) -> String {
        self.streams.lock().unwrap()[STDERR as usize].clone()
    }

    /// Each WASI function the guest called, with its count.
    pub fn calls(&self) -> BTreeMap<&'static str, u32> {
        self.calls.lock().unwrap().clone()
    }

    pub fn clear(&self) {
        *self.streams.lock().unwrap() = Default::default();
        self.calls.lock().unwrap().clear();
    }

    fn count(&self, name: &'static str) {
        *self.calls.lock().unwrap().entry(name).or_insert(0) += 1;
    }

    fn write(&self, stream: u32, bytes: &[u8]) {
        if let Some(buffer) = self.streams.lock().unwrap().get_mut(stream as usize) {
            buffer.push_str(&String::from_utf8_lossy(bytes));
        }
    }
}

fn ok_unit() -> Val {
    Val::Result(Ok(None))
}

fn ok_u64(value: u64) -> Val {
    Val::Result(Ok(Some(Box::new(Val::U64(value)))))
}

fn own<T: 'static, D>(store: &mut StoreContextMut<'_, D>, rep: u32) -> Result<Val> {
    Ok(Val::Resource(
        Resource::<T>::new_own(rep).try_into_resource_any(store)?,
    ))
}

/// The rep of a borrowed resource argument.
fn rep_of<T: 'static, D>(store: &mut StoreContextMut<'_, D>, arg: &Val) -> Result<u32> {
    match arg {
        Val::Resource(any) => Ok(Resource::<T>::try_from_resource_any(*any, store)?.rep()),
        _ => Err(format_err!("a resource was expected")),
    }
}

/// Links the WASI floor a std guest needs and returns what it gathers.
pub fn add_wasi_stubs<D: 'static>(linker: &mut Linker<D>) -> Result<StubOutput> {
    let out = StubOutput::default();
    let name = |interface: &str| format!("wasi:{interface}@{WASI}");

    let mut poll = linker.instance(&name("io/poll"))?;
    poll.resource("pollable", ResourceType::host::<PollableRep>(), |_, _| {
        Ok(())
    })?;
    let o = out.clone();
    poll.func_new("[method]pollable.block", move |_, _, _, _| {
        o.count("poll.pollable.block");
        Ok(())
    })?;
    let o = out.clone();
    poll.func_new("poll", move |_, _, args, results| {
        o.count("poll.poll");
        // Everything is ready at once.
        let ready = match &args[0] {
            Val::List(list) => (0..list.len() as u32).map(Val::U32).collect(),
            _ => Vec::new(),
        };
        results[0] = Val::List(ready);
        Ok(())
    })?;

    let mut mono = linker.instance(&name("clocks/monotonic-clock"))?;
    let o = out.clone();
    mono.func_new("now", move |_, _, _, results| {
        o.count("monotonic-clock.now");
        results[0] = Val::U64(0);
        Ok(())
    })?;
    let o = out.clone();
    mono.func_new("subscribe-duration", move |mut store, _, _, results| {
        o.count("monotonic-clock.subscribe-duration");
        results[0] = own::<PollableRep, _>(&mut store, 0)?;
        Ok(())
    })?;

    let mut wall = linker.instance(&name("clocks/wall-clock"))?;
    let o = out.clone();
    wall.func_new("now", move |_, _, _, results| {
        o.count("wall-clock.now");
        let zero = vec![
            ("seconds".to_string(), Val::U64(0)),
            ("nanoseconds".to_string(), Val::U32(0)),
        ];
        results[0] = Val::Record(zero);
        Ok(())
    })?;

    let mut error = linker.instance(&name("io/error"))?;
    error.resource("error", ResourceType::host::<ErrorRep>(), |_, _| Ok(()))?;

    let mut streams = linker.instance(&name("io/streams"))?;
    streams.resource("input-stream", ResourceType::host::<InputRep>(), |_, _| {
        Ok(())
    })?;
    streams.resource(
        "output-stream",
        ResourceType::host::<OutputRep>(),
        |_, _| Ok(()),
    )?;
    let o = out.clone();
    streams.func_new(
        "[method]output-stream.check-write",
        move |_, _, _, results| {
            o.count("streams.output-stream.check-write");
            results[0] = ok_u64(WRITE_ROOM);
            Ok(())
        },
    )?;
    let o = out.clone();
    streams.func_new(
        "[method]output-stream.write",
        move |mut store, _, args, results| {
            o.count("streams.output-stream.write");
            let stream = rep_of::<OutputRep, _>(&mut store, &args[0])?;
            if let Val::List(bytes) = &args[1] {
                let bytes: Vec<u8> = bytes
                    .iter()
                    .map(|b| if let Val::U8(b) = b { *b } else { 0 })
                    .collect();
                o.write(stream, &bytes);
            }
            results[0] = ok_unit();
            Ok(())
        },
    )?;
    let o = out.clone();
    streams.func_new(
        "[method]output-stream.blocking-flush",
        move |_, _, _, results| {
            o.count("streams.output-stream.blocking-flush");
            results[0] = ok_unit();
            Ok(())
        },
    )?;
    let o = out.clone();
    streams.func_new(
        "[method]output-stream.subscribe",
        move |mut store, _, _, results| {
            o.count("streams.output-stream.subscribe");
            results[0] = own::<PollableRep, _>(&mut store, 0)?;
            Ok(())
        },
    )?;

    for (interface, getter, stream) in [
        ("cli/stdout", "get-stdout", STDOUT),
        ("cli/stderr", "get-stderr", STDERR),
    ] {
        let mut std_out = linker.instance(&name(interface))?;
        let o = out.clone();
        let counted: &'static str = if stream == STDOUT {
            "cli.get-stdout"
        } else {
            "cli.get-stderr"
        };
        std_out.func_new(getter, move |mut store, _, _, results| {
            o.count(counted);
            results[0] = own::<OutputRep, _>(&mut store, stream)?;
            Ok(())
        })?;
    }
    let mut stdin = linker.instance(&name("cli/stdin"))?;
    let o = out.clone();
    stdin.func_new("get-stdin", move |mut store, _, _, results| {
        o.count("cli.get-stdin");
        results[0] = own::<InputRep, _>(&mut store, STDIN)?;
        Ok(())
    })?;

    let mut environment = linker.instance(&name("cli/environment"))?;
    let o = out.clone();
    environment.func_new("get-environment", move |_, _, _, results| {
        o.count("cli.get-environment");
        results[0] = Val::List(Vec::new());
        Ok(())
    })?;

    let mut exit = linker.instance(&name("cli/exit"))?;
    let o = out.clone();
    exit.func_new("exit", move |_, _, _, _| {
        o.count("cli.exit");
        Err(format_err!("the guest called exit"))
    })?;
    let o = out.clone();
    exit.func_new("exit-with-code", move |_, _, args, _| {
        o.count("cli.exit-with-code");
        Err(format_err!(
            "the guest called exit with code {:?}",
            args.first()
        ))
    })?;

    linker.instance(&name("cli/terminal-input"))?.resource(
        "terminal-input",
        ResourceType::host::<TerminalInputRep>(),
        |_, _| Ok(()),
    )?;
    linker.instance(&name("cli/terminal-output"))?.resource(
        "terminal-output",
        ResourceType::host::<TerminalOutputRep>(),
        |_, _| Ok(()),
    )?;
    for (interface, getter, counted) in [
        (
            "cli/terminal-stdin",
            "get-terminal-stdin",
            "cli.get-terminal-stdin",
        ),
        (
            "cli/terminal-stdout",
            "get-terminal-stdout",
            "cli.get-terminal-stdout",
        ),
        (
            "cli/terminal-stderr",
            "get-terminal-stderr",
            "cli.get-terminal-stderr",
        ),
    ] {
        let mut terminal = linker.instance(&name(interface))?;
        let o = out.clone();
        // No terminal: the guest sees a pipe.
        terminal.func_new(getter, move |_, _, _, results| {
            o.count(counted);
            results[0] = Val::Option(None);
            Ok(())
        })?;
    }
    Ok(out)
}
