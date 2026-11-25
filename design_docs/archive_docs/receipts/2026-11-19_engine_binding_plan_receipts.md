# Engine binding plan: receipts

Run evidence for `graphshell_docs/implementation_strategy/2026-11-19_engine_binding_plan.md`.

## 2026-11-24: E3, a missed delta is recovered

Worktree `C:/t/e3`, branch `e3/recovery`, first from `0b846ebb`. Windows laptop,
`CARGO_TARGET_DIR` and `CARGO_BUILD_BUILD_DIR` at `C:/t/e3-target`, debug info off,
`CARGO_INCREMENTAL=0`, one cargo build at a time.

Tests, debug:

- `cargo test -p scenotime -p chirograph -p graphshell-client`: exit 0, 235 passed.
- `cargo test -p graphshell-endpoint --features stdio,local,network,wit-testkit`: exit 0, 27 passed.
- `cargo test -p graphshell-host --features mere-host --lib` (host workspace): exit 0, 79 passed,
  six of them `mere_host::pin::recovery_tests`.
- `cargo check -p graphshell-host --features board,native,mere-host --tests`: exit 0.
- `cargo clippy --all-targets -p scenotime -p chirograph -p graphshell-client` and
  `-p graphshell-host --features mere-host`: exit 0, no warning in the new code.
- `cargo check -p backstep -p mere-cartography -p mere-resident --tests`: exit 101. Distillery
  `bf4bb4a`, a git dependency of mere-resident, fails `E0063` at `crates/distillery/src/chronicle.rs:783`
  (missing `base_engine` and `engine_revision` in a `SceneDiff`); mere-cartography's tests also fail
  without `physics-engine` (seiche items configured out), as they do without this change.

Costs, release, `e3_cost_probe` (kept beside the evidence, not committed): chain graph of n
nodes and n cites, each edit adding one node; five edits per scale. Raw lines:
`Code/testing/engine-split/e3_retention/numbers_e3_build.tsv`, logs `run_e3_cost_release.log`
and `run_e3_cost_release_v1_quadratic_binds.log` beside it.

| n | keyframe bytes | delta bytes | build ms | advance ms | keyframe count ms | scene diff ms | presentation diff ms |
|---|---|---|---|---|---|---|---|
| 1k | 1,497,244 | 544,110 | 27 to 34 | 7.2 to 8.7 | 1.7 | 0.8 to 0.9 | 1.2 to 1.8 |
| 10k | 15,056,222 | 5,501,624 | 521 to 925 | 118 to 168 | 41 | 10 to 20 | 22 to 44 |
| 100k | 151,271,563 | 55,646,009 | 9,319 to 10,420 | 1,414 to 2,256 | 742 | 122 to 182 | 308 to 408 |

Five deltas replayed: staged with one clone 3.4, 57 and 923 ms; cloned per delta 6.5, 141 and
1,455 ms; a keyframe applied instead 5.2, 85 and 728 ms; the client's whole suffix 13, 296 and
2,908 ms. Before `apply_changes`, advance took 12,007 to 17,052 ms at 100k and the client's suffix
57,632 ms. Two deltas stay retained at every scale under the byte cap.

## 2026-11-24: D5's constructors, D6's slot-stable builds, D7's resident retention

D5 step (i): Graphshell `27b2582b` (landed as `277278d5`); `cargo test -p scenotime -p chirograph`
exit 0, 131 passed; every hook passed.

D5 step (ii), each on `e3/d5-constructors` in its own worktree, did not commit: each repository's
own main fails a hook first. Distillery `bf4bb4a`: dependency bans, `constant_time_eq` 0.4.2
(blake3 1.8.7) beside 0.6.1 (zip 9.8.2). Knot `69402ac`: dependency bans, `skrifa` 0.44.0 and
0.49.0 and `read-fonts` 0.41.0 and 0.46.0 (genet-parley 0.11.0 against netrender). mer3ly
`0227107`: clippy cannot resolve, `genet-scripted-dom` patched at `=0.1.2` while genet main is
0.2.0. Turnstone `79104683`: clippy, Graphshell `bd57ee47`'s mere-fetch misses `http_response` in
four `page_load::Fetched` initializers.

E3 with D6 and D7, on `277278d5`, uncommitted until step (ii) lands:

- `cargo test -p scenotime -p chirograph -p graphshell-client`: exit 0, 235 passed.
- `cargo test -p graphshell-host --features mere-host --lib`: exit 0, 82 passed.
- `cargo test -p mere-resident`, with distillery, esp, esp-testkit, mere-athanor and mere-mesh
  patched to the distillery worktree by `--config`: 378 passed, 2 failed, both in
  `tests/personal_facts.rs` on an application broker path ("os error 123"), not checked against
  main. The library's 324 pass, `a_reconnected_application_resumes_from_retained_deltas` among them.
- `cargo clippy --all-targets -p graphshell-host -p mere-resident --features graphshell-host/mere-host`
  with the same patch: exit 0, no warning in the new files.

Slot-stable costs, release, the same probe (`numbers_e3_build.tsv`, v3; log
`run_e3_cost_release_v3_slot_stable.log`):

| n | keyframe bytes | delta bytes | build ms | advance ms | keyframe count ms | scene diff ms | presentation diff ms |
|---|---|---|---|---|---|---|---|
| 1k | 1,500,266 | 630,126 | 19 to 28 | 3.3 to 4.3 | 1.8 | 0.4 to 0.6 | 0.9 to 1.0 |
| 10k | 15,086,478 | 6,365,386 | 316 to 384 | 57 to 82 | 19 | 5.1 to 7.3 | 13 to 19 |
| 100k | 151,574,162 | 64,256,957 | 4,540 to 5,381 | 707 to 824 | 280 | 64 to 76 | 174 to 204 |

Each delta is 2n + 3 scene operations and two presentation changes. Five replayed with one clone:
3.2, 20 and 343 ms; cloned per delta 5.4, 48 and 709 ms; a keyframe applied instead 4.4, 36 and
456 ms; the client's whole suffix 9.0, 82 and 1,127 ms. Two deltas retained at every scale.

## 2026-11-24: D206, retention dropped; D5 step (ii) landings

D5 step (ii): distillery `dc7da32` (the `constant_time_eq` skip) and `d9a45cc` (literals); mer3ly
`3a85a6d` (lower-bound genet and cambium rows) and `c4c39ea` (literals); every hook passed. Knot
and Turnstone wait on genet-parley: Turnstone's `ports/graphshell/web` bans fail on the same
`skrifa` and `read-fonts` duplicates as knot. Turnstone's redshank errors were a stale untracked
lock holding Woodshed at `e5277cd8`; a fresh resolve takes `2e83763b`, which has the API.

E3 reworked under D206 on Graphshell `ab16d9d7`: D7's resident wiring removed, resumes answered by
`chirograph::keyframe_reply`. `cargo test -p scenotime -p chirograph -p graphshell-client`: exit 0;
`cargo test -p graphshell-host --features mere-host --lib`: exit 0, 80 passed; `cargo test -p
mere-resident --lib` against distillery main `d9a45cc`, fresh resolve: exit 0, 323 passed.
