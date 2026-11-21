# Mere CLI plan: the query tool, rebuilt on the host toolkit

**Status:** 2026-11-21: Q1 to Q3 built on branch `cli-q1`, unlanded. D1 to D21 ruled.

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

Mark answered twenty-four question cards on 2026-11-21, plus four free-text answers. Each
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

- **D8, native readers are invited by name, co-op's way.** Mark chose
  "Guest-bound, co-op's way (Recommended)". The reader sends the owner a
  contact (`Contact::create`). The owner issues an `Invitation` and a `Card` to
  that contact, and revokes per guest. The CLI's bearer token goes. The door's
  bearer redemption stays bound to DTLS and to browsers.

  Links: summary Native readers send a contact and receive an invitation and card issued to it, as Graphshell co-op does; no bearer file.

- **D9, keys live in the custodian.** Mark chose "The custodian
  (Recommended)". Owner and reader sign through castellan's `CustodyIdentity`,
  as co-op does. The CLI has no file-backed signer.

  Links: summary The CLI's owner and reader keys stay in the castellan custodian; there is no file-backed signer.

- **D10, the tool is a port.** Mark chose "host/ports/mere-cli
  (Recommended)". It sits beside `mere-resident` and `moot`.

  Links: summary The rebuilt tool is host/ports/mere-cli, a port beside mere-resident.

- **D11, browsers read through Turnstone's web app.** Mark chose "Turnstone's
  web app gains a query view (Recommended)". reader.html and reader.js are
  deleted. A browser reader opens Turnstone's Graphshell web page with the
  invite fragment, sends a chirograph `Query` and shows the rows. That view
  joins this landing.

  Links: summary Browser readers use Turnstone's Graphshell web app, which gains a query view; the CLI's JavaScript reader is deleted.

- **D12, murm gains an I2P transport.** Mark chose "murm gains an I2P Transport
  (Recommended)". graphshell-host's accept and dial take a murm `Transport`, and
  murm has none for I2P. murm's transport crate gains one, with the peer
  authenticated by Noise as the CLI's setup gate does today. murm lands first.

  Links: summary murm's transport crate gains an I2P Transport with a Noise-authenticated peer, so graphshell-host serves I2P unchanged.

- **D13, binary tests use a test-only identity.** Mark chose "Test-only identity
  feature (Recommended)". A `test-identity` feature reads an in-memory seed from
  a file the test writes. Like graphshell-host's `test-signer`, it is for tests
  only and never in a release build.

  Links: summary The CLI's binary tests take identities from a test-only feature, never enabled in a release build.

- **D14, invitations last a week by default.** Mark chose "7 days, --expires to
  change (Recommended)". The cap stays 30 days.

  Links: summary A CLI invitation expires after seven days unless --expires says otherwise, up to thirty.

- **D15, the CLI is its own custodian app.** Mark chose "Its own app,
  "mere-cli" (Recommended)". It declares app `mere-cli` with its own key salt
  and the delegation scope `mere.query`, so the custodian shows and limits it
  apart from Graphshell.

  Links: summary The CLI connects to the custodian as app mere-cli, with its own salt and the mere.query delegation scope.

- **D16, every carrier is an optional feature; a default build only queries
  locally.** Asked how the CLI carries Reticulum, whose retinue licence bars use
  in building model-training datasets (mere-resident keeps it off by default),
  Mark wrote "I would figure that they would all be optional, 1". Asked which
  carriers a default build includes, he chose "None: local query only". iroh,
  I2P, WebRTC and Reticulum are each a cargo feature, all off by default; a
  build without a carrier refuses it by name.

  Links: summary Each CLI carrier is a cargo feature, all off by default; a default build only queries local files.

- **D17, `mere revoke` writes a signed revocation file.** Asked how an owner
  revokes, Mark asked: "Hm. Would 2 apply if a distributed serve needed to
  propagate the revoke? What is the willow model…". A state file reaches one
  serve, but the revocation in it is a signed statement any host can verify and
  fold, so it is portable. Willow's Meadowcap has no revocation; its spec
  illustrates week-long capabilities reissued instead. Put back with that, he
  chose "`mere revoke` writes a signed file (Recommended)". `mere revoke` signs a
  revocation into serve's state directory, which serve polls and folds into its
  ledger. The same file revokes at any serve it is given to. Spreading
  revocations between serves automatically is not in this plan.

  Links: summary mere revoke writes a signed revocation into serve's state directory, which serve folds; the file is portable to other serves.

