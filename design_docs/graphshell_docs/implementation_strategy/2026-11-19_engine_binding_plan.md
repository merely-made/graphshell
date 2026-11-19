# Engine binding plan: a host view names the Mere revision it shows

**Status:** 2026-11-19: plan. Nothing started. The binding's shape is still an open lane of
`mere:engine-split P8`; this plan holds the host's done-conditions and decides none of that shape.

**For:** Graphshell's half of the engine-to-workspace contract
(`mere/design_docs/mere_docs/technical_architecture/2026-11-16_engine_workspace_contract.md`). The
contract says that the exact dataset revision bound to a rendered view accompanies its keyframe and
deltas, and that an intent against a stale bound revision is refused. It also records that the host
does not do this yet. No Graphshell document named the contract before this one.

**Not here:** the contract and its rulings, which stay in Mere's engine split plan
(`mere/design_docs/mere_docs/implementation_strategy/2026-11-07_engine_split_plan.md`, P8; D147 to
D149, D158, D161, D163, D166, D169, D179, D181); the kernel's surface half moving here (D146, the
same phase); how graph-stored arrangement facts and device-local attention share a host model (the
contract's open question); a trail's RDF form (`mere:engine-split D182`, P7).

## Constraints

- The engine's revision is the pair (session, journal cursor) locally and the v2 journal-prefix
  digest wherever it crosses a process or device (`mere:engine-split D148`). The host mints no
  revision identity. Its projection counter, if kept for cache invalidation, derives from the
  engine's revision and stays scoped to its view (the contract, "Ownership").
- The host owns its `PinnedView`, its lifetime and its release cost (`mere:engine-split D179`).
- A keyframe is a versioned CBOR envelope around the existing projection snapshot, and a host that
  misses a delta resumes or takes a fresh keyframe (`mere:engine-split D181`). The envelope's
  schema, its vectors and the retention bound are open there, and the bound is a number for Mark.
- The Resource digest is measured before it is used on the edit path (`mere:engine-split D181`).

## Phases

### E1. A scene names the engine revision it was built from
`MereHost` takes a pinned view and builds its scene from it, and the snapshot it hands out carries the
pin's session, cursor and digest beside the projection epoch and revision.
- Done when: a snapshot built from a pinned view carries that pin's session, cursor and digest, and
  the same test fails when the scene is built from a different cursor.
- Needs: Mere's `PinnedView` (landed); the envelope's schema.

### E2. An intent from a superseded view is refused
- Done when: with a pinned view open, the live graph is edited by another path, and an action sent
  from the old view comes back stale, naming the engine revision now current; the same action from a
  current view applies.
- Needs: E1.

### E3. A missed delta is recovered
- Done when: with one delta dropped in transit, the host sees the gap before it applies the next
  one, resumes from its acknowledged identity and epoch, and ends on the scene an undisturbed host
  holds; with the suffix out of retention it receives a fresh keyframe; every base and target is
  checked before a delta applies.
- Needs: E1; the retention bound.

### E4. Rearranging leaves the Resource digest unchanged
The host's side of P8's third probe.
- Done when: the Resource digest reported with a view is identical before and after an arrangement
  edit made in Graphshell, and differs after an edit to a Resource statement.
- Needs: the digest's measurement on the edit path.

### E5. The inspector shows a validation finding
Graphshell's part of the shared milestone
(`mere/design_docs/mere_docs/implementation_strategy/2026-11-07_engine_split_plan.md`, "Shared
milestone: a page's claims to a SHACL finding"), cited and not copied.
- Done when: the shared inspector shows a finding with its focus node and path beside the evidence
  rows it concerns, and opening it selects the claim's evidence row; for the milestone's second
  pass, the finding is unchanged in a pinned view after the live graph is edited (needs E1 and E2).
- Needs: Mere's SHACL call and its finding-to-evidence read (an open question in that plan).

## Open questions

- **A shape Graphshell already writes.** The table plan's T1 round-trips a fields list through a
  SHACL node shape. Whether the milestone's shapes graph should be one of those, so that a table of a
  page's claims is validated by its own fields list, has not been asked.

## Findings

- 2026-11-19, Graphshell `9b24775c`: no source file names `PinnedView`. In
  `host/crates/graphshell/graphshell-host/src/mere_host.rs` a scene's generation and revision are
  stamped from `self.projection_revision` (1416-1419), and the stale check compares
  `projection_epoch` and `projection_revision` (1578-1584). The contract's "Engine to host" section
  records the same at an earlier commit.

## Progress

- 2026-11-19: plan opened from a review's first point, folded in at Mark's word with Mere's shared
  milestone (`mere:engine-split D184` to `D186`). Nothing built.
