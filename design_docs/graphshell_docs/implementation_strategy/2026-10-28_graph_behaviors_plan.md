# Graph behaviors plan: watches, cascades, resident runs, and where mod authoring lives

**Status:** 2026-10-28: cut from `2026-08-13_graph_behaviors_plan.md` and
`2026-06-30_runtime_mod_authoring_loop_plan.md`, archived in `archive_docs/2026-10-28/`. Landed:
slices W0-W5 (watch table and matcher, region vocabulary, cascade runner, drain wiring, trigger
context, containment derived on mint, app-tier and time watches, the flagship summarizer) with a
green headed receipt (`behaviors_wake.scn`), and, in source, Servitor's R1a resident admission and
R1b run reducer with Turnstone's adapter. Open: R1 acceptance and durable effects, the
cursor-before-bodies recovery seam, three missing headed receipts and a clipped review row, R2 and
R3 (planned, no `ProcedureGraph` types), and every phase of the runtime mod authoring loop, whose
Meerkat host is gone.

**For:** behaviors as participants with a trigger: the participant gate answers "may it write", this
plan answers "when does it run" and how a resident run is admitted, recorded and recovered (Mark's
"neat lil ideas" conversation of 2026-08-13); and the mod authoring loop, create a mod, let a model
edit it, check, build, load at runtime, test against a fixture and reload, with Lua as the local
command lane (Rhai until 2026-11-23) and Wasm components as the portable untrusted lane (2026-06-30).
Where the Lua command packs and the authoring pane live now that Meerkat is gone is an open question
here, together with the overlap with the Lua command lane in `script-didge`.
Not here: the workshop composition and the first retained-passage evidence-brief workflow in
Turnstone and Graphshell (`2026-10-28_alembic_implementation_plan.md`; R2 and R3 do not gate it);
projection-tier rules such as nearby refresh, staleness badges and rollup counts, which field
regions own (`2026-10-28_dynamics_grammar_plan.md`); Servitor itself, which stays in Eidetic
(Alembic D8); custody's signing and lock contract (`repos/dramatis/design_docs/2026-10-28_custody_plan.md`); cross-app and
peer-carried triggers across murm or retinue, which the source calls "its own future plan" (not yet
written), with the cross-app woodshed practice-event row placed in the murm lane and out of scope.

Receipts from this plan's runs go to `archive_docs/<date>/`, never into this plan (DOC_POLICY §8).
Evidence that matters is a test in the repository or a line in Findings.

## Constraints

- One test splits the tiers: does it write graph truth? Projection rules only change how things look
  and need no gate; a graph behavior writes truth and is a participant with a standing subscription.
  Everything the gate plan ruled carries over: one authority model, petitions through
  `servitor::Gate`, no raw positions (spatial influence goes through fields), auditability plus
  compensating actions rather than universal undo
  (`archive_docs/2026-10-28/2026-08-13_graph_behaviors_plan.md:49-80`).
- Design (`:123-234`): a watch scope is within the read scope, within the grant, enforced at
  registration; a watch matches committed, attributed deltas through a per-watch journal cursor,
  never live mutation; self-authored entries never wake their author, and the journal's subject
  label convention is a registration argument; the cascade runs post-commit at the after-dispatch
  drain in stable subject order with a bounded round budget, exhaustion being loud; time is an
  injected watchable source, never wall time read by a body; a behavior may not watch another
  participant's grant projections, emit decorative notifications or write derived indexes (cited-by
  counts, backlinks) as truth.
- Rulings (`:480-490`): the noun is "watch"; watches are reviewed at install in the pack manifest
  beside the rings, and a later addition is a widening and re-reviews; the cascade budget is a
  setting, default 4 rounds, live-switchable, floor 1, no unlimited value, taking effect on the next
  cascade without a restart.