- **D18, the browser fetches the query text from signaling.** Mark chose
  "Signaling serves GET /query (Recommended)". The CLI's signaling adds
  `/query`, the allowed text; the page fetches it beside `/invite`, shows it and
  runs it. The invite format does not change.

  Links: summary The CLI's signaling serves the allowed query text at GET /query for the browser page.

- **D19, browser invites carry C4's placeholder release.** Mark chose "C4's
  placeholder for now (Recommended)". The `ReleaseRefV1` a CLI browser invite
  names is C4's fixed bytes until Turnstone's web app has published releases.
  Naming the real release is owed.

  Links: summary CLI browser invites name C4's placeholder release until Turnstone's web app publishes releases.

- **D20, a browser invite allows eight sessions.** Mark chose "8, with --uses
  (Recommended)". It still expires in seven days (D14).

  Links: summary A CLI browser invite allows eight sessions by default, adjustable with --uses.

- **D21, the query view is a page beside co-op's.** Mark chose "A query page
  beside co_op.html (Recommended)". `query.html` in Turnstone's
  `ports/graphshell/web` joins with the fragment, fetches the query, shows it,
  runs it and renders the rows as a table. graphshell-client's `RemoteSession`
  gains the paged query operation it uses.

  Links: summary Turnstone's web app gains query.html beside co_op.html, and graphshell-client's RemoteSession gains a paged query operation.

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
  query is allowed: the scope path is the query's digest (D7). The invitation is
  issued to the reader's contact (D8).
- Done when: a granted query returns rows, an ungranted or expired one is refused,
  and a revoked invitation is refused, each with its own test.

### Q3. graphshell-host owns the signaling listener

- The `signaling` module is lifted from C4. Turnstone's C4 host consumes it, and
  `c4_host_signaling.rs` still passes.
- Done when: graphshell-host's door loopback test signals through the module, and
  C4's signaling test passes on it.

### Q4. the tool, rebuilt in host/

- `host/ports/mere-cli` (D10). Keys come from the custodian (D9). It issues
  invitations with `Invitation::issue`, accepts and dials through
  graphshell-host's carrier functions, queries with Q1/Q2, and signals the browser
  with Q3. `getrandom`, `subtle`, `base64` and `wtransport` leave its manifest.
- Done when: its two existing tests (`local_query_preserves_unbound_and_escaped_cells`,
  `iroh_invitation_limits_access_across_repeated_queries`) pass in their new form,
  Reticulum and I2P keep whatever coverage they have today, and a browser joins
  through the door into Turnstone's web app and reads rows (D11).

### Q5. land

- `crates/cli` leaves Mere. Mere's README points to the tool's new home.
- Done when: Mere's bans check passes with 0 duplicates, Graphshell's and
  Turnstone's hooks pass, and everything lands in one push per repository, in
  dependency order.

## Progress

- 2026-11-21: plan written. D1 to D5 ruled, then D6 and D7.
- 2026-11-21: Q1 built (branch `cli-q1`). D8 to D11 ruled at Q2's start.
- 2026-11-21: Q2 built: `Invitation::issue_query`, the admitted action in
  `AdmittedEndpointContext`, and `query_host` (granted, ungranted, expired and
  revoked tests). Q3 built: `signaling`, with a loopback join through it, and
  Turnstone's C4 host ported to it. D12 to D15 ruled at Q4's start.
- 2026-11-21: murm's `I2pTransport` built (branch `i2p-transport`), tested over
  the SAM fixture. graphshell-host gains `dial_projection_session_bound` and
  `Invitation::hello_bound` for carriers bound to a link (Reticulum). D16 and
  D17 ruled.
- 2026-11-21: the CLI rebuilt as `host/ports/mere-cli`. Its reader flow (granted
  query answers, another refused, a tampered invitation refused, a revocation
  stops it) passes over iroh. The browser's hello now asks for the invite's
  action. D18 to D21 ruled for the browser path.
