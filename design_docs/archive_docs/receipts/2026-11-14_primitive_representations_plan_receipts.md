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
