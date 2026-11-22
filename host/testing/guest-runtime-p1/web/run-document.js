// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

// The guard cases for the document guest, shared by Node and a page (guest
// runtime plan, P1). Each guard that the native tests show (`tests/guarded.rs`
// and `tests/document_script.rs`) runs here under the supervisor, next to a
// control that must come out the other way: a check that cannot fail proves
// nothing. The document is the supervisor's fixture, not Genet's DOM.

import { DocumentFixture } from './document-fixture.js';
import {
  APPLIED, GUEST_TRAP, REFUSED, ScriptSupervisor, TIME_LIMIT, TRAPPED,
} from './supervisor.js';

const MIB = 1024 * 1024;
/** Per-turn limit: well over a normal turn, short enough to wait for a runaway. */
const TURN_LIMIT_MS = 1000;
/** The limit a memory test runs under, so a failed cap cannot run away. */
const MEMORY_TEST_LIMIT_MS = 5000;
/** The caps the native memory test and its control use. */
const CAP_SMALL = 32 * MIB;
const CAP_LARGE = 64 * MIB;
/** Native `Quota::default().mem_bytes` and the loop test's cap. */
const DEFAULT_CAP = 64 * MIB;
const LOOP_CAP = 256 * MIB;
/** Native `DEFAULT_MAX_FETCHES_PER_TURN`. */
const DEFAULT_FETCHES = 32;
const ORIGINS = ['example.com'];
const ALLOWED_URL = 'https://example.com/x';
const OUTSIDE_URL = 'https://attacker.test/leak';

const DOCUMENT_GUEST = { dir: 'document/', name: 'document' };
const BOMB_GUEST = { dir: 'bomb/', name: 'bomb' };

/** The native text the guest builds from the host's refusal. */
const outsideOrigins = (url) =>
  `fetch failed: net.fetch: ${url} is outside this script's net origins`;
const fetchLimit = (n, cap) =>
  `fetch ${n} failed: net.fetch: per-turn request limit reached (${cap})`;

async function supervise(guest, overrides) {
  const document = new DocumentFixture();
  const supervisor = new ScriptSupervisor(document, {
    ...guest,
    timeLimitMs: TURN_LIMIT_MS,
    memoryCapBytes: DEFAULT_CAP,
    origins: ORIGINS,
    maxFetchesPerTurn: DEFAULT_FETCHES,
    ...overrides,
  });
  await supervisor.attach();
  return { supervisor, document };
}

/** `checks` maps a name to a boolean; the case passes when all hold. */
function verdict(name, checks, detail) {
  return { name, passed: Object.values(checks).every(Boolean), checks, ...detail };
}

const unchanged = (document) => document.revision === 0n && document.texts().join() === 'Hello';

async function timeLimit() {
  const { supervisor, document } = await supervise(BOMB_GUEST, { memoryCapBytes: LOOP_CAP });
  const looped = await supervisor.deliver('loop', '');
  const detached = !supervisor.attached;
  const unchangedAfter = unchanged(document);
  // A fresh worker attaches to the same document and completes a turn.
  await supervisor.attach();
  const again = await supervisor.deliver('noop', '');
  supervisor.detach();
  return verdict('time limit: runaway loop is terminated', {
    trappedByTimeLimit: looped.outcome === TRAPPED && looped.reason === TIME_LIMIT,
    // The turn ran to the limit, not past it by much.
    stoppedNearTheLimit: looped.elapsedMs >= TURN_LIMIT_MS && looped.elapsedMs < TURN_LIMIT_MS * 3,
    detached,
    documentUnchanged: unchangedAfter,
    freshWorkerCompletesATurn: again.outcome === APPLIED,
  }, { limitMs: TURN_LIMIT_MS, elapsedMs: looped.elapsedMs, native: 'trapped' });
}

async function timeLimitControl() {
  const { supervisor, document } = await supervise(BOMB_GUEST, { memoryCapBytes: LOOP_CAP });
  const result = await supervisor.deliver('noop', '');
  const stillAttached = supervisor.attached;
  supervisor.detach();
  return verdict('time limit control: a noop turn completes under the same limit', {
    applied: result.outcome === APPLIED,
    underTheLimit: result.elapsedMs < TURN_LIMIT_MS,
    stillAttached,
    documentUnchanged: unchanged(document),
  }, { elapsedMs: result.elapsedMs, native: 'completed' });
}

/** Runs `grow` under a cap, once, with the time limit as a backstop. */
async function grow(capBytes) {
  const { supervisor } = await supervise(BOMB_GUEST, {
    memoryCapBytes: capBytes, timeLimitMs: MEMORY_TEST_LIMIT_MS,
  });
  const result = await supervisor.deliver('grow', '');
  supervisor.detach();
  return result;
}

