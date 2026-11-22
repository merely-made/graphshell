// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

// Guest runtime plan, P3: run the probe module on wasmi, where wasmi is itself
// a wasm module inside this JavaScript engine. Shared by the page and the
// Node runner; `load(path)` returns a file's bytes.

// What run() must return (`tool expected`).
export const EXPECTED = 1542256704;
// The host's "did not finish" value, `i32::MIN`.
const FAILED = -2147483648;
// Fuel far short of what the loop needs; the run must be cut off.
const SHORT_FUEL = 1000000n;
const NO_LIMIT = 0n;
const ROUNDS = 5;

export async function runProbe(load) {
  const host = await WebAssembly.instantiate(await load('./wasmi_host.wasm'), {});
  const { memory, alloc, run_module, last_error_ptr, last_error_len } = host.instance.exports;
  const lastError = () =>
    new TextDecoder().decode(new Uint8Array(memory.buffer, last_error_ptr(), last_error_len()));

  const bytes = await load('./module.wasm');
  const at = alloc(bytes.length);
  new Uint8Array(memory.buffer, at, bytes.length).set(bytes);

  const samples = [];
  let value;
  for (let i = 0; i < ROUNDS; i++) {
    const started = performance.now();
    value = run_module(at, bytes.length, NO_LIMIT);
    samples.push(performance.now() - started);
  }
  samples.sort((a, b) => a - b);
  const error = value === FAILED ? lastError() : null;

  // The control for the fuel guard: the same module, too little fuel.
  const short = run_module(at, bytes.length, SHORT_FUEL);
  const cutOff = short === FAILED;

  return {
    passed: value === EXPECTED && cutOff,
    value,
    expected: EXPECTED,
    // Each call also compiles the module.
    runMs: Number(samples[ROUNDS >> 1].toFixed(1)),
    fastestMs: Number(samples[0].toFixed(1)),
    slowestMs: Number(samples[ROUNDS - 1].toFixed(1)),
    rounds: ROUNDS,
    cutOffByShortFuel: cutOff,
    shortFuelMessage: cutOff ? lastError() : null,
    error,
  };
}
