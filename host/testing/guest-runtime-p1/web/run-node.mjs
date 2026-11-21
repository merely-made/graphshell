// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

// Node runner for the projection probe: the same module the page loads, with
// files read from disk. Exits nonzero unless the transcript matches and the
// one session the guest opened was closed.

import { readFile } from 'node:fs/promises';
import { CONTROL_CALL, runProjection } from './run-projection.js';

const load = (path) => readFile(new URL(path, import.meta.url));
// `--control` must report a difference at the altered call, or the
// comparison proves nothing.
const control = process.argv.includes('--control');
const result = await runProjection(load, { control });
console.log(JSON.stringify(result, null, 2));
const passed = control
  ? !result.equal && result.differsAt === CONTROL_CALL
  : result.equal && result.closedSessions === 1 && result.openSessions === 0;
process.exit(passed ? 0 : 1);
