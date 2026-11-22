# mere: query linked data, here or on an invited peer

`mere` runs a SPARQL query over an RDF file on this machine. It can also serve
one query over a graph to one invited reader, who asks it from their own
machine and gets the rows back. The graph stays with its owner.

It is built on Graphshell's host toolkit (`graphshell-host`): readers are
invited through its insigne invitations, sessions are admitted and served by
its resident projection host, and queries travel as chirograph `Query`
requests in 512 KiB pages. The design and its rulings are in
`design_docs/graphshell_docs/implementation_strategy/2026-11-21_mere_cli_plan.md`.

## Build

A default build only queries local files. Each carrier is a cargo feature:

| Feature | Carrier |
|---|---|
| `iroh` | iroh, through p2panda |
| `i2p` | I2P, through a local SAM router; each stream carries a Noise session |
| `reticulum` | Reticulum over TCP. Links retinue, whose Reticulum License bars use in building model-training datasets. |
| `webrtc` | A browser reader, through graphshell-host's WebRTC door |

```sh
cargo install --path ports/mere-cli --features iroh
```

A build refuses a carrier it was not built with, by name.

## Query a file

```sh
mere query --graph paper-authors.trig --query paper-authors.rq
```

Rows print as JSON arrays in column order, `null` where a variable is unbound.

## Invite a reader

Keys stay in the custodian: `mere` connects to it as the app `mere-cli`, which
the owner grants once. An invitation is issued to one person's contact, for
one query, and lasts seven days unless `--expires` says otherwise (at most
thirty).

1. The reader writes a contact and sends it to the owner:

   ```sh
   mere contact --out reader.contact
   ```

2. The owner serves the graph and the one query it allows, invited by that
   contact. `serve` writes a new invitation file and keeps serving until
   stopped:

   ```sh
   mere serve --via iroh --graph paper-authors.trig --allow-query paper-authors.rq \
     --contact reader.contact --invite paper.invite
   ```

   Send `paper.invite` to the reader privately.

3. The reader asks. The query file must match the allowed query byte for
   byte; anything else is refused:

   ```sh
   mere query --invite paper.invite --query paper-authors.rq
   ```

Reticulum needs `--listen <reachable-IP:port>` on the owner. I2P uses the SAM
bridge at `127.0.0.1:7656` unless `--sam` names another loopback address.

## Invite a browser

A browser reader needs no contact: the invitation is a link that admits up to
eight sessions (`--uses`) until it expires.

```sh
mere serve --via webrtc --listen 192.168.1.10:43864 --graph paper-authors.trig \
  --allow-query paper-authors.rq --invite paper-browser.invite
```

`--listen` is the address the browser reaches, for signaling (TCP) and the
WebRTC channel (UDP). The invitation file holds the signaling address, the
invite fragment, and `page`: append it to the address of Turnstone's
Graphshell web pages to open `query.html` filled in. The page fetches the
allowed query from the owner, shows it, and runs it. Send the file privately:
the fragment is the credential.

## Revoke

```sh
mere revoke --invite paper.invite
```

This signs a revocation into `paper.invite.revocations/`, which the serve
watches; the reader's open session stops at its next request and new sessions
are refused. The revocation file is a signed statement, so handing it to
another serve of the same invitation revokes there too. A browser link is not
revoked this way: it ends when its uses run out, it expires, or the serve
stops.
