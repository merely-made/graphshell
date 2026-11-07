# sceno

Core contracts for the scenograph projection engine: source references,
coordinate spaces, footprints, persisted scores, and scene instances carrying
representation slots and emphasis channels. It depends on `serde` and
`serde_json` only.

`sceno` is part of the Graphshell protocol layer
(`https://github.com/merely-made/graphshell`). The choreography compiler that
realizes a score into an arranged scene (`scenomise`) and the authoring
definitions (`scenograph`) are not here; they live in Mere. The runtime that
diffs, transitions and picks scenes is [`scenotime`](../scenotime).

- `SourceRef` / `SourceIx`, `Space` / `SpaceId`, `InstanceId`, `Backdrop`,
  `Footprint`, `Representation`, `ProjectedItem`, `RoutedRelation`, `Region`,
  `Fold`, `Scene`.
- The persisted `Score`, `ScoreItem`, `Arrangement`, `Placement` and
  `SCORE_VERSION` vocabulary.
- The geometry types `Vec2`, `Size2`, `Rect`, `Transform2`.

Here a **recipe** is authored configuration, a **score** is resolved layout
instructions, and a **scene** is the placed projection.
