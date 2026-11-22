// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

// Runs a document guest under the three guards, in Node or a page (guest
// runtime plan, P1). The guest lives in a worker; this supervisor holds the
// document, sends the worker each event with a snapshot of the document, and
// applies the batch that comes back. A JavaScript engine cannot interrupt
// wasm from the thread running it, so the time limit is a timer here that
// terminates the worker. A trapped turn ends the script, as it does natively
// (`DocumentScript::deliver_event`): the document is unchanged, and the caller
// attaches again for a fresh worker.

import { spawnWorker } from './env.js';

const WORKER_SCRIPT = 'guest-worker.js';

/** Outcomes of a turn, named as the native `TurnOutcome` and `Guarded`. */
export const APPLIED = 'applied';
export const CONFLICT = 'conflict';
export const UNKNOWN_NODE = 'unknownNode';
export const REFUSED = 'refused';
export const TRAPPED = 'trapped';

/** Why a turn trapped. */
export const TIME_LIMIT = 'time-limit';
export const GUEST_TRAP = 'guest-trap';
export const WORKER_ERROR = 'worker-error';

export class ScriptSupervisor {
  /**
   * `document` is a `DocumentFixture`. `options`: `dir` and `name` of the
   * converted guest, `timeLimitMs` per turn, `memoryCapBytes`, `origins` and
   * `maxFetchesPerTurn`.
   */
  constructor(document, options) {
    this.document = document;
    this.options = options;
    this.worker = null;
  }

  get attached() {
    return this.worker !== null;
  }

  /** Starts a worker, loads the guest under the memory cap and runs `activate`. */
  async attach() {
    if (this.worker) throw new Error('already attached');
    const { dir, name, memoryCapBytes, origins, maxFetchesPerTurn } = this.options;
    const worker = await spawnWorker(WORKER_SCRIPT);
    this.worker = worker;
    const ready = this.#next(worker, null);
    worker.post({ type: 'init', dir, name, memoryCapBytes, origins, maxFetchesPerTurn });
    const reply = await ready;
    if (reply.type !== 'ready') {
      this.detach();
      throw new Error(`attach failed: ${reply.message ?? reply.type}`);
    }
    return reply.logs;
  }

  /**
   * The next message from `worker`, or a `worker-error` if it fails first or
   * the time limit passes (`limitMs` null means none).
   */
  #next(worker, limitMs) {
    return new Promise((resolve) => {
      let timer = null;
      const finish = (reply) => {
        clearTimeout(timer);
        resolve(reply);
      };
      worker.onMessage(finish);
      worker.onError((error) => finish({ type: 'worker-error', message: String(error?.message ?? error) }));
      if (limitMs !== null) {
        timer = setTimeout(() => finish({ type: 'time-limit' }), limitMs);
      }
    });
  }

  /** Delivers one event; the result says what happened and what was seen. */
  async deliver(kind, payload) {
    if (!this.worker) throw new Error('not attached: a trapped script is detached');
    const worker = this.worker;
    const started = performance.now();
    const reply = this.#next(worker, this.options.timeLimitMs);
    worker.post({ type: 'turn', kind, payload, snapshot: this.document.view() });
    const message = await reply;
    const elapsedMs = performance.now() - started;
    if (message.type === 'turn') return { ...this.#settle(message), elapsedMs, logs: message.logs, memoryBytes: message.memoryBytes };
    // Every other ending is a trap: the worker is gone and so is its state.
    this.detach();
    if (message.type === 'time-limit') {
      return { outcome: TRAPPED, reason: TIME_LIMIT, elapsedMs, memoryBytes: null, logs: [] };
    }
    return {
      outcome: TRAPPED,
      reason: message.type === 'trap' ? GUEST_TRAP : WORKER_ERROR,
      message: message.message,
      stderr: message.stderr,
      elapsedMs,
      memoryBytes: message.memoryBytes ?? null,
      logs: message.logs ?? [],
    };
  }

  /** The guest's answer to the outcome: apply a batch, or pass its error on. */
  #settle(message) {
    if (message.batch) return this.document.apply(message.batch);
    switch (message.error.tag) {
      case 'revision-conflict': return { outcome: CONFLICT, revision: message.error.val };
      case 'unknown-node': return { outcome: UNKNOWN_NODE, node: message.error.val };
      default: return { outcome: REFUSED, message: message.error.val };
    }
  }

  /** Ends the worker, if there is one. The document stays. */
  detach() {
    const worker = this.worker;
    this.worker = null;
    worker?.terminate();
  }
}
