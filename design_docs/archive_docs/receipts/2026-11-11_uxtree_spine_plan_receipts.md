# uxtree spine receipts

## 2026-11-17: P0 federated prototype

Graphshell base: `c53ded07115d3857c62170f42af872d47a8b6e34`.
Review worktree: `Code/worktrees/graphshell-uxtree-p0`, branch `uxtree/p0`.
The primary checkout has unrelated source work and is preserved.

Raw output, resolution and source identities, the cycle-check adapter and
receipt hashes live under `Code/testing/graphshell/uxtree-p0/` (100 MB cap).
The stable reusable target is `C:/t/cargo-targets/graphshell`; no isolated
target or Cargo home is created. This is native, windowless qualification.

Commands:

```text
cargo test --manifest-path host/Cargo.toml -p cambium-a11y --test e1a_sessions -j 4 -- --nocapture
cargo test --offline --manifest-path host/Cargo.toml -p cambium-a11y -p uxtree -j 4
rustfmt --check --edition 2024 host/crates/cambium/cambium-winit-a11y/tests/e1a_sessions.rs
python Code/testing/graphshell/uxtree-p0/cycle_gate.py
```

- Baseline E1a: six passes on the unchanged manifest and test entry. The
  initial baseline attempt overlapped source edits and is not a valid baseline;
  `baseline-e1a-clean.log` is the accepted run.
- Extended E1a: ten passes, including the complete mixed-source gate and
  three expected assertion failures from deliberately broken controls.
  `p0-third-run.log` retains output showing 13 joined nodes, actual texture
  command dispatch, owner refusal after real NodeId reuse, missing-envelope
  detection and the clean tree. The wrong-owner control recaptures current
  provenance for an old observation and fails at the stale-target assertion;
  the other controls disable detection or freeze the document revision.
- Both supplier packages together: 67 passes, zero failures or ignores in
  six test/doc-test groups (`supplier-tests.log`).
- Consumer search: zero matches for producer names or stability classes in
  `tests/e1a_spine/consumer.rs`; the same search finds the adapter declarations
  as its positive control. `search-results.json` records both exit codes.
- Source-manifest cycle gate: 29 real local family checkouts, with this
  Graphshell worktree substituted; zero strict cycles, dev cycles or stale
  allowlist entries. The existing checker is reused with source-manifest
  enumeration (`git ls-files`, including ordinary untracked sources) and
  actual workspace locks, excluding ignored preparation/cache copies.
  `candidate-source-family-cycles.json` records the checker hash and scope.
  Resolved-lock graph cycles remain warnings at 14 lock roots elsewhere in
  the family; this result does not clear those warnings or claim strict-lock
  qualification.
  An earlier primary-only check was exploratory; an unrestricted scan into
  ignored preparation copies was stopped and is not the accepted gate.

The first prototype attempts exposed fixture requirements: a Genet document
needs a retained frame before its semantic read exists, and a Rootstock
TextureProducer publishes semantics through a custom-leaf slot, not the raw
external-texture element. Accepted source uses those existing APIs. Failed
attempt logs are retained rather than overwritten.

The envelope and the missing-metadata check are test-local. All roles,
labels, bounds, children and actions remain in AccessKit. Genet supplies its
actual revision, advances it after an actual value edit and rejects the old
action request itself. Its projection exports neither authored keys nor
command IDs; those fields remain empty. The texture adapter reads the
model's domain reference and its advertised command, and uses Rootstock's
existing weak publication identity and checked producer dispatch.

No production uxtree API, OS adapter, wire format, publisher policy or Genet
source is changed. P1 and later phases, browser/runtime and human screen-reader
acceptance remain open. The candidate worktree is retained by this P0 lane
until review and integration; normal hook output is kept in the raw packet.
