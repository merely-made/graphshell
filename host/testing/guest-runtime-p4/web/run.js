// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

// Guest runtime plan, P4: run the P3 module through Wasmer's `js` backend,
// which hands it to this JavaScript engine's own wasm support. Shared by the
// page and the Node runner; `load(path)` returns a file's bytes.

import init, { run_module } from './pkg/wasmer_host.js';

// What run() must return (P3's `tool expected`).
export const EXPECTED = 1542256704;
const ROUNDS = 5;

export async function runProbe(load) {
  await init({ module_or_path: await load('./pkg/wasmer_host_bg.wasm') });
  const bytes = new Uint8Array(await load('./module.wasm'));
  const samples = [];
  let value;
  let error = null;
  for (let i = 0; i < ROUNDS; i++) {
    const started = performance.now();
    try {
      value = run_module(bytes);
    } catch (reason) {
      error = String(reason);
      break;
    }
    samples.push(performance.now() - started);
  }
  samples.sort((a, b) => a - b);
  return {
    passed: value === EXPECTED,
    value,
    expected: EXPECTED,
    // Each call also compiles the module.
    runMs: samples.length ? Number(samples[samples.length >> 1].toFixed(1)) : null,
    fastestMs: samples.length ? Number(samples[0].toFixed(1)) : null,
    slowestMs: samples.length ? Number(samples[samples.length - 1].toFixed(1)) : null,
    rounds: samples.length,
    error,
  };
}
