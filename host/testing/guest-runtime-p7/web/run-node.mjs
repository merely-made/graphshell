// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

// Guest runtime plan, P7, in Node: the projection transcript (and its control)
// and the document cases on the wasm-built Wasmtime.
//   node run-node.mjs            projection, control and cases; exit 0 when all pass
//   node run-node.mjs --quiet    one summary line per check
import { readFile } from 'node:fs/promises';
import { CONTROL_CALL, runCases, runProjection, toJson } from './run-p7.js';

const root = new URL('../', import.meta.url);
const load = async (path) => new Uint8Array(await readFile(new URL(path, root)));
const quiet = process.argv.includes('--quiet');

const projection = await runProjection(load);
const control = await runProjection(load, { control: true });
const cases = await runCases(load);

const projectionPassed = projection.equal && projection.closedSessions === 1 && projection.openSessions === 0;
const controlPassed = !control.equal && control.differsAt === CONTROL_CALL;
const result = {
  node: process.version,
  passed: projectionPassed && controlPassed && cases.passed,
  projection: { passed: projectionPassed, ...projection },
  control: { passed: controlPassed, differsAt: control.differsAt, got: control.got, want: control.want },
  cases,
};
if (quiet) {
  console.log(`projection: ${projection.equal ? 'equal' : 'DIFFERS'} (${projection.calls} of ${projection.referenceCalls} calls)`);
  console.log(`control: ${controlPassed ? 'caught at call ' + control.differsAt : 'NOT caught'}`);
  for (const c of cases.cases) console.log(`${c.passed ? 'pass' : 'FAIL'}  ${c.name}`);
  console.log(`passed: ${result.passed}`);
} else {
  console.log(toJson(result));
}
process.exit(result.passed ? 0 : 1);
