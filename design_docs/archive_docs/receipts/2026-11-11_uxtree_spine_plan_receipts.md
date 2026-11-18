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

## 2026-11-17: P1 main integration

The P1 candidate `5e7bf7cbc9b037f282adf4fca68405c8c8a4fae4` was combined
with Graphshell main `f49826cefd60dba7cf2dfa181d95f782e70ce346` without
conflicts. The combined tree passes the same 74 supplier tests. Integration
receipts are retained separately under `Code/testing/graphshell/uxtree-p1/`
as `merge-tests.log`, `merge-hooks.log` and `merge-push.log`. Original P1
qualification receipts remain unchanged.


## 2026-11-17: P2 native envelope qualification

Base: Graphshell main merge `5267dd2ddeb866ca43d6dec0de7ea2ac85c177d7`.
Candidate branch/worktree: `uxtree/p2`, `Code/worktrees/graphshell-uxtree-p2`.
Receiving candidate: Turnstone `uxtree/p2`, based on
`d91d1ffb56808ae331487d980a17a5883b6e3579`.
Raw packet: `Code/testing/graphshell/uxtree-p2/`, 100 MB total across repositories.

`SemanticPublication` replaces `PublicationBindings`: complete uxtree envelopes
plus typed input-delivery routes, which retain the publication's weak witness.
The metadata alias uses the existing `A11yOwner`, `SemanticNodeIdentity` and
`sceno::SourceRef`. Native preparation maps each local address to its final
native ID; tree, envelopes, namespace and routes advance together. Refusal and
owner retirement clear the metadata with the routes. Authored DOM keys are
actual `data-key` or fallback `id`; DOM action kinds create no command IDs.
Producer action-button envelopes carry the actual command and their child's
source reference. Structural children carry the reference without an action
route. Existing native refusal policy is unchanged.

The native semantic publication payload walk passes, including every final address,
Life class, owner/provenance, command, source reference and authored key. The
new Rootstock test freezes a real envelope route, replaces the child while
retaining the old marker, and sees `StaleTarget` with no action effect; a fresh
route dispatches. The pre-layout root-only bootstrap remains
`PendingPresentation`, not a semantic submission, with no routes or envelopes.

Qualification so far:

- Baseline Rootstock library: 240 passed, 18 existing ignores.
- Restored supplier run: 340 passed, 19 ignores (including the existing ignored
  Rootstock doctest). This includes 241 Rootstock library and 75 uxtree/native
  supplier tests, plus Rootstock integration tests.
- Desktop accessibility receiving tests: 10 passed.
- Rootstock and browser host library wasm check: passed with
  `RUSTFLAGS='--cfg getrandom_backend="wasm_js"'`.
- Five source controls fail at runtime: omitted envelopes, unremapped source
  address, omitted command, omitted domain reference, and old-request rebinding.
  The first exploratory provenance-only bypass still passed because the
  independent producer child-token check refused it. Its log is retained;
  `stale_rebind_control.py` demonstrates the stronger defect and fails as expected.
- Rust-source search: zero `PublicationBindings` / `publication_bindings` hits.
- Family checker: 29 repositories, no strict or development source cycles;
  resolved-lock warnings remain separately recorded in the JSON.

Exact commands and output: `qualified-core-tests.log`, `receiving-host-tests.log`,
`wasm-host-check.log`, `control-*.log`, `control-results.json`,
`candidate-source-family-cycles.json`, and the control/checker scripts.
Generated baseline locks are retained, not edited or committed. Turnstone's
canvas qualification and normal hooks are still in progress at this checkpoint.
Browser execution and human/platform assistive-technology acceptance remain open.


The first offline hook attempt lacked cached `regalloc2` 0.15.3. The online
retry passed all three Clippy hooks, then found a pre-existing resolution split:
registry `peer-facts` 0.0.1 through old Dramatis `custody-wire`, alongside Armillary
Git 0.0.3. Current Dramatis main already uses the Armillary source. Normal
`cargo update --manifest-path host/Cargo.toml -p custody-wire` refreshed the 12
Dramatis Git packages from `2c779ded` to `9185d150` and removed registry 0.0.1.
No manifest, patch, skip or manual lock edit was needed. The before/after locks,
package IDs and failed hooks are retained in `graphshell-host*.Cargo.lock`,
`resolution-delta.json`, `refresh-dramatis.log` and `graphshell-commit-hooks*.log`.


