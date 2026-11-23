// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

// Timing readings for the guest-in-a-page probe (guest runtime plan, P7): the
// same cases on D1's route (P1's runners: the converted guest on the engine's
// own wasm) and on the wasm-built Wasmtime running Pulley, in one process.
// Each reading is a median over `rounds` runs and names its workload. Every
// timed run is checked, so a number is never for a run that went wrong.

import { loadBytes } from '../../guest-runtime-p1/web/env.js';
import { runProjection as runProjectionD1 } from '../../guest-runtime-p1/web/run-projection.js';
import { DocumentFixture } from '../../guest-runtime-p1/web/document-fixture.js';
import { APPLIED, ScriptSupervisor, TRAPPED } from '../../guest-runtime-p1/web/supervisor.js';
import { PulleyHost, SLOT } from './pulley-host.js';

const MIB = 1024 * 1024;
/** Rounds of the whole-script readings and of the per-turn readings. */
const ROUNDS = 7;
const TURN_ROUNDS = 5;
/** Document turns in one per-turn round. */
const TURNS = 200;
/** The cap the allocation bomb is timed to, as the native timing test uses. */
const BOMB_CAP = 256 * MIB;
/** Fuel for the runaway loop: a short and a long cut-off. */
const LOOP_FUEL = [2_000_000, 50_000_000];
/** A limit that no timed D1 turn reaches. */
const NO_TIME_LIMIT = 600_000;

export function median(samples) {
  const sorted = [...samples].sort((a, b) => a - b);
  return sorted[sorted.length >> 1];
}

const summary = (samples) => ({
  medianMs: Number(median(samples).toFixed(3)),
  minMs: Number(Math.min(...samples).toFixed(3)),
  maxMs: Number(Math.max(...samples).toFixed(3)),
  rounds: samples.length,
});

/** Runs `fn` and returns how long it took, in ms. */
async function timed(fn) {
  const started = performance.now();
  const value = await fn();
  return { ms: performance.now() - started, value };
}

async function projectionD1() {
  const total = [];
  const run = [];
  for (let i = 0; i < ROUNDS; i += 1) {
    const { ms, value } = await timed(() => runProjectionD1(loadBytes));
    if (!value.equal) throw new Error('D1 projection run differs from the reference');
    total.push(ms);
    run.push(value.elapsedMs);
  }
  return { loadCompileInstantiateAndRun: summary(total), runOnly: summary(run) };
}

async function projectionP7(load) {
  const files = {
    host: await load('build/host.wasm'),
    projection: await load('build/projection.pulley32.cwasm'),
  };
  const reference = JSON.parse(new TextDecoder().decode(await load('build/reference.json')));
  const [total, hostMs, deserialize, start, run] = [[], [], [], [], []];
  for (let i = 0; i < ROUNDS; i += 1) {
    const t0 = performance.now();
    const host = await PulleyHost.create(files.host);
    const t1 = performance.now();
    host.load(SLOT.projection, files.projection);
    const t2 = performance.now();
    host.projectionStart();
    const t3 = performance.now();
    const result = host.projectionRun();
    const t4 = performance.now();
    if (JSON.stringify(result.transcript) !== JSON.stringify(reference)) {
      throw new Error('Pulley projection run differs from the reference');
    }
    total.push(t4 - t0);
    hostMs.push(t1 - t0);
    deserialize.push(t2 - t1);
    start.push(t3 - t2);
    run.push(t4 - t3);
  }
  return {
    loadCompileInstantiateAndRun: summary(total),
    parts: { createHostModule: summary(hostMs), deserializeGuest: summary(deserialize), linkAndInstantiate: summary(start) },
    runOnly: summary(run),
  };
}

async function documentD1() {
  const attach = [];
  const turn = [];
  for (let round = 0; round < TURN_ROUNDS; round += 1) {
    const document = new DocumentFixture();
    const supervisor = new ScriptSupervisor(document, {
      dir: 'document/', name: 'document', timeLimitMs: NO_TIME_LIMIT,
      memoryCapBytes: 64 * MIB, origins: ['example.com'], maxFetchesPerTurn: 32,
    });
    attach.push((await timed(() => supervisor.attach())).ms);
    const { ms, value } = await timed(async () => {
      let last;
      for (let i = 0; i < TURNS; i += 1) last = await supervisor.deliver('set', 'x');
      return last;
    });
    if (value.outcome !== APPLIED || document.revision !== BigInt(TURNS)) throw new Error('D1 set turns did not all apply');
    turn.push(ms / TURNS);
    supervisor.detach();
  }
  return { attach: summary(attach), setTurn: summary(turn) };
}

