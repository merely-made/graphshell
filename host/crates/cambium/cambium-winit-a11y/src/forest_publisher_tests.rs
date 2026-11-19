// Copyright 2026 Mark Alan Boykin
// SPDX-License-Identifier: MPL-2.0

use std::collections::{HashMap, HashSet};

use accesskit::{Action, ActionData, TextPosition, TextSelection, Uuid};
use accesskit_consumer::{NodeRef, Tree, TreeChangeHandler};

use super::*;
use crate::forest_namespace::PreparedForest;

struct Ignore;

impl TreeChangeHandler for Ignore {
    fn node_added(&mut self, _: &NodeRef<'_>) {}
    fn node_updated(&mut self, _: &NodeRef<'_>, _: &NodeRef<'_>) {}
    fn focus_moved(&mut self, _: Option<&NodeRef<'_>>, _: Option<&NodeRef<'_>>) {}
    fn node_removed(&mut self, _: &NodeRef<'_>) {}
}

const GUEST: TreeId = TreeId(Uuid::from_u128(1));
const CHILD: TreeId = TreeId(Uuid::from_u128(2));
const LEAF: NodeId = NodeId(2);

fn chrome() -> NodeId {
    uxtree::node_id_for_path("turnstone/chrome/omnibar")
}

fn parent(tree_id: TreeId, guest: TreeId, graft_id: NodeId) -> TreeUpdate {
    let root = NodeId(1);
    let mut node = Node::new(Role::Group);
    node.set_children(vec![graft_id]);
    let graft = uxtree::graft::graft_node(Role::Group, guest, (10.0, 20.0), (80.0, 30.0));
    TreeUpdate {
        nodes: vec![(root, node), (graft_id, graft)],
        tree: Some(TreeInfo::new(root)),
        tree_id,
        focus: root,
    }
}

fn focus_only(tree_id: TreeId, focus: NodeId) -> TreeUpdate {
    TreeUpdate {
        nodes: vec![],
        tree: None,
        tree_id,
        focus,
    }
}

fn nested_forest() -> Vec<TreeUpdate> {
    // Two suppliers reuse the local IDs 1 and 2.
    let root = NodeId(1);
    let mut node = Node::new(Role::Group);
    node.set_children(vec![LEAF]);
    let mut leaf = Node::new(Role::TextRun);
    leaf.set_value("Synthetic text range");
    let guest = TreeUpdate {
        nodes: vec![(root, node), (LEAF, leaf)],
        tree: Some(TreeInfo::new(root)),
        tree_id: CHILD,
        focus: LEAF,
    };
    vec![
        parent(TreeId::ROOT, GUEST, chrome()),
        parent(GUEST, CHILD, LEAF),
        guest,
        focus_only(GUEST, LEAF),
        focus_only(TreeId::ROOT, chrome()),
    ]
}

fn native(candidate: &PreparedForest, source: TreeAddress) -> TreeAddress {
    candidate
        .sources
        .iter()
        .find_map(|(native, &local)| (local == source).then_some(*native))
        .expect("every source node has a native address")
}

#[test]
fn nested_forest_is_minted_and_replays_focus_through_the_native_pair() {
    let prepared = ForestNamespace::default()
        .prepare(nested_forest(), &HashMap::new(), &mut HashSet::new())
        .unwrap();
    let target = native(&prepared, (CHILD, LEAF));
    assert_eq!(target.0, CHILD);
    assert_ne!(target.1, native(&prepared, (GUEST, LEAF)).1);
    assert_ne!(native(&prepared, (TreeId::ROOT, chrome())).1, chrome());
    let initial = safe_initial(&prepared.updates[0]).unwrap();
    assert_eq!(initial.nodes.len(), 1);
    assert_eq!(initial.tree.as_ref().unwrap().root, initial.focus);
    assert!(initial.nodes[0].1.children().is_empty());
    let mut consumer = Tree::new(initial, true);
    for update in prepared.updates {
        consumer.update_and_process_changes(update, &mut Ignore);
    }
    let focus = consumer.state().focus().unwrap();
    assert_eq!(focus.locate(), (target.1, target.0));
    assert_eq!(focus.value().as_deref(), Some("Synthetic text range"));
}

#[test]
fn undeclared_or_misordered_guest_refuses_the_batch() {
    let current = ForestNamespace::default();
    let mut ordered = nested_forest();
    // Missing, unfocused child still refuses; no transient focus failure
    // can mask the structural admission check when this rule is disabled.
    ordered.truncate(2);
    assert!(
        current
            .prepare(ordered, &HashMap::new(), &mut HashSet::new())
            .is_err()
    );
    let mut ordered = nested_forest();
    ordered.swap(1, 2);
    assert!(
        current
            .prepare(ordered, &HashMap::new(), &mut HashSet::new())
            .is_err()
    );
    let mut ordered = nested_forest();
    ordered[0].focus = chrome();
    assert!(
        current
            .prepare(ordered, &HashMap::new(), &mut HashSet::new())
            .is_err()
    );
}

#[test]
fn native_request_lowers_selection_and_cannot_acquire_a_replacement_admission() {
    let queue = ActionQueue::default();
    let source = (CHILD, LEAF);
    let mut changed = HashSet::new();
    let lineage = HashMap::from([(CHILD, 1)]);
    let prepared = ForestNamespace::default()
        .prepare(nested_forest(), &lineage, &mut changed)
        .unwrap();
    let target = native(&prepared, source);
    let foreign = native(&prepared, (GUEST, LEAF));
    queue.publish_native(prepared.sources, HashMap::from([(source, "old")]), &changed);
    let request = |endpoint| ActionRequest {
        action: Action::SetTextSelection,
        target_tree: CHILD,
        target_node: target.1,
        data: Some(ActionData::SetTextSelection(TextSelection {
            anchor: TextPosition {
                node: endpoint,
                character_index: 0,
            },
            focus: TextPosition {
                node: endpoint,
                character_index: 2,
            },
        })),
    };
    queue.enqueue_native(request(foreign.1));
    assert!(
        queue.take().is_empty(),
        "a selection cannot cross supplier trees"
    );
    queue.enqueue_native(request(target.1));
    let captured = queue.take().pop().unwrap();
    assert_eq!(captured.request.target_tree, CHILD);
    assert_eq!(captured.request.target_node, LEAF);
    assert_eq!(captured.request.data, request(LEAF).data);
    assert_eq!(captured.admission.as_ref().unwrap().admission, "old");

    let current_binding = HashMap::from([(source, "new")]);
    let mut changed = queue.changed_admissions(&current_binding, PartialEq::eq);
    let next = prepared
        .namespace
        .prepare(nested_forest(), &lineage, &mut changed)
        .unwrap();
    let replacement = native(&next, source);
    queue.publish_native(next.sources, current_binding, &changed);
    queue.enqueue_native(request(target.1));
    assert!(
        queue.take().is_empty(),
        "a late callback keeps its retired native target"
    );
    assert_ne!(replacement, target);
    assert!(!queue.is_current(&captured.request, captured.admission.as_ref().unwrap()));
    assert!(!queue.native_target_is_current(&captured));
}

#[test]
fn a_new_supplier_lineage_gets_new_ids_after_pair_reuse() {
    let old = ForestNamespace::default()
        .prepare(
            nested_forest(),
            &HashMap::from([(CHILD, 1)]),
            &mut HashSet::new(),
        )
        .unwrap();
    let same = old
        .namespace
        .prepare(
            nested_forest(),
            &HashMap::from([(CHILD, 1)]),
            &mut HashSet::new(),
        )
        .unwrap();
    let replacement = old
        .namespace
        .prepare(
            nested_forest(),
            &HashMap::from([(CHILD, 2)]),
            &mut HashSet::new(),
        )
        .unwrap();
    let address = (CHILD, LEAF);
    assert_eq!(native(&old, address), native(&same, address));
    assert_ne!(native(&old, address), native(&replacement, address));
}
