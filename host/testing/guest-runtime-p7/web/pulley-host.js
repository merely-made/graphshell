// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

// The page's side of the wasm-built Wasmtime (guest runtime plan, P7): it
// copies bytes in and reads JSON out. It supplies Wasmtime nothing else: the
// module has no imports (`WebAssembly.Module.imports` is empty).

const FAILED = -1;
const encoder = new TextEncoder();
const decoder = new TextDecoder();

export class PulleyHost {
  #x;

  /** `bytes` is the wasm-built host; the page compiles it with its own engine. */
  static async create(bytes) {
    const { instance } = await WebAssembly.instantiate(bytes, {});
    const host = new PulleyHost();
    host.#x = instance.exports;
    host.imports = WebAssembly.Module.imports(new WebAssembly.Module(bytes));
    host.#call(() => host.#x.init());
    return host;
  }

  #answer() {
    const { memory, answer_ptr, answer_len } = this.#x;
    return decoder.decode(new Uint8Array(memory.buffer, answer_ptr(), answer_len()));
  }

  /** Runs a call that returns a status; throws with the host's reason on failure. */
  #call(call) {
    const status = call();
    const answer = this.#answer();
    if (status === FAILED) throw new Error(answer);
    return answer;
  }

  #put(bytes) {
    const at = this.#x.alloc(bytes.length);
    // Read the memory after `alloc`: it may have grown.
    new Uint8Array(this.#x.memory.buffer, at, bytes.length).set(bytes);
    return at;
  }

  /** Deserializes precompiled bytecode into a slot: 0 projection, 1 document, 2 bomb. */
  load(slot, bytes) {
    const at = this.#put(bytes);
    this.#call(() => this.#x.load(slot, at, bytes.length));
  }

  /** Links and instantiates the projection guest. */
  projectionStart() {
    this.#call(() => this.#x.projection_start());
  }

  /** Runs the started guest; the transcript and what the host counted. */
  projectionRun() {
    return JSON.parse(this.#call(() => this.#x.projection_run()));
  }

  /** One document turn from a request object. */
  turn(request) {
    const bytes = encoder.encode(JSON.stringify(request));
    const at = this.#put(bytes);
    return JSON.parse(this.#call(() => this.#x.turn(at, bytes.length)));
  }

  /** Attaches a script that keeps its instance across `deliver` calls. */
  attach(request) {
    const bytes = encoder.encode(JSON.stringify(request));
    const at = this.#put(bytes);
    return JSON.parse(this.#call(() => this.#x.script_attach(at, bytes.length)));
  }

  /** One turn on the attached script. */
  deliver(kind, payload = '') {
    const k = encoder.encode(kind);
    const p = encoder.encode(payload);
    const at = this.#put(k);
    const pat = this.#put(p);
    return JSON.parse(this.#call(() => this.#x.script_deliver(at, k.length, pat, p.length)));
  }

  detach() {
    this.#call(() => this.#x.script_detach());
  }

  get memoryBytes() {
    return this.#x.memory.buffer.byteLength;
  }
}

export const SLOT = { projection: 0, document: 1, bomb: 2 };
