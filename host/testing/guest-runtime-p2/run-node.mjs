// Guest runtime plan, P2: run a module compiled to Pulley bytecode on a
// Wasmtime that is itself a wasm module, inside this JavaScript engine.
//   node run-node.mjs <wasm-host.wasm> <module.cwasm> <expected>
// Prints one JSON object; exit 0 when run() returned the expected value.
import { readFileSync } from 'node:fs';

const [hostPath, modulePath, expectedText] = process.argv.slice(2);
const expected = Number(expectedText);
const ROUNDS = 9;

const host = await WebAssembly.instantiate(readFileSync(hostPath), {});
const { memory, alloc, run_precompiled, last_error_ptr, last_error_len } = host.instance.exports;

const bytes = readFileSync(modulePath);
const at = alloc(bytes.length);
new Uint8Array(memory.buffer, at, bytes.length).set(bytes);

const samples = [];
let value;
for (let i = 0; i < ROUNDS; i++) {
  const started = performance.now();
  value = run_precompiled(at, bytes.length);
  samples.push(performance.now() - started);
}
samples.sort((a, b) => a - b);

const error = new TextDecoder().decode(
  new Uint8Array(memory.buffer, last_error_ptr(), last_error_len()),
);
const result = {
  passed: value === expected,
  value,
  expected,
  // Each call also builds an engine and deserializes the module.
  runMs: Number(samples[ROUNDS >> 1].toFixed(1)),
  fastestMs: Number(samples[0].toFixed(1)),
  slowestMs: Number(samples[ROUNDS - 1].toFixed(1)),
  rounds: ROUNDS,
  error: error || null,
  node: process.version,
};
console.log(JSON.stringify(result, null, 2));
process.exit(result.passed ? 0 : 1);