async function memoryCap() {
  const small = await grow(CAP_SMALL);
  const large = await grow(CAP_LARGE);
  const trapped = (r) => r.outcome === TRAPPED && r.reason === GUEST_TRAP;
  return verdict('memory cap: allocation bomb traps at the cap', {
    smallTrappedByTheGuest: trapped(small),
    largeTrappedByTheGuest: trapped(large),
    smallWithinCap: small.memoryBytes > 0 && small.memoryBytes <= CAP_SMALL,
    largeWithinCap: large.memoryBytes > 0 && large.memoryBytes <= CAP_LARGE,
    // The guest really grew to near the cap, so the cap is what stopped it.
    smallReachedHalfTheCap: small.memoryBytes >= CAP_SMALL / 2,
    largeReachedHalfTheCap: large.memoryBytes >= CAP_LARGE / 2,
    // The control: a larger cap gives a larger memory at the trap.
    largerCapGivesLargerMemory: large.memoryBytes > small.memoryBytes,
  }, {
    capSmallBytes: CAP_SMALL,
    memoryAtTrapSmall: small.memoryBytes,
    capLargeBytes: CAP_LARGE,
    memoryAtTrapLarge: large.memoryBytes,
    trapMessage: small.message,
    elapsedMs: { small: small.elapsedMs, large: large.elapsedMs },
    native: 'trapped',
  });
}

async function originGate() {
  const { supervisor, document } = await supervise(DOCUMENT_GUEST);
  const refused = await supervisor.deliver('fetch', OUTSIDE_URL);
  const textAfter = document.texts().join();
  const revisionAfter = document.revision;
  supervisor.detach();

  const control = await supervise(DOCUMENT_GUEST);
  const allowed = await control.supervisor.deliver('fetch', ALLOWED_URL);
  const allowedText = control.document.texts().join();
  control.supervisor.detach();

  return verdict('origin gate: cross-origin fetch is refused', {
    refused: refused.outcome === REFUSED,
    sameTextAsNative: refused.message === outsideOrigins(OUTSIDE_URL),
    documentTextStillHello: textAfter === 'Hello',
    revisionUnchanged: revisionAfter === 0n,
    // The control: an allowed URL goes through and changes the text.
    controlApplied: allowed.outcome === APPLIED && allowed.revision === 1n,
    controlChangedTheText: allowedText === `fetched:${ALLOWED_URL}`,
  }, { message: refused.message, controlText: allowedText, native: 'Refused, text Hello' });
}

async function fetchCap() {
  const tight = await supervise(DOCUMENT_GUEST, { maxFetchesPerTurn: 1 });
  const capOne = await tight.supervisor.deliver('fetch-twice', ALLOWED_URL);
  const capOneText = tight.document.texts().join();
  tight.supervisor.detach();

  const roomy = await supervise(DOCUMENT_GUEST, { maxFetchesPerTurn: 2 });
  const first = await roomy.supervisor.deliver('fetch-twice', ALLOWED_URL);
  const second = await roomy.supervisor.deliver('fetch-twice', ALLOWED_URL);
  const capTwoText = roomy.document.texts().join();
  roomy.supervisor.detach();

  return verdict('fetch cap: a flood is refused and the count resets each turn', {
    capOneRefused: capOne.outcome === REFUSED,
    capOneSameTextAsNative: capOne.message === fetchLimit(2, 1),
    capOneDocumentUnchanged: capOneText === 'Hello',
    // The control: with room for two, the same event is applied...
    capTwoFirstTurnApplied: first.outcome === APPLIED && first.revision === 1n,
    // ...and again in a second turn, so the count was reset.
    capTwoSecondTurnApplied: second.outcome === APPLIED && second.revision === 2n,
    capTwoChangedTheText: capTwoText === `fetched:${ALLOWED_URL}`,
  }, { capOneMessage: capOne.message, native: 'cap 1: Refused; cap 2: Applied(1), Applied(2)' });
}

/** Runs every case and control; `passed` is true when all of them hold. */
export async function runDocument() {
  const cases = [];
  for (const run of [timeLimit, timeLimitControl, memoryCap, originGate, fetchCap]) {
    try {
      cases.push(await run());
    } catch (error) {
      cases.push({ name: run.name, passed: false, error: String(error?.stack ?? error) });
    }
  }
  return { passed: cases.every((c) => c.passed), cases };
}

/** JSON with the BigInts the contract uses written as strings. */
export function toJson(value) {
  return JSON.stringify(value, (_, v) => (typeof v === 'bigint' ? v.toString() : v), 2);
}
