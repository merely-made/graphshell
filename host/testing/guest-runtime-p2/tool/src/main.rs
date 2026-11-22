// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! `precompile <out.cwasm>` writes the probe module as Pulley bytecode for a
//! 32-bit host with no virtual memory and no signals, which is what Wasmtime is
//! when it is itself built for wasm32. `time` runs the same module natively on
//! the compiler and on Pulley and prints both.

use std::time::Instant;

use wasmtime::{Config, Engine, Instance, Module, Store};

/// Loop turns in the probe module: enough work to time an interpreter.
const TURNS: u32 = 50_000_000;
/// The multiplier of the loop's running hash.
const FACTOR: u32 = 31;
/// Native timing rounds; the median is printed.
const ROUNDS: usize = 5;

fn module_text() -> String {
    format!(
        r#"(module
  (func (export "run") (result i32)
    (local $i i32) (local $acc i32)
    (loop $again
      (local.set $acc (i32.add (i32.mul (local.get $acc) (i32.const {FACTOR})) (local.get $i)))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br_if $again (i32.lt_u (local.get $i) (i32.const {TURNS}))))
    (local.get $acc)))"#
    )
}

/// What the module must return, worked out here so every run has a check.
fn expected() -> i32 {
    let mut acc: u32 = 0;
    for i in 0..TURNS {
        acc = acc.wrapping_mul(FACTOR).wrapping_add(i);
    }
    acc as i32
}

/// The settings of an engine with no virtual memory and no signal handlers.
fn embedded(config: &mut Config) {
    config.signals_based_traps(false);
    config.memory_init_cow(false);
    config.memory_reservation(0);
    config.memory_guard_size(0);
    config.memory_reservation_for_growth(0);
}

fn precompile(out: &str) -> wasmtime::Result<()> {
    let mut config = Config::new();
    config.target("pulley32")?;
    embedded(&mut config);
    let engine = Engine::new(&config)?;
    let bytes = engine.precompile_module(module_text().as_bytes())?;
    std::fs::write(out, &bytes)?;
    println!(
        "wrote {out}: {} bytes, expecting run() = {}",
        bytes.len(),
        expected()
    );
    Ok(())
}

fn time_on(label: &str, config: &Config) -> wasmtime::Result<f64> {
    let engine = Engine::new(config)?;
    let module = Module::new(&engine, module_text())?;
    let mut samples = Vec::new();
    for _ in 0..ROUNDS {
        let mut store = Store::new(&engine, ());
        let instance = Instance::new(&mut store, &module, &[])?;
        let run = instance.get_typed_func::<(), i32>(&mut store, "run")?;
        let started = Instant::now();
        let value = run.call(&mut store, ())?;
        samples.push(started.elapsed().as_secs_f64() * 1e3);
        assert_eq!(value, expected(), "{label} returned the wrong value");
    }
    samples.sort_by(f64::total_cmp);
    let median = samples[ROUNDS / 2];
    println!(
        "{label}: backend={} turns={TURNS} run_ms={median:.1} (median of {ROUNDS})",
        if engine.is_pulley() {
            "pulley"
        } else {
            "compiler"
        }
    );
    Ok(median)
}

fn time() -> wasmtime::Result<()> {
    let compiler = time_on("native compiler", &Config::new())?;
    let mut config = Config::new();
    config.target("pulley64")?;
    let pulley = time_on("native pulley", &config)?;
    println!("pulley / compiler = {:.1}", pulley / compiler);
    Ok(())
}

fn main() -> wasmtime::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["precompile", out] => precompile(out),
        ["time"] => time(),
        ["expected"] => {
            println!("{}", expected());
            Ok(())
        },
        _ => {
            eprintln!("usage: precompile <out.cwasm> | time | expected");
            std::process::exit(2);
        },
    }
}