### P2 receiving and final review

Turnstone candidate `219c99b1e8168461ae7e214f2ed9e1fae1bd91de` is pushed on
`uxtree/p2`. The 13 canvas reader tests pass, including 12 actual sample members
retaining UUID, described key and command IDs through Rootstock. Dropping the
reference in the real canvas lowering fails the same runtime test; restoring
its bytes passes all 13 again. The first receiving compile exposed a missing
`LayoutDom` import in the new test; the corrected and restored logs are retained.
The no-default-feature `web` library wasm check passes.

Both normal Turnstone Clippy hooks, dependency bans at root/web/relay roots,
house patch checks, resolved-version recording and push hooks pass. Receiving
root package IDs are identical before and after those hooks. The native/wasm
checks and root hooks use Genet `0cd7e985`; the separate web ban resolution uses
`fe42f351`. The relay has no Genet source. These are separate qualification
claims, with exact locks and Git IDs in `receiving-hook-sources.json` and
`receiving-package-ids.json`; the local supplier selection is confined to the
retained gate Cargo configuration. All six contract packages resolve to the
Graphshell candidate checkout. Land Graphshell, refresh Turnstone's Graphshell
Git source with Cargo, then integrate the receiver.

Review removed an assumption that every native number differs from its source
number. Every envelope must match its final tree node and preserve provenance;
numeric coincidences between namespaces are valid. The revised native suite
passes 29 tests, omitting address finalization still fails its runtime assertion,
and restoring the source passes the 29 again. Production publication code is
unchanged from supplier `5c390762`; the final supplier commit adds this test fix
and the completed plan/receipt status.

The native walk uses production `NativeIds::prepare` and commits its output into
a detached host fixture. It certifies the prepared publication payload and
lifetime/admission behavior. OS adapter submission, installed binaries, browser
runtime and human AT are not claimed by these software gates. The existing
pre-layout bootstrap remains a pending window carrier with no semantic routes.

Raw final evidence: `canvas-restored.log`, `control-canvas-domain.log`,
`turnstone-wasm-check.log`, `turnstone-commit-hooks.log`, `turnstone-push.log`,
`native-review-restored.log`, `control-source-address-review.log` and
`qualified-source-family-cycles.json`. The full raw report and hash inventory
are `Code/testing/graphshell/uxtree-p2/report.md` and `SHA256SUMS`.

The P2 worktrees remain for review. Automatic approval review rejected the
normal temporary-home cleanup and the narrower literal junction unlink, stating
only "blocked by policy". The P2-owned home at
`C:/t/cargo-homes/turnstone-uxtree-p2` remains with 325,055 bytes of local files
and two links to shared caches. Its configuration and exact rejection/ownership
status are recorded in `cleanup.json`. Four cache origins created through that
home were normalized to persistent global cache paths. Shared source caches
and reusable Graphshell/Turnstone targets, including the CEF SDK cache, remain.


### P2 combined-main snapshot

During the long dependency checks, remote main advanced to Graphshell
`a9c12694a83b534971b6beeb129ce4adeae5edcb` and Turnstone
`66716cd5a8a4b0818eb2b425f1c8426d4c718430`. P1's merge remains an ancestor of
Graphshell main. Both snapshots merge cleanly into the P2 review branches.
Combined qualification repeats the 340 supplier tests, 10 desktop accessibility
tests, 13 canvas tests and receiving wasm check successfully. These source
changes add no manifest edge; the existing source-cycle qualification remains
applicable. Normal merge hooks also run before these candidates are pushed.

The original report, source records and `SHA256SUMS` are preserved unchanged.
Final combined heads, source diffs, hook/push output and the exact tested main
snapshots are recorded separately in `combined-report.md`,
`combined-candidate-record.json`, `combined-*.log` and `COMBINED-SHA256SUMS` under
`Code/testing/graphshell/uxtree-p2/`. The cleanup rejection remains recorded;
the named Cargo home is also reused for these combined receiving checks.



