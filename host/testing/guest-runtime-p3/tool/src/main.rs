// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! `emit <out.wasm>` writes the probe module, the same 50 million turn loop
//! P2 timed. `time <module.wasm>` runs it under wasmi natively, and once more
//! under a fuel limit too small to finish, which must be cut off.

use std::time::Instant;

/// Loop turns in the probe module, as in P2's.
const TURNS: u32 = 50_000_000;
/// The multiplier of the loop's running hash.
const FACTOR: u32 = 31;
/// Native timing rounds; the median is printed.
const ROUNDS: usize = 5;
/// Fuel far short of what the loop needs.
const SHORT_FUEL: u64 = 1_000_000;

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

fn time(path: &str) -> Result<(), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    let mut samples = Vec::new();
    for _ in 0..ROUNDS {
        let started = Instant::now();
        let value = wasmi_host::run(&bytes, None)?;
        samples.push(started.elapsed().as_secs_f64() * 1e3);
        if value != expected() {
            return Err(format!("run() = {value}, expected {}", expected()));
        }
    }
    samples.sort_by(f64::total_cmp);
    println!(
        "native wasmi: turns={TURNS} run_ms={:.1} (median of {ROUNDS}, compile included)",
        samples[ROUNDS / 2]
    );

    let started = Instant::now();
    let with_fuel = wasmi_host::run(&bytes, Some(u64::MAX))?;
    let fuel_ms = started.elapsed().as_secs_f64() * 1e3;
    if with_fuel != expected() {
        return Err(format!("with fuel, run() = {with_fuel}"));
    }
    println!("native wasmi, fuel counted: run_ms={fuel_ms:.1} (one run)");

    match wasmi_host::run(&bytes, Some(SHORT_FUEL)) {
        Err(reason) => println!("native wasmi, fuel {SHORT_FUEL}: cut off ({reason})"),
        Ok(value) => return Err(format!("fuel {SHORT_FUEL} did not stop the loop: {value}")),
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let outcome = match args.as_slice() {
        ["emit", out] => wat::parse_str(module_text())
            .map_err(|e| e.to_string())
            .and_then(|bytes| {
                std::fs::write(out, &bytes).map_err(|e| e.to_string())?;
                println!(
                    "wrote {out}: {} bytes, expecting run() = {}",
                    bytes.len(),
                    expected()
                );
                Ok(())
            }),
        ["time", path] => time(path),
        ["expected"] => {
            println!("{}", expected());
            Ok(())
        },
        _ => Err("usage: emit <out.wasm> | time <module.wasm> | expected".into()),
    };
    if let Err(reason) = outcome {
        eprintln!("{reason}");
        std::process::exit(1);
    }
}
