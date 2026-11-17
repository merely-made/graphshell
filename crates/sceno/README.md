# sceno

Core contracts for the scenograph projection engine: source references,
coordinate spaces, footprints, persisted scores, and scene instances carrying
representation slots and emphasis channels. It depends on `serde` and
`serde_json` only.

`sceno` is part of the Graphshell protocol layer
(`https://github.com/merely-made/graphshell`). The choreography compiler that
realizes a score into an arranged scene (`scenomise`) and the authoring
definitions (`scenograph`) live under this repository's
host/crates/cambium/scenes directory. The runtime that
diffs, transitions and picks scenes is [`scenotime`](../scenotime).

- `SourceRef` / `SourceIx`, `Space` / `SpaceId`, `InstanceId`, `Backdrop`,
  `Footprint`, `Representation`, `ProjectedItem`, `RoutedRelation`, `Region`,
  `Fold`, `Scene`.
- The persisted `Score`, `ScoreItem`, `Arrangement`, `Placement` and
  `SCORE_VERSION` vocabulary, including optional `Frame` and `FrameKind`.
- `SceneFrame` binds an authored frame to its live coordinate `Space`.
- The geometry types `Vec2`, `Size2`, `Rect`, `Transform2`.

Here a **recipe** is authored configuration, a **score** is resolved layout
instructions, and a **scene** is the placed projection.


A score can choose an unbounded origin, a bounded rectangle, a nested rectangle
through a Space, or a page width whose height grows with content. Arrangements
declare the kinds they accept. An explicit frame supplies their effective
geometry; missing frames retain legacy family bounds and origins. Those fields
remain compatibility inputs for older Scores.

Score version 6 records the frame slot. Current readers preserve older Score
placements, and older readers reject version 6 rather than silently drop a
frame. A scene retains its instantiated frame so routing, motion and snapshots
can keep the same domain. Nested frames read the live Space transform after
group movement.
