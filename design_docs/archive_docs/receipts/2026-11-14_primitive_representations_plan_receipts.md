# Receipts: primitive representations plan

## 2026-11-14: Crystal probe 1 (ceiling, no geometry)

A throwaway script, not in the repository. Model: a pinned tile holds at most
`floor(520 / smallest side)` linked tiles on its border (a 160 by 100 tile); a link is a
border only if one end is pinned (D8); a nucleus shows at most that many of its links (D9);
a link between two nuclei uses one place on each; conflicts between neighbouring nuclei are
ignored, so every figure is a ceiling. Pins are taken breadth-first from the best-connected
node. The graph is every `.md` file under a `design_docs/` as a node and every mention of
another file's name as a link, with `DOC_README.md` and `README.md` left out.

Controls, same run: a star of 30 leaves at 10 places gives 10 borders; a path of 20 with
every node pinned gives 19 of 19; with one end pinned, 1.

Share of all links that can be borders after k pins:

| Graph | Nodes | Links | Mean, median, max links | Smallest side | Places | Nodes over | 1 | 5 | 10 | 25 | 50 | All |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Graphshell docs | 70 | 118 | 3.4, 3, 10 | 32 | 16 | 0.0% | 8% | 23% | 31% | 58% | 89% | 100% |
| | | | | 48 | 10 | 0.0% | 8% | 23% | 31% | 58% | 89% | 100% |
| | | | | 64 | 8 | 2.9% | 7% | 21% | 29% | 54% | 86% | 97% |
| | | | | 96 | 5 | 17.1% | 4% | 15% | 25% | 45% | 73% | 85% |
| Mere docs | 437 | 1468 | 6.7, 5, 116 | 32 | 16 | 5.5% | 1% | 5% | 10% | 22% | 32% | 86% |
| | | | | 48 | 10 | 16.0% | 1% | 3% | 6% | 14% | 24% | 73% |
| | | | | 64 | 8 | 25.6% | 1% | 2% | 4% | 11% | 20% | 65% |
| | | | | 96 | 5 | 45.3% | 0% | 1% | 3% | 6% | 12% | 47% |
| Turnstone docs | 74 | 134 | 3.6, 3, 15 | 32 | 16 | 0.0% | 11% | 37% | 52% | 68% | 94% | 100% |
| | | | | 48 | 10 | 2.7% | 7% | 31% | 46% | 62% | 88% | 94% |
| | | | | 64 | 8 | 6.8% | 6% | 26% | 41% | 57% | 82% | 88% |
| | | | | 96 | 5 | 18.9% | 4% | 15% | 27% | 46% | 66% | 72% |

Planar limit (at most 3n - 6 links can be borders): 100% for the two small graphs, 89% for
Mere's.

With the index file kept, Graphshell's graph is 82 nodes and 198 links with one node of 80
links, and the all-pinned figure at 10 places falls to 65%.

Read at Graphshell `a0469fc1`; Mere and Turnstone at their local checkouts' working trees.

## 2026-11-14: Crystal probe 2 (greedy packer, real rectangles)

A throwaway script, not in the repository. Same graphs, index files left out, and the same
breadth-first pin order as probe 1. Rules: a pinned tile does not move; pinning packs its
linked tiles against its border at one common size, the largest of 160 by 100, 128 by 80,
96 by 60, 80 by 48 and 48 by 48 that lets the most of them touch it; a border counts when it
is at least half the smallest side long (24 px at 48), 16 px between other linked tiles;
tiles that are not linked stay 8 px apart; an unpinned tile already packed elsewhere moves
only to a place touching more of its linked tiles; a linked tile with no place stays loose.
Simplifications: loose tiles are not obstacles; a nucleus never packed is dropped clear of
the structure; candidate places are tried every 16 px; the search is greedy.

Controls, same run: a star of 30 leaves gives 9 borders against a ceiling of 10, none
false; a path of 20 gives 19 of 19; a triangle 3 of 3. Every run asserted no overlaps.

Share of all links shown as borders, with probe 1's ceiling in brackets:

| Graph | Smallest side | Pinned tile | 1 pin | 5 | 10 | 25 | 50 | All | Unlinked tiles touching |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Graphshell docs (70, 118) | 48 | keeps size | 10% (8) | 17% (23) | 19% (31) | 38% (58) | 53% (89) | 64% (100) | 0 |
| | 48 | grows back | 10% (8) | 18% (23) | 19% (31) | 40% (58) | 55% (89) | 65% (100) | 0 |
| | 64 | keeps size | 10% (7) | 16% (21) | 18% (29) | 41% (54) | 57% (86) | 67% (97) | 0 |
| | 64 | grows back | 10% (7) | 16% (21) | 18% (29) | 40% (54) | 56% (86) | 66% (97) | 0 |
| Turnstone docs (74, 134) | 48 | keeps size | 10% (7) | 15% (31) | 19% (46) | 31% (62) | 42% (88) | 47% (94) | 0 |
| | 48 | grows back | 10% (7) | 18% (31) | 23% (46) | 37% (62) | 46% (88) | 51% (94) | 0 |
| | 64 | keeps size | 8% (6) | 14% (26) | 20% (41) | 32% (57) | 43% (82) | 49% (88) | 0 |
| | 64 | grows back | 8% (6) | 16% (26) | 23% (41) | 34% (57) | 46% (82) | 51% (88) | 0 |
| Mere docs (437, 1468) | 48 | keeps size | 1% (1) | 2% (3) | 2% (6) | 8% (14) | 14% (24) | 31% (73) | 0 |
| | 48 | grows back | 1% (1) | 2% (3) | 2% (6) | 8% (14) | 14% (24) | 31% (73) | 0 |
| | 64 | keeps size | 1% (1) | 2% (2) | 3% (4) | 9% (11) | 14% (20) | 32% (65) | 0 |
| | 64 | grows back | 1% (1) | 2% (2) | 3% (4) | 9% (11) | 15% (20) | 33% (65) | 0 |

A single pin can beat probe 1's ceiling by a point or two because the ceiling counted places
as perimeter over smallest side and the packer lets a tile overhang a corner.

Each run took about a second. Read at Graphshell `1b1a181c`; Mere and Turnstone at their
local checkouts' working trees.

## 2026-11-17 G8 polygon picking and link attachment

The full Cartography library suite passed on Windows and on Fedora T-PC with
`canvas,physics-engine` and `--test-threads=1`. Both runs passed 758 tests with 18 ignored.
The tested source landed as Graphshell `8624f9f5`.

Turnstone's Windows test `native_canvas_picks_and_attaches_to_polygon_silhouettes`
passed against that resolved Git revision. The fixture supplies two registry bodies to
the live canvas through `set_node_shapes`. The native shell pointer route missed the
clipped corner and selected the center on both the event diamond and the software
hexagon. Each link tip was within the test's 0.05 px tolerance of its painted outline.
The 1024 by 600 capture shows both bodies and the connecting link. The diamond clip
cuts part of the event caption.

The capture and log are in the workspace directory
`testing/turnstone/receipts/2026-11-17_m1_silhouettes/windows-git/`.
The SHA-256 of `diamond-and-hexagon.png` is
`2449f1982d7ea4e0e575def643d1e8417006b4cf54360d85534427e9f27d07bf`.
