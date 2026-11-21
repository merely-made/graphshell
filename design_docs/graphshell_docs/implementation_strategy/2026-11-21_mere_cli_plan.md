# Mere CLI plan: the query tool, rebuilt on the host toolkit

**Status:** 2026-11-21: plan, nothing built. D1 to D7 ruled.

**For:** the `mere` command-line tool. It queries a linked-data graph locally, or
serves one allowed SPARQL query to an invited peer over iroh, Reticulum, I2P or a
browser over WebRTC. Another session added it to Mere as `crates/cli`
(`eb1fcbec0`, `f8a413d16`, 2026-11-20 and 21). It rebuilt pieces the family stack
already has, and its carriers put murm's old RustCrypto generation into Mere's
engine graph: 52 duplicates that fail Mere's bans check on every commit. Mere main
stays red until this plan lands (D3).

**Not here:** the engine's query API (`mere::query::sparql_on`), which the tool
keeps calling. Mere's engine split (`mere/design_docs/mere_docs/implementation_strategy/2026-11-07_engine_split_plan.md`).
Moving WebRTC signaling onto an admitted carrier or `mer3ly.net` (transport
agnosticism note, T8, and C5 there).

## What it rebuilt, and the stack's piece for each

Read at Mere `aae4d3ba1`, Graphshell `8cc8c6bf`, dramatis and murm main, 2026-11-21.

| The CLI | The stack |
|---|---|
| A random 32-byte bearer token (`getrandom`), compared with `subtle`, in a JSON `Invitation` file. One query is allowed. | `graphshell-host::invitation::Invitation::issue` (invitation.rs:168): an insigne delegation with a capability scope, expiry, nonce and revocation. The browser form is `webrtc_carrier::InviteV1`. |
| u32-length frames, a token frame, then a query frame, a JSON reply, and a 1-byte completion. | notochord `read_frame`/`write_frame` (dramatis, io.rs:79/110), under chirograph's carrier request/response. There is no query body yet (D4). |
| A hand-written match over carriers. When dialling, it connects before `SetupGate::reserve`, and it throws away the `AcceptedSession`. | `graphshell-host::carrier::accept_projection_session` / `admit_stream` (carrier.rs:154/241) and `network_carrier::dial_projection_session`. |
| A WebTransport signaling server on raw `wtransport`, with a self-signed certificate whose hash goes into an HTML page. reader.js re-implements murm-webrtc's initiator and frame decoding. | The WebRTC door: `webrtc_door::{issue_invite, redeem, admit_webrtc_session}` and `webrtc_session::serve_webrtc_join`, bound to DTLS by `LinkChallenge`. The browser initiator is murm-webrtc's `browser` feature. There is no signaling listener yet (D5). |

## Decisions

Mark answered eight question cards on 2026-11-21, plus two free-text answers. Each
entry quotes his answer.

- **D1, the tool lives in Graphshell's host workspace, rebuilt on the stack.**
  Asked how to clear Mere's duplicates, Mark chose "I fix it at the cause". On
  the CLI's own workspace he wrote "sure, but do update the cli. seems like it
  can be a useful tool!", then: "uh, if it's sidestepped the stack, be sure to
  repoint its dependencies through the stack equivalents". Asked where the
  repointed tool lives, he chose "Graphshell host workspace (Recommended)". The
  stack pieces it needs live in graphshell-host, which depends on mere. From
  inside Mere they would bring a second copy of mere. Mere's engine goes back to
  having no carrier.

  Links: summary The mere CLI moves to Graphshell's host workspace and is rebuilt on graphshell-host's invitation, carrier and WebRTC door.

- **D2, missing pieces grow the stack.** Mark chose "Grow the stack now". These
  are the parts the CLI needs that the stack lacks: a query request and a
  signaling listener. They become stack components that the CLI consumes, not
  CLI code.

  Links: summary What the CLI needs and the stack lacks is added to the stack, not kept in the CLI.

- **D3, one landing.** Mark chose "One landing". The tool leaves Mere in the same
  landing that adds it to Graphshell. There is no interim workspace split, so
  Mere's bans check stays red until then.

  Links: summary The CLI leaves Mere and arrives in Graphshell in one landing, with no interim split.

