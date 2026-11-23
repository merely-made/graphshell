// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

// The checks of the guest-in-a-page probe, shared by Node and a page (guest
// runtime plan, P7): the projection transcript against the native reference
// (with a control that alters one reference outcome), and every document case
// against its expectation and against the native Pulley run of the same case.
// `load(path)` returns the bytes of a file under `build/` or `cases.json`.

import { PulleyHost, SLOT } from './pulley-host.js';

const HOST = 'build/host.wasm';
const PROJECTION = 'build/projection.pulley32.cwasm';
const DOCUMENT = 'build/document.pulley32.cwasm';
const BOMB = 'build/bomb.pulley32.cwasm';
const REFERENCE = 'build/reference.json';
const NATIVE = 'build/native-pulley64.json';
const CASES = 'cases.json';
/** The call whose reference outcome the control run alters (as P1). */
export const CONTROL_CALL = 3;

const json = async (load, path) => JSON.parse(new TextDecoder().decode(await load(path)));

/** Index of the first call whose name or outcome differs, or -1. */
function firstDifference(got, want) {
  const length = Math.max(got.length, want.length);
  for (let i = 0; i < length; i += 1) {
    if (got[i]?.[0] !== want[i]?.[0] || got[i]?.[1] !== want[i]?.[1]) return i;
  }
  return -1;
}

/** A host with all three guests loaded. */
export async function openHost(load) {
  const host = await PulleyHost.create(await load(HOST));
  host.load(SLOT.projection, await load(PROJECTION));
  host.load(SLOT.document, await load(DOCUMENT));
  host.load(SLOT.bomb, await load(BOMB));
  return host;
}

export async function runProjection(load, { control = false } = {}) {
  const host = await openHost(load);
  host.projectionStart();
  const run = host.projectionRun();
  const reference = await json(load, REFERENCE);
  if (control) reference[CONTROL_CALL][1] += ' (altered by the control)';
  const differsAt = firstDifference(run.transcript, reference);
  return {
    equal: differsAt === -1,
    calls: run.transcript.length,
    referenceCalls: reference.length,
    differsAt,
    got: differsAt === -1 ? null : run.transcript[differsAt] ?? null,
    want: differsAt === -1 ? null : reference[differsAt] ?? null,
    closedSessions: run.closedSessions,
    openSessions: run.openSessions,
    stderr: run.stderr,
    wasiCalls: run.wasiCalls,
    hostImports: host.imports,
  };
}

/** The expectation keys of `cases.json`, each a check on one result. */
const CHECKS = {
  outcome: (r, v) => r.outcome === v,
  messageIncludes: (r, v) => r.message.includes(v),
  messageEquals: (r, v) => r.message === v,
  text: (r, v) => r.text === v,
  revision: (r, v) => r.revision === v,
  appliedRevision: (r, v) => r.appliedRevision === v,
  growthsDenied: (r, v) => r.growthsDenied === v,
  growthsDeniedAtLeast: (r, v) => r.growthsDenied >= v,
  memoryPeakAtMost: (r, v) => r.memoryPeakBytes <= v,
  memoryPeakAtLeast: (r, v) => r.memoryPeakBytes >= v,
  fuelUsedAtLeast: (r, v) => r.fuelUsed >= v,
  fuelUsedAtMost: (r, v) => r.fuelUsed <= v,
};

/** The fields two runs of one case must agree on, native Pulley and here. */
const AGREE = [
  'outcome', 'message', 'appliedRevision', 'text', 'revision', 'fuelUsed',
  'memoryPeakBytes', 'growthsDenied', 'stderr',
];

export function evaluate(expect, result) {
  const checks = {};
  for (const [key, want] of Object.entries(expect)) checks[key] = CHECKS[key](result, want);
  return checks;
}

/**
 * A script that is attached keeps its instance across turns; a trap ends it,
 * as natively, and a fresh attach completes a turn. Checks, not cases.
 */
function persistentScript(host) {
  const entries = [];
  const check = (name, checks) => entries.push({ name, group: 'script', checks, passed: Object.values(checks).every(Boolean) });

  host.attach({ guest: 'document' });
  const first = host.deliver('set', 'one');
  const second = host.deliver('set', 'two');
  host.detach();
  check('script: the document and instance persist across turns', {
    firstApplied: first.outcome === 'applied' && first.appliedRevision === 1 && first.text === 'one',
    secondApplied: second.outcome === 'applied' && second.appliedRevision === 2 && second.text === 'two',
  });

  host.attach({ guest: 'bomb', fuel: 2000000 });
  const before = host.deliver('noop');
  const cut = host.deliver('loop');
  const after = host.deliver('noop');
  host.detach();
  host.attach({ guest: 'bomb', fuel: 2000000 });
  const fresh = host.deliver('noop');
  host.detach();
  check('script: a cut-off turn ends the script and a fresh attach completes a turn', {
    beforeApplied: before.outcome === 'applied',
    cutOffByFuel: cut.outcome === 'trapped' && cut.message.includes('all fuel consumed'),
    endedAfterTheTrap: after.outcome === 'trapped' && after.message.includes('ended by an earlier trap'),
    documentUnchanged: cut.text === 'Hello' && cut.revision === 0,
    freshAttachCompletes: fresh.outcome === 'applied',
  });
  return entries;
}

/** Every case in `cases.json` on the wasm-built host, beside the native run. */
export async function runCases(load) {
  const host = await openHost(load);
  const cases = await json(load, CASES);
  let native = null;
  try {
    native = await json(load, NATIVE);
  } catch {
    // The native control was not written: the cases still run against their expectations.
  }
  const out = [];
  for (const [index, c] of cases.entries()) {
    const entry = { name: c.name, group: c.group };
    try {
      const result = host.turn(c.request);
      entry.result = result;
      entry.checks = evaluate(c.expect, result);
      if (native) {
        const there = native.cases[index];
        entry.nativeAgrees = there.name === c.name
          && AGREE.every((k) => JSON.stringify(there.result[k]) === JSON.stringify(result[k]));
        entry.nativeDiffers = AGREE.filter((k) => JSON.stringify(there.result[k]) !== JSON.stringify(result[k]));
      }
      entry.passed = Object.values(entry.checks).every(Boolean) && (!native || entry.nativeAgrees);
    } catch (error) {
      entry.passed = false;
      entry.error = String(error?.stack ?? error);
    }
    out.push(entry);
  }
  out.push(...persistentScript(host));
  // The control across cases: a larger cap lets the same bomb get further.
  const peak = (needle) => out.find((e) => e.name.includes(needle))?.result?.memoryPeakBytes;
  const small = peak('denied at a 32 MiB cap');
  const large = peak('at a 64 MiB cap');
  const cross = { name: 'memory: the larger cap reached a larger memory', passed: large > small, small, large };
  // The instrument: an expectation that is wrong for a result must be reported as failing.
  const trapped = out.find((e) => e.result?.outcome === 'trapped')?.result;
  const wrong = trapped ? evaluate({ outcome: 'applied', text: 'nothing' }, trapped) : null;
  const instrument = {
    name: 'instrument: a wrong expectation is caught',
    passed: wrong !== null && Object.values(wrong).every((ok) => !ok),
  };
  const all = [...out, cross, instrument];
  return { passed: all.every((e) => e.passed), nativeCompared: Boolean(native), cases: all };
}

/** JSON with BigInts written as strings. */
export function toJson(value) {
  return JSON.stringify(value, (_, v) => (typeof v === 'bigint' ? v.toString() : v), 2);
}
