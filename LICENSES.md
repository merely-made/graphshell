# Licenses in this repository

**This repository: MPL-2.0.** Every file Mark wrote carries Exhibit A and the
SPDX tag `MPL-2.0`, per the
[license posture brief](../mere/design_docs/2026-08-22_license_posture_brief.md)
of 2026-08-22 (mere `design_docs/2026-08-22_license_posture_brief.md`). The
full text is in [`LICENSE`](LICENSE).

This file is the provenance ledger. It is the authority for what the relicense
tool (mere `scripts/relicense_headers.py`) skips: the backtick-quoted paths in
the **Retained licenses** table are never touched. Provenance comes before
license: a file gets Exhibit A only if Mark wrote it.

## Retained licenses

Third-party code keeps its own license and its own notices. Nothing here is
relicensed, and nothing here receives a Merely copyright line.

| Path | License | Upstream | Notice files |
|---|---|---|---|
| `host/support/patches/cubek-matmul` | MIT OR Apache-2.0 | [tracel-ai/cubek](https://github.com/tracel-ai/cubek) | `LICENSE-MIT`, `LICENSE-APACHE` in-tree; source and patch provenance in `MERE-PATCH.md` |
| `host/support/patches/light-file-dialog` | Zlib | [wengchengjian/light-file-dialog](https://github.com/wengchengjian/light-file-dialog), 3.21.3 with local dialog fixes | License and attribution in `README.md` and `Cargo.toml`, exact import and patch provenance in `UPSTREAM.json` |
| `host/ports/pelt/examples/resources` | MPL-2.0 | [servo/servo](https://github.com/servo/servo) (`resources/`), by way of genet | Servo's |

`host/ports/pelt/examples/resources` holds two Servo logo images that Pelt's
example documents reference by URLs that climb to the repository root and
descend again. They came with Pelt from genet on 2026-09-03, sat at the
repository root until later the same day, and are Servo's work under the same
MPL-2.0, so they keep Servo's copyright and take no Merely line; genet's own
ledger carries the identical row. The directory's `README.md` is Mark's and
carries the house header.

These are vendored patch trees consumed through
`[patch]`; they are upstream's work carrying upstream's terms.

## Published grants that arrived with a crate

A published version keeps the grant it shipped with, permanently; MPL-2.0
reaches it at its next functional bump, per the sweep plan's invariant 8.

| Crate | Path | Published version's grant | In-tree notice files |
|---|---|---|---|
| `tinct` | `crates/tinct` | 0.1.2, MIT OR Apache-2.0 | — (deleted by Mark's ruling, `642ca2d7`, 2026-09-24) |
