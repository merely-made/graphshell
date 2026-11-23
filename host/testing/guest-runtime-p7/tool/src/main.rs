// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Native tool for the guest runtime plan, P7.
//!
//! - `reference <out.json>`: the native transcript, as P1's tool writes it.
//! - `precompile <kind> <guest.wasm> <out.cwasm> <target>`: a component as
//!   Pulley bytecode (`pulley32` for the wasm-built host, `pulley64` for the
//!   native control). `kind` is `projection` (plain settings) or `document` /
//!   `bomb` (fuel and epoch interruption on).
//! - `native <projection> <document> <bomb> <reference> <cases> <out.json>`:
//!   the control. The same runtime the wasm host is built from, run natively
//!   on `pulley64` with the same embedded settings, over the same bytecode
//!   kind and the same cases.
//! - `time <projection.wasm> <document.wasm> <bomb.wasm>`: native readings of
//!   the same cases on the compiler and on `pulley64`.
//! - `imports <guest.wasm>`: what a guest imports.

use std::process::ExitCode;
use std::time::Instant;
use std::{env, fs, thread};

use graphshell_endpoint::wit::testkit::{
    CALLS_BEFORE_REVOCATION, CountdownGate, FixtureEndpoint, assert_outcomes, run,
};
use graphshell_endpoint::wit::{Calls, MAX_FRAME, WitCarrier, WitServer, serve};
use serde_json::{Value, json};
use wasmtime::component::Component;
use wasmtime::component::types::ComponentItem;
use wasmtime::{Config, Engine};

#[path = "../../shared/config.rs"]
mod config;
#[path = "../../shared/document_host.rs"]
mod document_host;
#[path = "../../shared/driver.rs"]
mod driver;
#[path = "../../shared/projection_host.rs"]
mod projection_host;
#[path = "../../shared/stubs.rs"]
mod stubs;

/// The target of the native control: Pulley at this machine's pointer width.
const NATIVE_PULLEY: &str = "pulley64";
/// The call whose reference outcome the control alters, as P1's page does.
const CONTROL_CALL: usize = 3;
/// Rounds of a native timing reading; the median is printed.
const ROUNDS: usize = 15;
/// Document turns in one timing round.
const TURNS: usize = 200;
/// The cap the allocation bomb is timed to, as the native timing test uses.
const BOMB_CAP_BYTES: u64 = 256 * 1024 * 1024;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn reference() -> Result<Vec<(String, String)>> {
    let (client, end) = script_wire::pipe::pair(MAX_FRAME)?;
    let native = thread::spawn(move || {
        let mut server = WitServer::new(
            FixtureEndpoint::default(),
            CountdownGate::new(CALLS_BEFORE_REVOCATION),
        );
        serve(&mut server, end)
    });
    let transcript = run(&mut WitCarrier::new(Calls::new(client)));
    native
        .join()
        .map_err(|_| "the native server thread panicked")??;
    assert_outcomes(&transcript);
    Ok(transcript)
}

/// Settings for one kind of guest on `target` (`None` = the compiler).
fn config_for(kind: &str, target: Option<&str>) -> Result<Config> {
    let mut config = Config::new();
    if let Some(target) = target {
        config.target(target)?;
        config::embedded(&mut config);
    }
    if kind != "projection" {
        config::guarded(&mut config);
    }
    Ok(config)
}

fn precompile(kind: &str, input: &str, output: &str, target: &str) -> Result<()> {
    let engine = Engine::new(&config_for(kind, Some(target))?)?;
    let bytes = engine.precompile_component(&fs::read(input)?)?;
    fs::write(output, &bytes)?;
    println!("{kind} -> {output}: {} bytes for {target}", bytes.len());
    Ok(())
}

fn runtime(target: Option<&str>) -> Result<driver::Runtime> {
    Ok(driver::Runtime::new(
        &config_for("projection", target)?,
        &config_for("document", target)?,
    )?)
}

fn native(args: &[&str]) -> Result<()> {
    let [projection, document, bomb, reference_path, cases_path, out] = args else {
        return Err("native wants six paths".into());
    };
    let mut rt = runtime(Some(NATIVE_PULLEY))?;
    // SAFETY: the bytes were written by `precompile` for this target and settings.
    unsafe {
        rt.load(driver::PROJECTION, &fs::read(projection)?)?;
        rt.load(driver::DOCUMENT, &fs::read(document)?)?;
        rt.load(driver::BOMB, &fs::read(bomb)?)?;
    }
    let want = reference()?;
    fs::write(reference_path, serde_json::to_string_pretty(&want)?)?;

    rt.projection_start()?;
    let run: Value = serde_json::from_str(&rt.projection_run()?)?;
    let got: Vec<(String, String)> = serde_json::from_value(run["transcript"].clone())?;
    let mut altered = want.clone();
    altered[CONTROL_CALL]
        .1
        .push_str(" (altered by the control)");
    let projection_result = json!({
        "pulley": rt.is_pulley(),
        "calls": got.len(),
        "referenceCalls": want.len(),
        "equal": got == want,
        "controlCaught": got != altered,
        "closedSessions": run["closedSessions"],
        "openSessions": run["openSessions"],
        "stderr": run["stderr"],
        "wasiCalls": run["wasiCalls"],
    });

    let cases: Vec<Value> = serde_json::from_str(&fs::read_to_string(cases_path)?)?;
    let mut results = Vec::new();
    for case in &cases {
        let result: Value = serde_json::from_str(&rt.turn(&case["request"].to_string()))?;
        results.push(json!({ "name": case["name"], "result": result }));
    }
    let report =
        json!({ "target": NATIVE_PULLEY, "projection": projection_result, "cases": results });
    fs::write(out, serde_json::to_string_pretty(&report)?)?;
    println!(
        "native {NATIVE_PULLEY}: projection equal={} controlCaught={} calls={}; {} cases written to {out}",
        got == want,
        got != altered,
        got.len(),
        results.len()
    );
    Ok(())
}

