// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! `time-native <module.wasm> <expected>` runs the module under Wasmer
//! natively and prints the median time, compile included.

use std::time::Instant;

/// Timing rounds; the median is printed.
const ROUNDS: usize = 5;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [path, expected] = args.as_slice() else {
        eprintln!("usage: time-native <module.wasm> <expected>");
        std::process::exit(2);
    };
    let expected: i32 = expected.parse().expect("expected is an integer");
    let bytes = std::fs::read(path).expect("the module file");
    let mut samples = Vec::new();
    for _ in 0..ROUNDS {
        let started = Instant::now();
        let value = wasmer_host::run(&bytes).unwrap_or_else(|reason| {
            eprintln!("{reason}");
            std::process::exit(1);
        });
        samples.push(started.elapsed().as_secs_f64() * 1e3);
        assert_eq!(value, expected, "run() returned the wrong value");
    }
    samples.sort_by(f64::total_cmp);
    println!(
        "native wasmer: run_ms={:.1} (median of {ROUNDS}, compile included)",
        samples[ROUNDS / 2]
    );
}
