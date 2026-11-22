// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

// Node runner for the Wasmer probe: the same module the page loads, with files
// read from disk. Exits nonzero unless run() returned the expected value.

import { readFile } from 'node:fs/promises';
import { runProbe } from './run.js';

const load = (path) => readFile(new URL(path, import.meta.url));
const result = await runProbe(load);
result.node = process.version;
console.log(JSON.stringify(result, null, 2));
process.exit(result.passed ? 0 : 1);