fn median_ms(mut samples: Vec<f64>) -> f64 {
    samples.sort_by(f64::total_cmp);
    samples[samples.len() / 2]
}

/// Compiles a guest component in-process and loads it into a slot.
fn compile_and_load(
    rt: &mut driver::Runtime,
    slot: u32,
    path: &str,
    config: &Config,
) -> Result<()> {
    let engine = Engine::new(config)?;
    let bytes = engine.precompile_component(&fs::read(path)?)?;
    // SAFETY: the bytes were just written by `precompile_component` on this config.
    unsafe { rt.load(slot, &bytes)? };
    Ok(())
}

/// Native readings of the cases P7 times on the wasm host, on one backend.
fn time_on(label: &str, target: Option<&str>, paths: &[&str]) -> Result<()> {
    let [projection, document, bomb] = paths else {
        return Err("time wants three guests".into());
    };
    let want = reference()?;
    let (plain, guarded) = (
        config_for("projection", target)?,
        config_for("document", target)?,
    );
    let (mut start, mut run_ms) = (Vec::new(), Vec::new());
    let (mut attach, mut turn, mut bomb_ms) = (Vec::new(), Vec::new(), Vec::new());
    let grow =
        json!({ "guest": "bomb", "kind": "grow", "payload": "", "memoryCap": BOMB_CAP_BYTES })
            .to_string();
    for _ in 0..ROUNDS {
        let mut rt = driver::Runtime::new(&plain, &guarded)?;
        compile_and_load(&mut rt, driver::PROJECTION, projection, &plain)?;
        let started = Instant::now();
        rt.projection_start()?;
        start.push(started.elapsed().as_secs_f64() * 1e3);
        let started = Instant::now();
        let transcript: Value = serde_json::from_str(&rt.projection_run()?)?;
        run_ms.push(started.elapsed().as_secs_f64() * 1e3);
        let got: Vec<(String, String)> = serde_json::from_value(transcript["transcript"].clone())?;
        assert_eq!(got, want, "{label}: the timed run must equal the reference");

        compile_and_load(&mut rt, driver::DOCUMENT, document, &guarded)?;
        compile_and_load(&mut rt, driver::BOMB, bomb, &guarded)?;
        let attach_request = json!({ "guest": "document" }).to_string();
        let started = Instant::now();
        rt.script_attach(&attach_request);
        attach.push(started.elapsed().as_secs_f64() * 1e3);
        let started = Instant::now();
        for _ in 0..TURNS {
            let r: Value = serde_json::from_str(&rt.script_deliver("set", "x"))?;
            assert_eq!(r["outcome"], "applied");
        }
        turn.push(started.elapsed().as_secs_f64() * 1e3 / TURNS as f64);
        rt.script_detach();
        let started = Instant::now();
        let r: Value = serde_json::from_str(&rt.turn(&grow))?;
        bomb_ms.push(started.elapsed().as_secs_f64() * 1e3);
        assert_eq!(r["outcome"], "trapped");
    }
    println!(
        "timing: backend={label} rounds={ROUNDS} projection_instantiate_ms={:.3} projection_run_ms={:.3} \
         document_attach_ms={:.3} document_set_turn_ms={:.4} bomb_grow_to_256MiB_cap_ms={:.2} (medians)",
        median_ms(start),
        median_ms(run_ms),
        median_ms(attach),
        median_ms(turn),
        median_ms(bomb_ms),
    );
    Ok(())
}

fn imports(path: &str) -> Result<()> {
    let engine = Engine::new(&Config::new())?;
    let component = Component::from_file(&engine, path)?;
    for (name, item) in component.component_type().imports(&engine) {
        let ComponentItem::ComponentInstance(instance) = item.ty else {
            continue;
        };
        println!("{name}");
        for (export, item) in instance.exports(&engine) {
            match item.ty {
                ComponentItem::ComponentFunc(_) => println!("  func {export}"),
                ComponentItem::Resource(_) => println!("  resource {export}"),
                _ => {},
            }
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let result = match args.as_slice() {
        ["reference", out] => reference().and_then(|t| {
            fs::write(out, serde_json::to_string_pretty(&t)?)?;
            println!("{} calls written to {out}", t.len());
            Ok(())
        }),
        ["precompile", kind, input, output, target] => precompile(kind, input, output, target),
        ["native", rest @ ..] => native(rest),
        ["time", rest @ ..] => time_on("compiler", None, rest)
            .and_then(|()| time_on(NATIVE_PULLEY, Some(NATIVE_PULLEY), rest)),
        ["imports", path] => imports(path),
        _ => Err(
            "usage: reference | precompile | native | time | imports (see the source header)"
                .into(),
        ),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        },
    }
}
