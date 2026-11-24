# Engine binding plan: a host view names the Mere revision it shows

**Status:** 2026-11-23: E1, E2 and E4 are built: a scene names its engine revision, a stale intent
names the revision now current, and `MereHost` reports a pinned view's Resource digest on request
(D1, D2). The E5 finding panel and native kept-source binding are built. E3 and the general scene
binding remain open lanes of `mere:engine-split P8`.

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
- Needs: Mere's `PinnedView::resource_digest` (landed, `mere:engine-split D190` to `D194`). The
  digest stays off the edit path: a pin computes it on the first ask (`mere:engine-split D191`).
- Rulings: D1, D2.

### E5. The inspector shows a validation finding
Graphshell's part of the shared milestone
(`mere/design_docs/mere_docs/implementation_strategy/2026-11-07_engine_split_plan.md`, "Shared
milestone: a page's claims to a SHACL finding"), cited and not copied.
- Done when: the shared inspector shows a finding with its focus node and path beside the evidence
  rows it concerns, and opening it selects the claim's evidence row; for the milestone's second
  pass, the finding is unchanged in a pinned view after the live graph is edited (needs E1 and E2).
- Needs: Mere's SHACL call and its finding-to-evidence read (an open question in that plan).

The native L1 slice uses `graphshell_host::pinned_shacl`. It reads one named capture's claims and
block evidence from a `PinnedView`, validates that capture's graph, and holds the pin with the
report and engine revision. Before opening evidence, the caller reads its authoritative session
head and supplies it to Graphshell's revision check. Historical reports stay readable when their
evidence actions become stale. The scene envelope and its intent routing remain in E1 and E2.

## Decisions

- **D1. The host reports the Resource digest through an accessor, not in the snapshot (Mark, 2026-11-23):** Mere exposes `PinnedView::resource_digest` (`mere:engine-split D192`, D194), computed lazily once per pin at about 3 s for 100k nodes (D191), and no Graphshell host code reports it yet. "Host accessor (Recommended)": `MereHost` returns the digest with its algorithm id and version on request, the asker paying the lazy cost once per pin, and snapshots stay as cheap as they are; against an optional snapshot field filled only when the pin has already computed it, whose presence would depend on who asked first, and a digest in every snapshot, which breaks D161's 100 ms edit target. Outcome: *New.*

  Links: draws on mere:engine-split D161, mere:engine-split D191, mere:engine-split D192 · plans engine-binding E4 · summary MereHost reports the Resource digest through an accessor on request; snapshots do not carry it.

- **D2. The digest accessor is sync, borrows Mere's type and errors without a pin (Mark, 2026-11-23):** `MereHost::pinned()` is public; the accessor adds a named entry point, behaviour without a pin and the host's error. "Take the combination (Recommended)": `pinned_resource_digest(&self) -> Result<&ResourceDigest, MereHostError>`, sync like every pin method, borrowing `pandect::ResourceDigest`, with `MereHostError::NotPinned` and `MereHostError::ResourceDigest(SortedNQuadsError)` and `dep:mere` under the `mere-host` feature; against `Ok(None)` with a cloned digest and a copied-out quad, and an owned `scenotime::ResourceDigest` ahead of D181's envelope schema. Outcome: applies D1.

  Links: draws on engine-binding D1, mere:engine-split D192 · plans engine-binding E4 · summary pinned_resource_digest is sync, borrows Mere's ResourceDigest, and errors with NotPinned or the wrapped SortedNQuadsError.

## Open questions

- **A shape Graphshell already writes.** The table plan's T1 round-trips a fields list through a
  SHACL node shape. Whether the milestone's shapes graph should be one of those, so that a table of a
  page's claims is validated by its own fields list, has not been asked.

## Findings

- 2026-11-23, Mere `ac883e75c`: no host path puts a blank node in the Resource stratum. The
  projection mints named nodes only and skips a term whose IRI does not parse
  (`crates/graph/engine/src/projection.rs:62,180,245`, `crates/graph/graph-kernel/src/rdf.rs:61,179`),
  and ingest skolemizes blank subjects and objects (`crates/graph/engine/src/acquire/ingest.rs:211`,
  `acquire/ingest/contribution.rs:68,343`). E4's refusal test therefore checks Mere's real
  `SortedNQuadsError` becoming `MereHostError::ResourceDigest`, not a refusing pin.
- 2026-11-23: `mere-host` is documented as native and wasm alike, but graphshell-host declared its
  optional `mere` in the native-only table. D2's `dep:mere` moved it to the all-target table, as
  pandect already is; `mere-host` already pulled `mere` through pandect on every target, and the other
  features that name `dep:mere` (`mere-query`, `pinned-shacl`) gate their code to native. No gate
  builds `mere-host` for wasm32.

- 2026-11-19, Graphshell `9b24775c`: no source file names `PinnedView`. In
  `host/crates/graphshell/graphshell-host/src/mere_host.rs` a scene's generation and revision are
  stamped from `self.projection_revision` (1416-1419), and the stale check compares
  `projection_epoch` and `projection_revision` (1578-1584). The contract's "Engine to host" section
  records the same at an earlier commit.

## Progress

- 2026-11-23: E4 built (branch `e4/digest-accessor`). `MereHost::pinned_resource_digest` returns
  the shown pin's `pandect::ResourceDigest` (`mere-resource-sorted-nquads-blake3` v1), sync and
  borrowed from the pin, with `MereHostError::NotPinned` and `MereHostError::ResourceDigest` (D2).
  `mere_host_digest_tests.rs`: pins taken before and after a FrameMember arrangement edit report the
  same digest while the journal cursor, journal digest and whole projection moved (the positive
  control); a pin after a Resource statement edit reports a different one; equality asserted across
  a Resource edit fails (`should_panic`); the host hands back the digest the pin keeps; the live view
  has none; Mere's blank-node refusal becomes the host's error with its quad.

- 2026-11-22: E2's stale reply names the engine revision (`mere:engine-split D193`).
  `IntentResult::Stale` carries an optional `current_engine_revision`, absent on the wire when
  unset so older readers see the reply they knew; `IntentResult::stale_at_engine` builds it, and
  Graphshell's 25 literal stale replies now go through the constructors. `MereHost` names the live
  engine revision in both stale branches, so a reply refused only because the engine moved no
  longer repeats a counter the caller already holds. The pin test checks both branches, and a
  chirograph test reads a pre-D193 reply and round-trips one that names the revision.
- 2026-11-20: E1 and E2 built (`19bd3652`), probe 3's content check (`1b6b9793`). `MereHost` holds an
  optional `PinnedView` and builds scenes from the pin's graph or the live graph, and every snapshot
  carries the engine revision it was built from (`mere:engine-split D187` to `D189`). An intent is
  refused as stale when the engine revision or the projection counter moved; the reply does not yet
  name the engine revision. Probe 3's content check passes: an arrangement edit made in Graphshell
  leaves every Resource-stratum quad unchanged. The Resource digest itself is not built: RDFC-1.0
  refuses the projection's triple terms and a whole recompute takes seconds at 100k nodes, which
  went back as forks.
- 2026-11-20: The native host reads a kept source's validation report and block evidence from
  one pin. The focused regression edits the same source's Resource facet through the journal,
  reopens the session, and checks that the old pin keeps its evidence while the old action is
  refused. The native Turnstone round trip is a product gate.
- 2026-11-19: The shared Cambium finding panel displays the focus node, path, severity and
  message beside the evidence offered by its caller. Opening the finding selects its sole
  match when there is one, or offers a button per candidate when there are several. The
  caller owns the current selection and resolves the evidence handle. Cambium's 456 Windows
  library tests pass, including keyboard activation, multiple candidates and a finding with
  no matching evidence. E5's native product round trip and pinned-view acceptance remain open.
- 2026-11-19: plan opened from a review's first point, folded in at Mark's word with Mere's shared
  milestone (`mere:engine-split D184` to `D186`). Nothing built.
