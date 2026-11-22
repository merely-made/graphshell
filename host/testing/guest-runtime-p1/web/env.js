// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

// What differs between Node and a browser, in one place (guest runtime plan,
// P1): how a file beside this one is read, and how a worker is started and
// talked to. The supervisor and the worker script are the same code in both.

export const IS_NODE = typeof process !== 'undefined' && Boolean(process.versions?.node);

/** The bytes of a file named relative to this directory. */
export async function loadBytes(path) {
  const url = new URL(path, import.meta.url);
  if (IS_NODE) {
    const { readFile } = await import('node:fs/promises');
    return new Uint8Array(await readFile(url));
  }
  return new Uint8Array(await (await fetch(url)).arrayBuffer());
}

/**
 * Starts the worker script at `path` (relative to this directory) and returns
 * its supervisor-side handle: `post`, `onMessage`, `onError` and `terminate`.
 * `onError` also fires when the worker exits without being terminated.
 */
export async function spawnWorker(path) {
  const url = new URL(path, import.meta.url);
  if (IS_NODE) {
    const { Worker } = await import('node:worker_threads');
    const worker = new Worker(url);
    let terminated = false;
    return {
      post: (message) => worker.postMessage(message),
      onMessage: (handler) => worker.on('message', handler),
      onError(handler) {
        worker.on('error', handler);
        worker.on('exit', (code) => {
          if (!terminated) handler(new Error(`worker exited with code ${code}`));
        });
      },
      terminate() {
        terminated = true;
        return worker.terminate();
      },
    };
  }
  const worker = new Worker(url, { type: 'module' });
  return {
    post: (message) => worker.postMessage(message),
    onMessage: (handler) => worker.addEventListener('message', (event) => handler(event.data)),
    onError: (handler) => worker.addEventListener('error', (event) => handler(new Error(event.message))),
    terminate() {
      worker.terminate();
    },
  };
}

/** The worker side of the same channel: `post` and `onMessage`. */
export async function openPort() {
  if (IS_NODE) {
    const { parentPort } = await import('node:worker_threads');
    return {
      post: (message) => parentPort.postMessage(message),
      onMessage: (handler) => parentPort.on('message', handler),
    };
  }
  return {
    post: (message) => self.postMessage(message),
    onMessage: (handler) => self.addEventListener('message', (event) => handler(event.data)),
  };
}
