// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

// Guest runtime plan, P7, step 4 in Node: D1's route and the wasm-built
// Wasmtime on the same cases. Needs P1's build products (see the README).
//   node timing-node.mjs
import { readFile } from 'node:fs/promises';
import { timeAll } from './timing.js';

const root = new URL('../', import.meta.url);
const load = async (path) => new Uint8Array(await readFile(new URL(path, root)));
const result = await timeAll(load);
result.node = process.version;
console.log(JSON.stringify(result, null, 2));
