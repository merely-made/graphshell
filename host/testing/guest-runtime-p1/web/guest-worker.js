// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

// The worker a document guest runs in (guest runtime plan, P1). It holds the
// guest and its host-side rules, and nothing of the document: `inspect` is
// served from the snapshot the supervisor sends with each turn, and the
// guest's batch goes back to the supervisor to apply. The supervisor ends a
// runaway turn by terminating this worker, so nothing here can be asked to
// stop; it only has to report what it can.
//
// Messages in:  { type: 'init', dir, name, memoryCapBytes, origins, maxFetchesPerTurn }
//               { type: 'turn', kind, payload, snapshot }
// Messages out: { type: 'ready', logs }
//               { type: 'turn', batch | error, logs, memoryBytes }
//               { type: 'trap', message, logs, memoryBytes, stderr }
//               { type: 'failed', message }   (the script could not be loaded)

import { loadBytes, openPort } from './env.js';
import initHost, { FetchGate, limitMemory } from './host/guest_runtime_web_host.js';
import * as wasi from './wasi-stubs.js';

const HOST_MODULE = 'host/guest_runtime_web_host_bg.wasm';
const PAGE_BYTES = 65536;
/** What `caps.granted()` reports, as the native `Grant::allow_all` does. */
const GRANTED = ['mere:script/log', 'mere:script/document-host', 'mere:script/net'];

const port = await openPort();
let script = null;

/** `inspect` over the snapshot: the whole view, or the subtree under a node. */
function inspect(snapshot, query) {
  if (query.tag === 'whole-document') return snapshot;
  const included = new Set();
  const nodes = [];
  // The view is in document order, so a parent comes before its children.
  for (const node of snapshot.nodes) {
    if (node.id === query.val || (node.parent !== undefined && included.has(node.parent))) {
      included.add(node.id);
      nodes.push(node);
    }
  }
  return { revision: snapshot.revision, nodes };
}

async function load({ dir, name, memoryCapBytes, origins, maxFetchesPerTurn }) {
  await initHost({ module_or_path: await loadBytes(HOST_MODULE) });
  const state = { logs: [], snapshot: null, memory: null };
  const gate = new FetchGate(origins, maxFetchesPerTurn);
  const imports = {
    './wasi-stubs.js': wasi,
    'mere:script/log': { log: (message) => state.logs.push(message) },
    'mere:script/caps': { granted: () => GRANTED },
    'mere:script/net': { fetch: (request) => gate.fetch(request.url) },
    'mere:script/document-host': { inspect: (query) => inspect(state.snapshot, query) },
  };
  const capPages = memoryCapBytes / PAGE_BYTES;
  // The cap is written into each core module before it is compiled.
  const getCoreModule = async (file) =>
    WebAssembly.compile(limitMemory(await loadBytes(dir + file), capPages));
  // Keep the guest's memory, so its size can be read at any point.
  const instantiateCore = async (module, importObject) => {
    const instance = await WebAssembly.instantiate(module, importObject);
    if (instance.exports.memory instanceof WebAssembly.Memory) state.memory = instance.exports.memory;
    return instance;
  };
  const { instantiate } = await import(`./${dir}${name}.js`);
  const guest = await instantiate(getCoreModule, imports, instantiateCore);
  script = { guest, gate, state };
  guest.activate();
}

function memoryBytes() {
  return script?.state.memory?.buffer.byteLength ?? null;
}

function takeLogs() {
  return script.state.logs.splice(0);
}

async function turn({ kind, payload, snapshot }) {
  script.gate.beginTurn();
  script.state.snapshot = snapshot;
  try {
    const batch = script.guest.handleEvent({ kind, payload });
    port.post({ type: 'turn', batch, logs: takeLogs(), memoryBytes: memoryBytes() });
  } catch (error) {
    if (script.guest._util.isComponentError(error)) {
      port.post({ type: 'turn', error: error.payload, logs: takeLogs(), memoryBytes: memoryBytes() });
    } else {
      // A trap: the guest aborted, ran out of memory or hit an unreachable.
      port.post({
        type: 'trap',
        message: String(error?.message ?? error),
        logs: takeLogs(),
        memoryBytes: memoryBytes(),
        stderr: wasi.written.stderr,
      });
    }
  }
}

port.onMessage(async (message) => {
  try {
    if (message.type === 'init') {
      await load(message);
      port.post({ type: 'ready', logs: takeLogs() });
    } else if (message.type === 'turn') {
      await turn(message);
    }
  } catch (error) {
    port.post({ type: 'failed', message: String(error?.stack ?? error) });
  }
});
