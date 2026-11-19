# Table projection plan: receipts

Run evidence for `graphshell_docs/implementation_strategy/2026-11-14_table_projection_plan.md`.

## 2026-11-19: T4 speed on the Windows laptop

Branch `t4-table-view`. Release build of `cambium-desktop`'s `table` example, window
960 x 640 logical at scale 2 (1920 x 1280 captured), body 480 px, rows 26 px, six
fields. Scenario `host/crates/cambium/cambium-genet-winit-host/examples/table.scn`;
captures and text receipts under `Code/testing/graphshell/table-t4/`. Other sessions
were free to use the machine; no load control.

```text
TABLE_SCENARIO=crates/cambium/cambium-genet-winit-host/examples/table.scn \
TABLE_CAPTURE_DIR=<dir> TABLE_RECEIPT=<dir>/receipt.txt TABLE_ROWS=<n> TABLE_GATE=1 \
  target/release/examples/table.exe
```

Times are the host's `FrameProfile.total_us` (UI-thread work for one presented frame).
A sort is timed from the top of the frame that applies it to that frame drawn.

| run | scroll p95 | idle p95 (control) | sort p95 | open |
|---|---|---|---|---|
| 10,000 rows | 104.9 ms (418 frames) | 22.7 ms (119 frames) | 133.8 ms (10 sorts) | 149 ms |
| 100,000 rows | 103.4 ms | 21.0 ms | 546.6 ms | 1,473 ms |
| 10,000 rows, `TABLE_STALL_MS=20` | 125.5 ms | 43.9 ms | 192.7 ms | 192 ms |

The stall run is the positive control: 20 ms slept inside each timed frame moves every
figure by about that much, so the instrument sees work in the frame. The idle row is
frames drawn with nothing changed.

One scroll frame at 10,000 rows, after the rows moved into one scrolled strip (one DOM
mutation a frame): total 94.5 ms, style 26.4 ms, layout with text 29.8 ms, accessibility
publish 17.2 ms, scene emit 7.1 ms, raster 3.7 ms. Before the strip (24 mutations) the
total was 97.4 ms, and at an 8-row body 31.1 ms: cost follows the rows drawn, not the
mutations.

Sort compute alone (`table_bench`, removed before commit): `click_header` 7.3 ms at
10,000 rows and 156 ms at 100,000 through the shared reading order; a full compile was
101 ms and 1.02 s. Before the solver's source interning was indexed a compile was
752 ms at 10,000 rows (14, 46 and 179 ms at 1,000, 2,000 and 5,000: quadratic).

Gate (D8, D10): not met. Scroll p95 104.9 ms against 16.7 ms; sort 133.8 ms against
100 ms. iMacs, Fedora and Mint: not measured (not reachable from this lane).

## 2026-11-19: T4 headed in Chrome and Firefox

`cambium-web`'s `table_page` (release, `--features webgl`, wasm-bindgen 0.2.129), served
on loopback by `Code/testing/graphshell/table-t4/browser/serve.py`, opened in headed
Chrome 157 and Firefox 160 with fresh profiles. The page sorts Pages twice, moves the
focus to row 0, column 2 and scrolls 6 px a frame for 181 frames through the table's own
calls, then posts its report (`browser/reports/`). Pictures: `browser/chrome-10k-screen.png`,
`browser/firefox-10k-screen.png` (whole screen).

| browser | frame interval p95 while scrolling | sort to next frame |
|---|---|---|
| Chrome 157 | 106.8 ms | 130.1, 103.3 ms |
| Firefox 160 | 206.0 ms | 107, 124 ms |

Both show the table sorted by Pages descending with "10000 rows, sorted by Pages,
descending". The sort arrow draws as a missing glyph: the page brings only Roboto, which
has no U+2193. Firefox's first-run terms dialog covers the right of the table; it was
not accepted. The page's DOM accessibility mirror held no `columnheader` or `gridcell`
element (0 of each), so the browser check proves the picture, not the mirrored roles.
