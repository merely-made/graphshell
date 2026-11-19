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

## 2026-11-19: T5 stage 1, compute time outside the frame

A temporary integration test in scenomise (`tests/t5_timing_tmp.rs`, removed before
staging), run with `cargo test --release -p scenomise --features authoring --test
t5_timing_tmp -- --ignored --nocapture` and again without `--release`
(`CARGO_PROFILE_DEV_DEBUG=0`). Dataset: books with id, title, pages (missing on every
seventh), read, added (a date) and shelf (twelve values, missing on every eleventh), six
fields listed. Median of 9 runs at 10,000 rows and 3 at 100,000, on the Windows laptop
with other lanes building. The filter and grouping are stand-ins, not a chosen form: the
filter keeps pages >= 450, a title containing "1" and read (511 of 10,000 kept); the
grouping is by shelf, no value last.

| step | 10,000 release | 10,000 dev | 100,000 release | 100,000 dev |
| --- | --- | --- | --- | --- |
| compile | 107.2 ms | 202.6 ms | 1,285.8 ms | 2,110.3 ms |
| re-sort, one number key | 6.8 ms | 21.5 ms | 141.5 ms | 334.0 ms |
| re-sort, one date key | 22.1 ms | | 524.2 ms | |
| re-sort, three keys (read, added, title) | 36.8 ms | 125.2 ms | 626.4 ms | 2,217.2 ms |
| filter pass over the sorted rows | 4.2 ms | 5.3 ms | 62.7 ms | 105.1 ms |
| filtered dataset compiled again | 60.3 ms | 44.8 ms | 427.8 ms | 858.8 ms |
| group: stable partition with counts | 37.9 ms | 45.1 ms | 506.2 ms | 985.1 ms |
| group as a leading sort key | 29.7 ms | 102.7 ms | 614.6 ms | 2,509.1 ms |

Not measured here: frame time, which T4's runs above put at about 95 ms a frame (D21).
## 2026-11-19: host frame fixes (D33), before and after

The desktop example `table` (release, debug info off) at 10,000 rows and `table.scn`,
run twice from each binary in alternation in one session: `before` is main at `9b24775c`,
`after` adds the accessibility republish rule and keyed grid rows. Both carried the same
throwaway per-frame profile dump (not committed). No rustc processes were running at the
start or end of any run. Raw output, scripts (`run.sh`, `classes.py`) and pictures:
`Code/testing/graphshell/frame-cost-hostfix/`. A strip move is a scroll frame with one
DOM mutation; a window shift has more. Times are the host's frame time (UI work to
present, accessibility included); "sort, drawn" is the example's sort-to-drawn figure.

| frame | before p50 / p95 | after p50 / p95 |
|---|---|---|
| unchanged (119 frames) | 9.5 / 10.7, 12.7 / 16.5 ms | 3.4 / 5.2, 3.4 / 3.7 ms |
| strip move (276) | 44.9 / 54.2, 42.8 / 50.7 ms | 44.4 / 53.5, 47.6 / 66.5 ms |
| window shift (142) | 48.6 / 55.9, 53.3 / 62.1 ms | 45.2 / 50.6, 43.2 / 51.3 ms |
| sort frame (10) | 56.6 / 71.2, 44.2 / 47.4 ms | 45.0 / 47.6, 49.4 / 61.7 ms |
| sort, drawn, p95 | 105.6, 78.8 ms | 76.8, 92.8 ms |

Accessibility on an unchanged frame: 6.7 and 8.9 ms (p50) before, 0.3 ms after (the
action drain alone). Publications per run: 622 before, 439 after. Every strip move, window
shift and sort still republishes, as each moves the boxes or the rows. Mutations per window
shift: 155 to 161 before whatever the rows moved; after, 83 for a one-row shift and 165 for
two (the 40 px a frame stretch). The 83 are the strip's offset, the leaving row's removal
and the arriving row's own construction; no surviving row is written (grid test
`a_window_shift_keeps_the_surviving_rows`).

Control: `TABLE_STALL_MS=5` in the same session moved unchanged frames from 3.4 to 8.7 ms
(after) and from 9.5 and 12.7 to 17.5 ms (before), by the 5 ms slept. Scroll and sort
spread between runs of one binary (up to 13 ms at p95) is larger than 5 ms, so those rows
show no change outside noise; the unchanged frame is the one this rule moves.