### P2 published-source landing

Graphshell main advanced to `7c40580ec7ebbdb692705fbbea96e7cd5a8f7e29`.
Turnstone generated a fresh lock without the local supplier override. Its
Rootstock, native accessibility, uxtree and Sceno packages all resolved to that
Graphshell Git commit. The 13 canvas reader tests and wasm library check passed.
This receipt uses Genet `fe42f351`. Turnstone’s dependency bans for root, web and
relay passed along with resolved-version recording and pushing. Turnstone main
advanced to `7406f9f3` with the published-source receipt in its index.
The original qualification packet is preserved. The `landing-turnstone-*`
files under `Code/testing/graphshell/uxtree-p2/` record the final source and results.

## 2026-11-17: P3 Pelt and client item envelopes

Graphshell base `7c40580e`. Raw logs, source controls and generated Cargo lock
are under `Code/testing/graphshell/uxtree-p3/` (100 MB cap). Pelt primary and
tearout windows use the same document lowering. Each mounted document node
carries its window, tile and session, its published local ID, and the captured
admission facts. A Frisket-only DOM lookup still routes shell actions. The
old Pelt `action_map` name has zero Rust source occurrences.

The host adapts the graphshell-client tree by pairing it with the same mounted
scene read. Items carry the observed epoch as their scope, the observed revision
as provenance, the scene source reference and the advertised intent IDs. The
loopback reader uses the adapter. The client crate keeps its tree and routing.

- Pelt desktop library, 85 tests passed. The tearout fixture checks the tile
  envelope and distinct window addresses. Existing stale-action tests pass.
- Graphshell host library, 26 tests passed. The envelope fixture uses epoch 7 and
  revision 13 and checks the complete advertised command list for both profiles.
- Three broken controls fail at runtime if the tile or client source is dropped,
  or if revision replaces epoch. The scripts restore the original source bytes.
- Host library wasm check passed with default features disabled.
- Family source cycles checked across 29 repositories with none found.

These are software receipts. App runtime and human assistive-technology
acceptance have their own gates.


## 2026-11-17: P3 landing and P4 retained-document lookup

P3 landed on Graphshell at `bf987154`. The merged tree with main `2e041d0b`
passed 85 Pelt and 26 Graphshell host tests. `uxtree-p3/landing-record.md`
links the raw results, normal hooks and push record in the existing packet.

Genet candidate `b75face72c9ca054b80d7628633bfb1117a6bdb0` adds optional
`authored_key` and `opaque_address` to Taproot's borrowed node view. Its own
lookups leave both empty. The host uses Taproot's existing role spelling.
Taproot passes 47 tests, including the new metadata check. A planted address
in a built-in lookup trips that check. All Genet commit gates pass. Workspace
metadata covers 59 members and finds no uxtree edge.

Graphshell's Pelt lookup uses Scoped envelopes, with the current session and
captured engine revision. The real Genet-backed test proves a current write,
rejects a captured write after the revision changed, and accepts a fresh write.
It also tests refusal in a different session. The broken control substitutes
the live revision and fails the stale-write assertion. The restored receiving
suite passes 116 Pelt core tests. The cycle gate covers 29 repositories and
finds no strict or development cycles.

The first receiving run used a local Taproot override and passed with Cargo
path-override warnings. The final run selects all 37 Genet packages from the
published Git candidate via a generated lock. There is no committed override
or lock change. Land Genet after review, then integrate Graphshell with the
published supplier. The raw packet under `Code/testing/graphshell/uxtree-p4/`
contains the locks, logs and broken controls, with a 100 MB cap across both repos.


The first Graphshell commit passed Clippy and found a resolver drift in the
ban gate: `codespan-reporting` selected `unicode-width` 0.1.14. A focused Cargo
refresh selected the compatible 0.2.3 for the same holder and kept the exact
Genet source. The policy and manifests are unchanged by this refresh. The
final receiving test log and resolution delta are retained in the P4 packet.