- No new scripting language or lane. Since 2026-11-23 (`inker:lua-evaluator` L6) the lanes are:
  Lua on didgeridoo for privileged local automation (`script-didge`: note fences, presentation rules,
  `lua-command` batches) and for sandboxed participant bodies (Genet's piccolo engine), and Wasm
  components for portable untrusted mods. Rhai is retired; an old `rhai` fence is refused with a
  message naming the Lua form (`inker:lua-evaluator D1`, D27; Mark: "Decide now: Lua replaces
  Rhai"). A new capability never rides an existing preselected grant, which would silently widen
  installed packs; authoring earned its own ring (`:62-80,417-435`). The one exception is a grant
  the user took as open-ended: a command pack's class grant covers commands the host files under
  that class later (`command-packs D3`).
- Non-goals (`:467-478`): operative edges or edge-kind variants, a general scheduler (the cascade
  runner is a bounded drain loop; depth machinery waits for evidence), spatial influence through
  petitions, universal undo.
- Servitor boundary (`:492-570`): module boundaries stay in the existing Servitor crate;
  authority-only consumers (Gemot, document-host) must not acquire an ESP, model, thread-runtime or
  host dependency; another crate is extracted only when dependency direction or a real independent
  consumer requires it; the name `denizen` belongs to the Isometry simulation family.
- Run contract (`:570-690`): explicit run states (ready, deciding, awaiting a host result, paused,
  reconciling an uncertain effect, terminal) with distinct terminal reasons; a wake records manual
  invocation, journal identity plus matched positions, or a host-clock tick; the overlap setting
  rejects, queues or coalesces; every outbound request carries run, step, attempt and binding
  generation, and the reducer rejects duplicate, stale and post-cancellation results; intent is
  recorded before dispatch and the result after; replay never resends effects and a crash between an
  effect and its receipt enters reconciliation, never blind retry; grants stay live, so every
  consequential dispatch rechecks authority and a reloaded watch does not prove read authority;
  budgets (decisions, tokens, tool calls, elapsed host time, consecutive failures) survive pause and
  restart; slow model work yields rather than hold a synchronous drain open. Procedures are optional
  and advisory: success cites observations, never a model's assertion; refinement yields an inert
  candidate; adoption is compare-and-adopt against binding version, evidence applicability and
  authority, a delayed result never overwriting a newer adoption, active runs keeping their pinned
  versions, and no adoption rewriting grants.
- Mod loop
  (`archive_docs/2026-10-28/2026-06-30_runtime_mod_authoring_loop_plan.md:13-30,76-145,179-213`):
  the shared contract is the host capability surface, not a source language; a command pack (Rhai
  in the source, Lua now) never receives a live host handle (read a context snapshot, return requested actions, the host
  validates and executes); a mod holds only what its manifest requested and host policy granted,
  with filesystem, network, storage, personal mesh jobs, private graph reads and credentialed
  fetches granted separately; a new GPU or web engine is a native adapter unless it lowers into a
  parser or viewer world; model output never becomes ambient authority.

## Phases

### GB1. R1 acceptance and durable effects
R1a (resident binding and run admission) and R1b (the shared run reducer and outcomes) are
implemented in Servitor's `resident` and `run` modules with Turnstone's adapter. What remains before
end-to-end durable effects can be claimed: correlated port completion callbacks and a fallible
graph-persistence acknowledgement, because `Effect::SaveSession` is a request to the shell and not a
durable acknowledgement; strict effect tracking is a host option, the default labelling returned
effects as released for untracked handoff; no settings UI exists for run controls.
- Done when: R1a, unique instance routing, explicit pause and backlog disposition, current read and
  write authority and legacy identity preservation are proven, and revocation and lifecycle evidence
  persist so reopening cannot revive denied work. R1b, bounded work, cancellation, stale-result
  refusal, per-commit attribution, durable intent and result ordering and replay without repeat
  effects are proven, and uncertain-outcome reconciliation is exercised before resumable external
  effects are enabled.
- Needs: GB2; correlated completion callbacks and a graph-persistence acknowledgement from the
  consuming hosts.
- Rulings: behaviors 1-3
- From: `archive_docs/2026-10-28/2026-08-13_graph_behaviors_plan.md:693-725,768-825`

### GB2. Recovery and authority seams
Turnstone's `drain_graph_tier` advances `behavior_cursor` to the tail boundary before running
bodies, so a crash between loses wakes (cursor advancement and disposition must be durably
associated). Turnstone composes the content of each root delegation and custody signs it on a
delegation-salt rule without a petition the owner reviews in custody (the resident delegation
petition note). Wake and run routing finds a resident by subject, ambiguous for identical packs:
import must bind the instance or refuse. Turnstone's synchronous lowering changes the journal's
current author, so carrying that author across asynchronous waits would misattribute concurrent
work: per-commit run attribution is needed.
- Done when: not stated as a condition in the source; the gaps it names are closed, the cursor and
  its disposition advancing together durably, a root delegation passing a petition the owner reviews
  in custody, import binding the instance or refusing ambiguity, and each commit carrying its own
  run's attribution across awaits.
- Needs: custody plan for the petition review.
- Rulings: -
- From: `archive_docs/2026-10-28/2026-08-13_graph_behaviors_plan.md:525-549,602-605,637-639,833-837`

### GB3. Watch-slice follow-ups
The clock tier (which needs a scenario clock verb), the app-tier watches and the budget setting have
no headed receipt of their own. The review row clips in the palette ("wakes on: fi...") because the
under-96-character guard in `denizen.rs` measures characters, not rendered width. The W2.5
consequences stand: a `mere://` folder derives no containment, a folder addressed without its
trailing slash contains nothing so its watch is silently inert, and the Domain half stays a
whole-graph rebuild. The W5 digest is plain text until the intel seam replaces the prose. The
candidate catalog's graph-behavior ideas with no slice: auto-tag on content, watch-and-notify,
rollup written back as node content, template expansion on drop and gardening (petition to archive
orphans).
- Done when: the clock, app-tier and budget slices each have a headed receipt and the review row
  reads at rendered width (the status's follow-ups); the other items state no done-condition.
- Needs: -
- Rulings: behaviors 1-3
- From: `archive_docs/2026-10-28/2026-08-13_graph_behaviors_plan.md:21-29,304-305,344-356,420-421,446-465,925-932`

### GB4. R2 optional procedural guidance (planned)
A `ProcedureGraphV1` of registered actions, observable checks and task states, with guidance and
pitfall edges, validated against registry ids and schema versions, locally retrievable within
bounded depth, history window and token cap.
- Done when: artifact validation and bounded local retrieval work; two residents reuse one version
  with different grants and independent progress; flat context is compared with procedural guidance
  on the same task set, recording success, invalid or repeated actions, steps, tokens and latency;
  invalid action ids and fabricated completion fail closed; a live ESP provider supplies a measured
  receipt before any model benefit is claimed.
- Needs: GB1; a later extension point that does not gate the Alembic two-host workflow.
- Rulings: -
- From: `archive_docs/2026-10-28/2026-08-13_graph_behaviors_plan.md:655-692,707-712,722-723`

### GB5. R3 evaluated adoption (planned)
Candidates are created from recorded failures and evaluated on separately held-out tasks; the
owner's adopted policy chooses explicit review or bounded automatic adoption with declared
thresholds.
- Done when: rejection and owner-authorized adoption are both exercised; evidence and version
  mismatch refuse; adoption races, unchanged active-run versions, and cancellation or revocation
  while evaluation is outstanding are proven; no statistical improvement is claimed from one
  deterministic fixture and validation tasks are never reused as the final test set.
- Needs: GB4
- Rulings: -
- From: `archive_docs/2026-10-28/2026-08-13_graph_behaviors_plan.md:679-692,713-718,722-723`

### GB6. The runtime mod authoring loop (host open)
Five phases, none started, none of the plan's objects existing (`install_mod`, `CommandModManifest`,
`ModProject`, `meerkat.mod.*`). P1 Lua command packs (Rhai in the source; now its own plan,
`2026-11-24_command_packs_plan.md`): dynamic command entries from a manifest, a
`CommandContext` snapshot, an action-request return type, install, reload, unload and run commands,
palette and menu exposure; the diagnostic names (`meerkat.mod.*`) need replacing. P2 to P5
and the tails below are now `2026-11-25_mod_projects_plan.md`. P2 Wasm mod
projects: a `ModProject` store, a component build and check command, attach, run and reload through
`DocumentScript`, a fixture runner and logs. P3 the model-assisted authoring pane: patch loop,
check, build and run buttons, trap display, rollback to the last passing build. P4 parser and viewer
mod worlds with package and install from the project pane. P5 document what cannot be runtime-loaded
in a browser and keep GPU and web engines behind native adapter seams. Nothing auto-binds installed
mods: `discover_wasm_mods_in_dir` has no production caller and no `<mere_root>/mods/` path exists.
Tails from the document-script net hardening plan: mod install, approval and optional signing (its
E3), `.cwasm` mods invisible to discovery (its B4), uncredentialed same-origin `net.fetch`, and
cross-origin `net` declarations for mods with broad-glob guards.
- Done when: a Lua command pack installs at runtime and appears in the palette without a Rust enum
  variant; the command gets a read-only context and returns an action request; an unauthorized
  request is rejected with a visible reason; reloading changes behaviour without a restart; a Wasm
  `DocumentScript` project is edited, checked, loaded, attached, exercised with a fixture and
  reloaded from one pane; a trapped mod is detached or rolled back and the logs name the failing
  capability, import or turn; a model patches a project and drives the same loop the user sees. The
  two conditions that name Meerkat (restart, one pane) cannot be met as written.
- Needs: the host question below; the settings-lane grant UI and `net.fetch` hardening, carried by
  an archived follow-ons plan.
- Rulings: -
- From: `archive_docs/2026-10-28/2026-06-30_runtime_mod_authoring_loop_plan.md:4-11,50-70,145-175,258-312`;
  tails `archive_docs/2026-10-28/2026-07-03_archived_plan_tails_plan.md:85-89,93-101`

### GB7. Participant gate leftovers
The participant gate packs plan, which this one extends, left three items: its B5, a tessera receipt
on a live moot, the mere-native meadowcap structural-cap layer, and "broader consumer admission".
- Done when: not stated in the source (see the question below).
- Needs: -
- Rulings: -
- From: tails
  `archive_docs/2026-10-28/2026-07-03_archived_plan_tails_plan.md:1184-1193`

## Decisions

- **D1. Command packs live in the host toolkit's mod registry (Mark, 2026-11-23):** Evidence:
  Meerkat, P1's host, is gone; the host toolkit's `mod_loader` registry
  (`host/crates/script/document-host`) has no production caller; Turnstone has its own
  `engine_pack`. Offered: the host's `mod_loader`, palette entries drawn from it so every app on the
  host gets packs (recommended); Turnstone first, lifted later; keep P1 as a record. Answer:
  "Graphshell host mod_loader (Recommended)". What follows: P1 builds in the host toolkit, not in an
  app; the open question's "where" half is answered. Outcome: new.

  Links: area The stack · plans graph-behaviors GB6 · summary Lua command packs install into the host toolkit's mod registry, and every app's palette reads them from there.

- **D2. A pack command runs through `lua-command` (Mark, 2026-11-23):** Evidence: `script-didge`'s
  `collect_invocations` runs a chunk under fuel against a host-supplied `CommandSet` and returns
  invocations without executing them; Isocosm's generators and Mesocosm's expression packs, on the
  same Lua fork, return proposals the host validates (`inker:lua-evaluator` Findings 34). Offered:
  reuse `lua-command` over a read-only `CommandContext`, each returned request checked against the
  pack's grants and a refusal shown with its reason (recommended); a new lane with direct host
  functions; replace the fence lane with packs. Answer: "Reuse lua-command (Recommended)". What
  follows: the open question's "reuse, supersede or stay separate" half is answered; P1's
  action-request return type is the invocation batch. Outcome: new.

  Links: area The stack · plans graph-behaviors GB6 · summary A command pack's Lua proposes invocations through lua-command; the host checks them against grants and decides.

- **D3. A pack's admission is a guarantee the user chooses (Mark, 2026-11-23):** Evidence: Mesocosm
  admits a pack all or nothing (an ABI number, `deny_unknown_fields`, declared files only, qualified
  unique ids, named refusals, fixtures pinning the exact proposal); Isocosm loads what parses and
  skips duplicates with a diagnostic (`inker:lua-evaluator` Findings 34). Offered: Mesocosm's rules
  (recommended); Isocosm's looser discovery; decide when P1 starts. Answer: "those both sound like
  levels of guarantees which the user should be able to choose between ("trusted" or "conformant"
  pack vs "untrusted" "debug" "dev mode" pack). sounds like we just need to communicate the
  difference depending on what the user is dealing with, not make the choice for them (annoying!)".
  What follows: both levels exist; a pack carries the level it was admitted at and the host says
  which, rather than refusing for the user. Outcome: new.

  Links: area The stack · plans graph-behaviors GB6 · summary A pack is admitted at a level the user picks, conformant or dev mode, and the host shows which level it holds.

- **D4. A pack is conformant unless the user chooses dev mode (Mark, 2026-11-23):** Evidence: D3
  leaves the level unset when a user installs without choosing. Offered: conformant, with dev mode
  chosen per pack or folder and a strict refusal offering dev mode with its failures listed
  (recommended); by origin, local projects in dev mode; ask at every install. Answer: "Conformant,
  dev mode by choice (Recommended)". Outcome: refines D3.

  Links: area The stack · amends graph-behaviors D3 · plans graph-behaviors GB6 · summary An installed pack is held to conformant admission unless the user switches it to dev mode.

- **D5. Dev mode loosens admission, never grants (Mark, 2026-11-23):** Evidence: D3; the lane
  constraint above says a new capability never rides an existing grant. Offered: admission only,
  undeclared files tolerated, duplicates skipped with a diagnostic, what parses loaded, grants
  unchanged (recommended); admission plus prompt-free grants for local projects; both behind a
  banner. Answer: "Admission only (Recommended)". Outcome: refines D3.

  Links: area The stack · amends graph-behaviors D3 · plans graph-behaviors GB6 · summary A dev-mode pack is admitted loosely but gets no capability the user has not granted.

- **D6. The level shows in the pack manager (Mark, 2026-11-23):** Evidence: D3 asks that the
  difference be communicated. Offered: a badge on the pack and its palette commands with the strict
  failures listed (recommended); the pack manager only; a badge without diagnostics. Answer: "Pack
  manager only". What follows: the palette carries no level marker; the pack manager shows each
  pack's level. Outcome: refines D3.

  Links: area The stack · amends graph-behaviors D3 · plans graph-behaviors GB6 · summary A pack's admission level is shown where packs are managed, not in the palette.

## Open questions

- Answered for command packs 2026-11-23 (D1, D2); the authoring pane's home and the mod plan's role
  stay open. Where do the Lua command packs and the authoring pane live now that Meerkat is gone: retarget
  onto a living host (Turnstone or Cambium, with the registry's `mod_loader`), or keep the mod plan
  as a historical design record, the authoring-ergonomics source that this plan and the participant
  gate packs plan cite? And how does P1 relate to `script-didge`'s command lane
  (`collect_invocations`, which replaced Hest H4's `script_rhai::command`), which already collects
  command invocations through the shared sandbox from a host-supplied `CommandSet` and registers a
  separately allowlisted `lua-command` block evaluator that returns invocation data and never
  executes it: reuse it, supersede it or stay separate? From:
  `archive_docs/2026-10-28/2026-06-30_runtime_mod_authoring_loop_plan.md:43-55,297-312`;
  `archive_docs/2026-10-28/2026-10-20_hest_command_architecture_plan.md:174-190`
- Does the repin to mere `bd5912fb` plus the Cargo runs close R1a and R1b validation, or does R1
  stay open until headed and crash-recovery receipts exist? From:
  `archive_docs/2026-10-28/2026-08-13_graph_behaviors_plan.md:768-772`
- Is "broader consumer admission" a phase with a done-condition, or dropped as met by the three
  actor kinds (script, wasm component, moot peer) that already petition through the one gate? From:
  tails `archive_docs/2026-10-28/2026-07-03_archived_plan_tails_plan.md:1188-1193`

## Findings

- 2026-10-25: since Turnstone `5d2c910`, `rebuild` adopts a stored certificate chain only if it
  verifies under the current root and otherwise refuses; the only reissue is the owner-reviewed
  `install`, which bumps the lifecycle generation, and `resident_admission`'s lifecycle record tells
  revoked and paused from a migration heal. The cursor-before-bodies seam remains (`:506-549`).
- 2026-08-13: a real fetch rewrites a node's URL out from under the watch that resolved to it, so an
  address, not a mechanism, has twice broken a watch; running a body re-enters the drain (a
  `draining` flag makes the tiers structural); a constant digest journals nothing after the first
  write (`:417-435,905-932`).
- 2026-10-06: the omnibar Rhai lane left with Meerkat; the shared sandboxed engine in
  `inker/crates/script/rhai/src/lib.rs` holds. `DocumentScript` attaches a Wasm component to a focused
  page and gates imports before instantiation (mod plan `:31-70`).

## Progress

- 2026-10-28: cut from the two sources named in Status; nothing new landed.
- 2026-11-23: Rhai retired (`inker:lua-evaluator` L6). The lane constraint, GB6 and the host
  question read Lua; the command lane is `script-didge`'s `lua-command`.
- 2026-11-25: GB6 P2 to P5 and their tails cut into `2026-11-25_mod_projects_plan.md` (mod-projects
  D1 to D11).
- 2026-11-24: GB6 P1 cut into `2026-11-24_command_packs_plan.md` (command-packs D1 to D5); the lane
  constraint names open-ended class grants.
- 2026-11-23: D1 to D6 ruled for GB6 P1: packs live in the host's `mod_loader`, run through
  `lua-command`, and are admitted conformant unless the user picks dev mode, which loosens
  admission and never grants; the pack manager shows the level.

## Received from the backlog holder, 2026-10-28

Items the archived-plan-tails plan held with no owner, given this one when the active plans were cut; each is as the holder recorded it, not re-verified.

- True non-blocking fiber suspension for `net.fetch`, E2 of the DocumentScript net hardening. From: `archive_docs/2026-10-28/2026-07-03_archived_plan_tails_plan.md:90-92`
- `Fetched` carries no HTTP status (D2 of the same hardening). From: `archive_docs/2026-10-28/2026-07-03_archived_plan_tails_plan.md:102-103`
- A DNS-rebinding defence for `net.fetch`: resolve, then pin. From: `archive_docs/2026-10-28/2026-07-03_archived_plan_tails_plan.md:104-105`
