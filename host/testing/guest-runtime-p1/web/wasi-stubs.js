// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

// The WASI a wasip2 guest's standard library reaches for, as little of it as
// the two test guests read (guest runtime plan, P1). Nothing here grants the
// guest anything: no files, no network, no environment. Output is collected
// so a panic message from the guest can be shown.

export const written = { stdout: '', stderr: '' };

const decoder = new TextDecoder();

export class Error {
  toDebugString() {
    return 'stub stream error';
  }
}

export class Pollable {
  ready() {
    return true;
  }
  block() {}
}

export class InputStream {
  read() {
    throw { tag: 'closed' };
  }
  blockingRead() {
    throw { tag: 'closed' };
  }
  subscribe() {
    return new Pollable();
  }
}

export class OutputStream {
  constructor(sink) {
    this.sink = sink;
  }
  checkWrite() {
    return 4096n;
  }
  write(bytes) {
    written[this.sink] += decoder.decode(bytes);
  }
  blockingWriteAndFlush(bytes) {
    this.write(bytes);
  }
  flush() {}
  blockingFlush() {}
  subscribe() {
    return new Pollable();
  }
}

export class TerminalInput {}
export class TerminalOutput {}

export const cliEnvironment = {
  getEnvironment: () => [],
  getArguments: () => [],
  initialCwd: () => undefined,
};
export const cliExit = {
  exit(status) {
    throw new globalThis.Error(`guest called exit: ${JSON.stringify(status)}`);
  },
  exitWithCode(code) {
    throw new globalThis.Error(`guest called exit with code ${code}`);
  },
};
export const cliStdin = { getStdin: () => new InputStream() };
export const cliStdout = { getStdout: () => new OutputStream('stdout') };
export const cliStderr = { getStderr: () => new OutputStream('stderr') };
export const cliTerminalInput = { TerminalInput };
export const cliTerminalOutput = { TerminalOutput };
export const cliTerminalStdin = { getTerminalStdin: () => undefined };
export const cliTerminalStdout = { getTerminalStdout: () => undefined };
export const cliTerminalStderr = { getTerminalStderr: () => undefined };
export const clocksMonotonicClock = {
  now: () => BigInt(Math.round(performance.now() * 1e6)),
  subscribeDuration: () => new Pollable(),
};
export const clocksWallClock = {
  now() {
    const ms = Date.now();
    return { seconds: BigInt(Math.floor(ms / 1000)), nanoseconds: (ms % 1000) * 1e6 };
  },
};
export const ioError = { Error };
export const ioPoll = { Pollable, poll: (list) => Uint32Array.from(list.keys()) };
export const ioStreams = { InputStream, OutputStream };