- **D4, a query is a chirograph request.** Mark chose "chirograph Query variant
  (Recommended)". `CarrierRequestBody::Query` and its response are appended last,
  bounded and read-only, the way `RetainedPages` was. An endpoint with no dataset
  refuses it. One admitted carrier session serves projections and queries.

  Links: summary SPARQL queries travel as an appended, read-only chirograph request that an endpoint without a dataset refuses.

- **D5, signaling is lifted from Turnstone's C4 host into graphshell-host.** Mark
  chose "Extract C4's HTTP into graphshell-host (Recommended)". The hand-rolled
  HTTP listener in Turnstone's `ports/graphshell/src/bin/c4_webrtc_host.rs`
  (POST /offer, GET /invite) becomes a `signaling` module behind graphshell-host's
  `webrtc-session` feature. C4 and the tool both consume it. This follows T8
  ("keep HTTP for now"), and `wtransport` leaves the tool.

  Links: summary WebRTC signaling is an HTTP module in graphshell-host, lifted from Turnstone's C4 host and shared by C4 and the CLI.

- **D6, large results are paged.** Mark chose "Paged rows, 512 KiB pages
  (Recommended)". A reply carries up to 512 KiB of encoded rows and a cursor.
  Asking for the next page acknowledges the last one, as `ResourceChunk` does, so
  each reply stays under the 1 MiB carrier frame (`MAX_RESOURCE_CHUNK_BYTES`
  explains the arithmetic). Query text is capped at 64 KiB, the CLI's limit.

  Links: summary Query results travel in 512 KiB pages with a cursor; query text is capped at 64 KiB.

- **D7, an invitation grants one query.** Mark chose "One query, its hash as path
  (Recommended)". The scope's path is the digest of the allowed query, under a
  `query` action. A guest runs exactly that query, and expiry and revocation come
  from the insigne delegation.

  Links: summary An invitation grants one query: the scope path is that query's digest under a query action.

## Phases

Each phase is verified in its own worktree. Nothing reaches main until Q5, which
lands everything together (D3).

### Q1. chirograph carries a query

- `CarrierRequestBody::Query(QueryRequest)` and a matching response, appended
  last, with the query text capped at 64 KiB and the rows paged in 512 KiB
  pages behind a cursor (D6).
- Every exhaustive matcher answers it. Today that is
  graphshell-endpoint and Turnstone's kept port (`ports/graphshell/mere/src/kept.rs`).
  An endpoint without a dataset refuses the request.
- Done when: a round-trip test over the in-memory carrier returns rows, a refusal
  test passes, and the wire form is pinned by a vector.

### Q2. graphshell-host answers a query under a grant

- A resident host with a mere dataset answers `Query` through
  `mere::query::sparql_on`, and the invitation's insigne scope decides whether the
  query is allowed: the scope path is the query's digest (D7).
- Done when: a granted query returns rows, an ungranted or expired one is refused,
  and a revoked invitation is refused, each with its own test.

### Q3. graphshell-host owns the signaling listener

- The `signaling` module is lifted from C4. Turnstone's C4 host consumes it, and
  `c4_host_signaling.rs` still passes.
- Done when: graphshell-host's door loopback test signals through the module, and
  C4's signaling test passes on it.

### Q4. the tool, rebuilt in host/

- It issues invitations with `Invitation::issue`, accepts and dials through
  graphshell-host's carrier functions, queries with Q1/Q2, and signals the browser
  with Q3. `getrandom`, `subtle`, `base64` and `wtransport` leave its manifest.
- Done when: its two existing tests (`local_query_preserves_unbound_and_escaped_cells`,
  `iroh_invitation_limits_access_across_repeated_queries`) pass in their new form,
  Reticulum and I2P keep whatever coverage they have today, and the browser path
  joins through the door.

### Q5. land

- `crates/cli` leaves Mere. Mere's README points to the tool's new home.
- Done when: Mere's bans check passes with 0 duplicates, Graphshell's and
  Turnstone's hooks pass, and everything lands in one push per repository, in
  dependency order.

## Open questions

These come back as cards when a phase reaches them.

- **The browser reader.** The tool's reader.html/reader.js re-implements
  murm-webrtc in JavaScript. The stack's browser initiator is Rust compiled to
  wasm. Which page does a browser reader open, and who builds it?
- **Where the tool sits in host/.** A port beside `mere-resident`, or a crate
  under `crates/graphshell/`.

## Progress

- 2026-11-21: plan written. D1 to D5 ruled, then D6 and D7.