async function documentP7(load) {
  const host = await PulleyHost.create(await load('build/host.wasm'));
  const bytes = await load('build/document.pulley32.cwasm');
  const deserialize = [];
  const attach = [];
  const turn = [];
  for (let round = 0; round < TURN_ROUNDS; round += 1) {
    deserialize.push((await timed(() => host.load(SLOT.document, bytes))).ms);
    attach.push((await timed(() => host.attach({ guest: 'document' }))).ms);
    const { ms, value } = await timed(() => {
      let last;
      for (let i = 0; i < TURNS; i += 1) last = host.deliver('set', 'x');
      return last;
    });
    if (value.outcome !== 'applied' || value.revision !== TURNS) throw new Error('Pulley set turns did not all apply');
    turn.push(ms / TURNS);
    host.detach();
  }
  return { deserializeGuest: summary(deserialize), attach: summary(attach), setTurn: summary(turn) };
}

async function bombD1() {
  const ms = [];
  for (let round = 0; round < TURN_ROUNDS; round += 1) {
    const supervisor = new ScriptSupervisor(new DocumentFixture(), {
      dir: 'bomb/', name: 'bomb', timeLimitMs: NO_TIME_LIMIT,
      memoryCapBytes: BOMB_CAP, origins: ['example.com'], maxFetchesPerTurn: 32,
    });
    await supervisor.attach();
    const result = await supervisor.deliver('grow', '');
    if (result.outcome !== TRAPPED) throw new Error('D1 bomb did not trap');
    ms.push(result.elapsedMs);
    supervisor.detach();
  }
  return summary(ms);
}

async function bombP7(load) {
  const host = await PulleyHost.create(await load('build/host.wasm'));
  host.load(SLOT.bomb, await load('build/bomb.pulley32.cwasm'));
  const ms = [];
  for (let round = 0; round < TURN_ROUNDS; round += 1) {
    host.attach({ guest: 'bomb', memoryCap: BOMB_CAP });
    const { ms: took, value } = await timed(() => host.deliver('grow'));
    if (value.outcome !== 'trapped' || value.growthsDenied < 1) throw new Error('Pulley bomb was not denied by the cap');
    ms.push(took);
    host.detach();
  }
  return summary(ms);
}

async function loopP7(load) {
  const host = await PulleyHost.create(await load('build/host.wasm'));
  host.load(SLOT.bomb, await load('build/bomb.pulley32.cwasm'));
  const readings = {};
  for (const fuel of LOOP_FUEL) {
    const ms = [];
    for (let round = 0; round < TURN_ROUNDS; round += 1) {
      host.attach({ guest: 'bomb', fuel });
      const { ms: took, value } = await timed(() => host.deliver('loop'));
      if (value.outcome !== 'trapped' || !value.message.includes('all fuel consumed')) throw new Error('loop was not cut off by fuel');
      ms.push(took);
      host.detach();
    }
    readings[`fuel_${fuel}`] = summary(ms);
  }
  return readings;
}

/** Every reading, as one object. `load(path)` reads under this probe's root. */
export async function timeAll(load) {
  return {
    note: 'Medians over the rounds named. Each timed run is checked (transcript equal, turns applied, traps as expected).',
    projection_20_call_script: {
      d1_engines_own_wasm: await projectionD1(),
      pulley_in_wasmtime_wasm: await projectionP7(load),
    },
    document_set_turn_200_per_round: {
      d1_engines_own_wasm: await documentD1(),
      pulley_in_wasmtime_wasm: await documentP7(load),
    },
    bomb_grow_to_256MiB_cap_deliver_only: {
      d1_engines_own_wasm: await bombD1(),
      pulley_in_wasmtime_wasm: await bombP7(load),
    },
    runaway_loop_cut_off: {
      d1_note: 'D1 ends a loop by terminating its worker at the time limit, so its time is the limit (P1 receipts: 1,008 ms for 1,000).',
      pulley_by_fuel: await loopP7(load),
    },
  };
}