Pictures (`after-1/*.png`): opened, sorted, focused, wheeled, scrolled and resorted, six
distinct frames, none blank. Opened and resorted match the before digests exactly; sorted
and focused differ from them by one or two pixels at most, by one level in one channel, at
the body's clipped bottom edge and the footer, and differ by as much between the two after
runs. Timing-dependent frames (wheeled, scrolled) differ between the two before runs too.
The accessibility check is a test, `the_grid_tree_is_current_after_a_scroll_and_a_sort`:
it reads the published rows and cells after a strip move, a window shift and a sort.
Genet lowers `row` and `gridcell` but not `grid` or `columnheader` (finding above), so the
test reads row and cell roles and the header names.

Gate (D8): still not met. Scroll p95 50.6 to 66.5 ms against 16.7 ms.

## 2026-11-19: T5 stage 2, compute time through the table's own calls

A temporary integration test in cambium (`tests/t5_timing_tmp.rs`, removed before
staging) over the desktop and browser examples' fixture
(`cambium/examples/support/table_fixture.rs`: title, pages missing on every
eleventh, read, added date, author reference of 40, tags list), run with `cargo test
--release -p cambium --test t5_timing_tmp -- --ignored --nocapture` and again
without `--release` (`CARGO_PROFILE_DEV_DEBUG=0`). Each figure is one
`ProjectionTable` call timed alone, the path a person's change takes: the editor's
undo step, the draft to a definition, `ProjectionCompiler::table_reading` (filter,
then one sort with the group keys leading, then the bands) and the band list. No
compile runs on any of these paths. Median of 9 runs at 10,000 rows and 3 at
100,000, each from an unsorted, unfiltered, ungrouped reading, on the Windows laptop
with other lanes building. The filter is pages >= 450, title contains "1", read is
true (469 of 10,000 kept; 4,698 of 100,000).

| step | 10,000 release | 10,000 dev | 100,000 release | 100,000 dev |
| --- | --- | --- | --- | --- |
| open (a compile) | 137.5 ms | 325.7 ms | 1,368.7 ms | 3,554.5 ms |
| filter, three conditions | 4.7 ms | 8.4 ms | 50.1 ms | 83.2 ms |
| group by author (40 groups) | 26.8 ms | 47.1 ms | 395.7 ms | 827.7 ms |
| group by read, then author (82 groups) | 31.0 ms | 65.3 ms | 445.1 ms | 1,029.4 ms |
| sort, one number key | 7.6 ms | 31.6 ms | 129.3 ms | 365.9 ms |
| sort, one date key | 4.9 ms | 25.7 ms | 67.4 ms | 304.1 ms |
| sort, three keys (read, added, title) | 14.8 ms | 56.5 ms | 257.0 ms | 832.2 ms |
| Shift+header adds a third key | 12.0 ms | 54.5 ms | 253.6 ms | 745.2 ms |
| collapse one group | < 0.1 ms | 0.1 ms | < 0.1 ms | 1.3 ms |
| filter, then group two levels (two steps) | 9.7 ms | 20.8 ms | 89.2 ms | 221.4 ms |

A date key now costs no more than a number key, with each row's date parsed once
before sorting; stage 1's 22.1 ms date figure was over a different dataset, so the
two are not a before and after. Frame cost is not in these figures (D21, D22).

## 2026-11-19: table rows kept by occurrence id (D41)

`cambium` lib tests in `projection_table/key_tests.rs`, dev build, over the shared
`ScriptedDom` with its mutation log drained before each step.

- One-row scroll of the 10,000-row table at 2,400 px, body 240 px: 83 mutations, the
  same as with index keys (the arriving row's construction); every row in both
  windows keeps its node.
- A re-sort of 20 rows all drawn (Pages descending): every row keeps its node and no
  text inside a row is written; the rows are moved and their place attributes
  (`style`, `aria-rowindex`, `data-row`, zebra class) change. Positive control: with
  the rows keyed by band index the same test fails ("Title 17… kept its node").
- A collapse with headings keyed by group path: the moved heading and every row still
  drawn keep their nodes (positive control with positional keys fails, as recorded
  for the stage 2 merge).
