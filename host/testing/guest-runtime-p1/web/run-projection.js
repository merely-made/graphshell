// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

// Runs the projection guest on the JavaScript engine's own wasm support and
// compares its transcript with the native reference (guest runtime plan, P1).
//
// The guest is the component converted to modules; its imports are served by
// the fixture server, which is Rust compiled to wasm. `load(path)` returns
// the bytes of a file beside this one, so a page and Node share this module.

import initHost, { FixtureHost } from './host/guest_runtime_web_host.js';
import { instantiate } from './projection/projection.js';
import * as wasi from './wasi-stubs.js';

const HOST_MODULE = 'host/guest_runtime_web_host_bg.wasm';
const GUEST_DIR = 'projection/';
const REFERENCE = 'reference.json';

/** The import object the glue expects for `mere:script/projection-session`. */
function sessionImports(host) {
  class Session {
    constructor(handle) {
      this.handle = handle;
    }
    snapshot(ask) {
      return host.snapshot(this.handle, ask);
    }
    fetch(address) {
      return host.fetch(this.handle, address);
    }
    chunk(address, offset, length) {
      return host.chunk(this.handle, address, offset, length);
    }
    resume(epoch, revision) {
      return host.resume(this.handle, epoch, revision);
    }
    intent(call) {
      return host.intent(this.handle, call);
    }
    suspend() {
      host.suspend(this.handle);
    }
    takeNotice() {
      return host.takeNotice(this.handle);
    }
    waitNotice() {
      return host.waitNotice(this.handle);
    }
    // The guest dropped its handle: tell the server, as the native host does.
    [Symbol.dispose]() {
      host.dropSession(this.handle);
    }
  }
  return {
    Session,
    discover: () => host.discover(),
    open(request) {
      const [handle, opened] = host.open(request);
      return [new Session(handle), opened];
    },
  };
}

/** Index of the first call whose name or outcome differs, or -1. */
function firstDifference(got, want) {
  const length = Math.max(got.length, want.length);
  for (let i = 0; i < length; i += 1) {
    if (got[i]?.[0] !== want[i]?.[0] || got[i]?.[1] !== want[i]?.[1]) return i;
  }
  return -1;
}

/** The call whose reference outcome the control run alters. */
export const CONTROL_CALL = 3;

/**
 * `control` alters one reference outcome first, so a run that still reports
 * equality shows the comparison is not looking.
 */
export async function runProjection(load, { control = false } = {}) {
  await initHost({ module_or_path: await load(HOST_MODULE) });
  const host = new FixtureHost();
  const imports = {
    './wasi-stubs.js': wasi,
    'mere:script/projection-session': sessionImports(host),
  };
  const guest = await instantiate(
    async (name) => WebAssembly.compile(await load(GUEST_DIR + name)),
    imports,
  );
  const started = performance.now();
  const transcript = guest.run();
  const elapsedMs = performance.now() - started;
  const reference = JSON.parse(new TextDecoder().decode(await load(REFERENCE)));
  if (control) reference[CONTROL_CALL][1] += ' (altered by the control)';
  const differsAt = firstDifference(transcript, reference);
  return {
    equal: differsAt === -1,
    calls: transcript.length,
    referenceCalls: reference.length,
    differsAt,
    got: differsAt === -1 ? null : transcript[differsAt] ?? null,
    want: differsAt === -1 ? null : reference[differsAt] ?? null,
    closedSessions: host.closedSessions(),
    openSessions: host.openSessions(),
    elapsedMs,
    guestStderr: wasi.written.stderr,
  };
}
