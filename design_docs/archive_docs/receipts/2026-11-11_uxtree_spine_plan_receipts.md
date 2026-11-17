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

## 2026-11-17: P0 main integration

Mark authorized the P0 merge and continuation to P1. The P0 branch at
`985035098b4cf7ee3836f433653cfdf1de5163a5` was combined with main
`18744fffcb5b84725615a4ea836819b1f5ef6b8d` without conflicts.
`cargo test --offline --manifest-path host/Cargo.toml -p cambium-a11y -p uxtree -j 4`
passes all 67 supplier tests on that combined tree. New raw output is
`Code/testing/graphshell/uxtree-p0/merge-tests.log`; merge hooks and push output
are retained separately. Original P0 receipts are preserved.

## 2026-11-17: P1 production metadata

Base: published P0 merge `2581ba615c7cf27d47522779420c08ca333c3bf5`.
Candidate: branch `uxtree/p1`, worktree `Code/worktrees/graphshell-uxtree-p1`.
Raw packet: `Code/testing/graphshell/uxtree-p1/`, 100 MB across-repository cap.
The existing stable target `C:/t/cargo-targets/graphshell` is reused. No isolated
target or Cargo home is created.

`uxtree::envelope` defines `Address<I>`, `Stability` and `Envelope<I, P, D>`.
The common instance, provenance and domain types are supplied by the reader's
owners. Six metadata fields sit beside AccessKit; none duplicates node
semantics. Constructors explicitly declare Life, Scoped revision/epoch,
Derived optional owner generation, or Authored lookup key. They allocate no
lives and authorize no action. Publishing adapters and generation maps remain
P2 onward. The per-field producer table lives in the module docs.

Accepted commands:

```text
cargo test --offline --manifest-path host/Cargo.toml -p uxtree -p cambium-a11y -j 4
cargo check --offline --manifest-path host/Cargo.toml -p uxtree --all-features --lib -j 4
cargo check --offline --manifest-path host/Cargo.toml -p uxtree --all-features --lib --target wasm32-unknown-unknown -j 4
python Code/testing/graphshell/uxtree-p1/constructor_controls.py
python Code/testing/graphshell/uxtree-p1/cycle_gate.py
```

- Supplier run: 74 passes, zero failures or ignores across six unit,
  integration and doc-test groups (`supplier-tests.log`). The P0 E1a bench
  now uses the public metadata and named constructors with the same real
  source projections and all three original broken controls.
- Native and wasm all-feature library checks pass. This is compilation,
  not browser execution or human screen-reader acceptance.
- Actual source mutations are detected: Scoped returning Life, an Authored
  constructor dropping its key, and a duplicated AccessKit Role field each
  fail the focused test at its intended assertion. Full failed outputs and
  expected failures are recorded in `control-*.log` and
  `constructor-control-results.json`; the runner restores exact source bytes.
- The actual Envelope declaration is checked for exactly the six metadata
  fields and absence of role/name/state/bounds/children/relations. A permanent
  negative control plants each prohibited field in the schema guard.
- Uxtree's manifest is byte-for-byte unchanged. The common consumer still has
  no producer-kind or stability-class branch. `manifest-and-consumer-checks.json`
  records both searches; the diff and name-search logs are retained.
- Source-manifest cycle gate: 29 actual family checkouts, with this worktree
  substituted for Graphshell; no strict/dev cycle or stale allowlist entry.
  The same accepted P0 checker adapter is reused. Existing resolved-lock
  warnings at 14 roots remain warnings; strict-lock qualification is not claimed.

The README and crate docs now describe the new metadata. Related stale
documentation was corrected: stitch concatenates child nodes and hashes only
its new root, and the E1a consumer/version wording follows the current source.
The primary checkout's concurrent work remains preserved. Required normal
commit and push hook output, selected dependency identities and source hashes
are retained in the raw packet; the P1 worktree stays until integration.
