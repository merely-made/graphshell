# uxtree

Portable accessibility / automation tree for the
[mere](https://crates.io/crates/mere) browser. Projects portable structural
elements into [AccessKit](https://accesskit.dev) nodes with stable,
deterministic ids.

The same projection feeds screen readers through AccessKit's platform adapters
(`accesskit_windows`, `accesskit_unix`, `accesskit_macos`), automation tooling
that pins to stable identity, and inspector overlays.

## API

| Item | Role |
| --- | --- |
| `UxTree` | `root: NodeId`, `nodes: Vec<(NodeId, Node)>`. The root is pushed last, after all descendants. |
| `envelope::Envelope<I, P, D>` | Six metadata fields beside a node, preserving caller-owned instance, provenance and domain types. |
| `envelope::Address<I>` | A producer instance and its local AccessKit ID. |
| `envelope::Stability` | Explicit Life, Scoped revision, Derived generation or Authored key semantics. |
| `UxTree::to_tree_update(focus: Option<NodeId>) -> accesskit::TreeUpdate` | Wraps the nodes for a platform adapter. `None` focus defaults to the root. |
| `node_id_for_path(&str) -> NodeId` | Hashes a domain path to a stable id. |
| `stitch(root_path: &str, root: Node, subtrees: Vec<UxTree>) -> UxTree` | Merges domain subtrees (workbench, gloss, apparatus, ...) under one application root, overwriting the root's children. |
| `project_document(&inker::EngineDocument) -> UxTree` | Projects a document into a `Role::Document` root. |
| `graft::tree_id_for_path(&str) -> TreeId` | Hashes a domain path to a stable subtree id, never the root's. |
| `graft::graft_node(Role, TreeId, origin, size) -> Node` | A graft node for one guest tree: the origin becomes its transform, its bounds are pane-local. |
| `graft::Composition::new(host, guests) -> Result<Composition, GraftError>` | Checks one frame's host tree and guest trees against the rules the AccessKit consumer enforces by panicking, and addresses each guest to its tree. |
| `graft::Grafts` | Per adapter: `activate` answers the activation request; `frame` orders a composition's updates so grafts precede subtrees and focus into a new guest is one move. |
| `graft::GraftTable<K>` | Routes a screen reader's `ActionRequest` to the host or to the guest that owns `target_tree`. |
| `VERSION`, `STAGE` | Crate version string and lifecycle marker (`"pre-alpha"`). |

## Joining sessions as subtrees

`stitch` concatenates nodes in one namespace and hashes only its new root;
callers keep child IDs unique. `graft` joins independent AccessKit subtrees:
each session keeps its own ids, the host
tree carries one graft node per session, focus follows the graft chain, bounds
compose through the graft's transform, and actions arrive addressed by tree.
This is the stack's join for composed sessions (app composition brief, AC2).
`tests/e1a_subtrees.rs` is experiment E1a's tree half, run against the
shared consumer used by the platform adapters.

## Node metadata

`envelope` carries address, stability, provenance, advertised command IDs,
an optional domain reference and an optional authored key. AccessKit keeps
the node semantics. Producers choose the class through `life`, `scoped`,
`derived` or `authored`; construction allocates no lives or generations and
authorizes no actions. Owners still check captured targets before dispatch.
Derived observations without a generation are not action targets. Authored
keys resolve to a current Life or Scoped target at use.

Instance, provenance and domain types remain caller-owned so existing weak
publication witnesses and domain references can be retained without host
dependencies. Producers within one reader agree on those common types. The
module docs contain the per-field source table. Publishing-adapter migration,
generation maps and the full lint remain later phases of the spine plan.

## Stable ids

`node_id_for_path` hashes the path with `std::collections::hash_map::DefaultHasher`
and wraps the `u64` as `accesskit::NodeId`. The same path always yields the same
id, so automation and snapshot tests can pin a node without relying on bounds or
render order.

Paths built by `project_document`:

| Node | Path |
| --- | --- |
| Document root | `engine:{address}` |
| Block `i` | `engine:{address}#blocks/{i}` |
| Nested quote block | `{parent}/quote/{i}` |
| List item | `{parent}/list/{i}` |
| Block inside a list item | `{parent}/list/{i}/item/{j}` |
| Link in a paragraph | `{parent}/paragraph-link/{n}` |
| Link in a heading | `{parent}/heading-link/{n}` |

## Block role mapping

| `inker::Block` | AccessKit role |
| --- | --- |
| `Heading` | `Heading` (with `set_level`) |
| `Paragraph` | `Paragraph` |
| `List` | `List`, children `ListItem`; ordered lists get `description = "ordered"` |
| `Quote` | `Blockquote`, children projected recursively |
| `CodeBlock` | `Code` (language recorded in `description`) |
| `Preformatted` | `Code` |
| `Table` | `Table`, cells flattened into the label |
| `Image` | `Image` (alt as label, url as value) |
| `Rule` | `Splitter` |
| `FeedHeader` | `Section` |
| `FeedEntry` | `Article` |
| `MetadataRow` | `Group` (label / value) |
| `Badge` | `Note` |

Inline spans flatten into the enclosing block's accessible name. `InlineSpan::Link`
is the exception: each link becomes an addressable `Role::Link` child carrying the
url as its value.

## Scope

Bounds come from rendering; the host calls `Node::set_bounds` after layout. Input
and event synthesis stay with the host. Dependencies are `accesskit`, `inker`
(git, genet `main`), and `tracing`.

## Status

Pre-1.0. Document blocks and inline links are covered. Mere structural elements
project through the domain crates that sit on top of this one (`workbench`,
`mere-gloss`), each emitting its own subtree for `stitch`.
