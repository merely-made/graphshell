# scenotime

Runtime for the scenograph projection engine: stable epochs, idempotent diffs,
host-clocked transitions, and picking. It depends on [`sceno`](../sceno),
`serde` and `serde_json`.

`scenotime` is part of the Graphshell protocol layer
(`https://github.com/merely-made/graphshell`).

- `SceneSnapshot` / `SceneTables` with tombstoned slots; `SceneEpoch`,
  `Revision` and the id types.
- `SceneDiff` / `SceneOp` / `apply_diff` returning `ApplyOutcome`.
- `TransitionSpec` / `TransitionSchedule`: pure host-time sampling.
- `SceneTrace`: a base snapshot and labelled steps, each an optional diff plus an
  opaque host annotation, chain-checked and replayed with `snapshot_at`. There is
  no cursor; that is host policy. A host that wants undo and redo over a trace
  keeps it in its own history (Mere's `backstep` does, and its test of that lives
  in Mere).
- `pick(world) -> Option<InstanceId>`.