### 2026-11-18: P4 landing

Genet main advanced to `b75face72c9ca054b80d7628633bfb1117a6bdb0`. The
Graphshell candidate `3b247317` used that same supplier and the recorded final
package graph. The focused retained-document test passed again once Genet main
held the tested commit. Its capture, stale replay and fresh action check the
normal host-to-engine route. Graphshell lands the tested source plus this status
and receipt update. The original P4 packet stays intact. New landing outputs
and hashes are in `Code/testing/graphshell/uxtree-p4/landing-*` and
`LANDING-SHA256SUMS`.

## 2026-11-18: P5 shared generation map

D10 is recorded. uxtree owns the generation map. A publishing window owns its
instance and provides admission equality. Path-derived trees can emit Derived
observation envelopes. They carry the producer instance and each local node
ID. An action takes a generation from the window's published admission.

The uxtree native suite passed 28 unit tests, nine integration tests and one
doctest. The wasm32 all-features check passed. A control that reused the
generation for every retained address failed the stale-admission test at
runtime. The original source was restored before the passing runs. Raw
output is under `Code/testing/graphshell/uxtree-p5/`.

### 2026-11-18 P5 paired qualification

Graphshell `a059566801f26a0754c19d75b3435d55b7ee2bde` and Turnstone
`6f45ffc493e8edcefeb9278dc30b172efa1e02eb` incorporate the matching Partition
updates from the two main branches. Turnstone resolves Graphshell from Git.
The generated root and web locks match the post-hook copies.

The combined Graphshell tree passed the 38 uxtree tests and the wasm check.
Turnstone passed 71 accessibility tests and the palette Keep/Drop test. The
receiving control accepted a retired generation and failed at the stale row
assertion. The restored source passed the same test. Document, frozen, Gloss
and Platen fixtures exercised the Derived envelope view. The Graphshell and
Turnstone commit and push hooks passed. The family check found zero strict or
development cycles across 29 repositories.

Turnstone's legacy graph-member selection fills use IDs that differ from the
Frozen Projection nodes, so the publisher filters the fills out. Frozen
Projection selection still needs a binding. The raw results and generated
locks are under `Code/testing/graphshell/uxtree-p5/`. The dependency order for
landing is Graphshell, then Turnstone.

### 2026-11-18 P5 landing

Graphshell main received `5a8d2d1e142e0414bdd83bb4d47b7ba10a47d362`, with code
matching the qualified supplier `a0595668`. All 38 uxtree tests passed again.

The Turnstone root and web locks selected that revision from Graphshell
main. The 71 accessibility tests and palette Keep/Drop test passed. The root,
web and relay dependency bans passed, and the regular push advanced Turnstone
main to `6f45ffc493e8edcefeb9278dc30b172efa1e02eb`. The landed revisions, locks
and gate outputs are recorded in `Code/testing/graphshell/uxtree-p5/landing-*`.

## 2026-11-18 P6 tree-qualified grafts

Graphshell `78c9f4ca6aedfbbd2656691e6705749ce8c79e48` defines the two address spaces, producer instance and
installed tree. Its native uxtree suite passed 29 unit tests, nine integration
tests and one doctest. Ten E1a real-session tests and the all-features wasm check
passed. Replacing the installed tree ID with ROOT failed the address test.

Turnstone `563ed2349fcee5a503e6135548227e19ccd96861` passed 74 accessibility tests against that exact Git
revision. The foreign fixture resolves all nodes of a nested foreign surface.
Retiring it refuses its old action while another surface's same local ID still
resolves. The contributed fixture resolves each published local node through
the actual AccessKit consumer and checks removal from the composed tree.
Comparing only the local ID admits a cross-tree click and fails the deliberate
control. Restoring the source passes it again.

Normal Graphshell and Turnstone hooks passed, including Turnstone's root,
web and relay bans. The receiving root and web locks stayed identical
through the hooks. The family check found zero strict or development cycles
across 29 checkouts. Raw outputs and locks are under `Code/testing/graphshell/uxtree-p6/`.
