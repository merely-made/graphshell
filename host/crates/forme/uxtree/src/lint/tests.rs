// Copyright 2026 Mark Alan Boykin
// SPDX-License-Identifier: MPL-2.0

use super::*;
use accesskit::{Rect, TreeInfo};

fn node(role: Role) -> Node {
    let mut node = Node::new(role);
    node.set_bounds(Rect::new(0.0, 0.0, 10.0, 10.0));
    node
}

fn fixture() -> TreeUpdate {
    let mut root = node(Role::Window);
    root.set_children(vec![NodeId(2)]);
    let mut button = node(Role::Button);
    button.set_label("Play");
    button.add_action(Action::Click);
    button.add_action(Action::Focus);
    TreeUpdate {
        tree_id: TreeId::ROOT,
        tree: Some(TreeInfo::new(NodeId(1))),
        nodes: vec![(NodeId(1), root), (NodeId(2), button)],
        focus: NodeId(1),
    }
}

fn inspect_fixture(tree: &TreeUpdate) -> Vec<Finding> {
    inspect(
        TreeId::ROOT,
        std::slice::from_ref(tree),
        |_| true,
        |_, _| true,
    )
    .findings
}

#[test]
fn clean_and_planted_findings_use_the_same_inspector() {
    use FindingKind::*;
    assert!(inspect_fixture(&fixture()).is_empty());
    type Case = (fn(&mut TreeUpdate), FindingKind, u64);
    let cases: &[Case] = &[
        (
            |tree| tree.nodes.push(tree.nodes[1].clone()),
            Structure(StructureIssue::DuplicateNode),
            2,
        ),
        (
            |tree| tree.nodes[1].1.set_children(vec![NodeId(1)]),
            Structure(StructureIssue::Cycle),
            1,
        ),
        (
            |tree| tree.nodes[0].1.set_children(vec![NodeId(2), NodeId(2)]),
            Structure(StructureIssue::RepeatedChild),
            2,
        ),
        (
            |tree| tree.nodes[0].1.set_children(vec![NodeId(2), NodeId(9)]),
            Structure(StructureIssue::DanglingNode),
            9,
        ),
        (
            |tree| tree.nodes.push((NodeId(3), node(Role::Group))),
            Structure(StructureIssue::UnreachableNode),
            3,
        ),
        (|tree| tree.nodes[1].1.clear_label(), MissingLabel, 2),
        (|tree| tree.nodes[1].1.clear_bounds(), MissingBounds, 2),
    ];
    for &(plant, kind, node) in cases {
        let mut tree = fixture();
        plant(&mut tree);
        assert_eq!(
            inspect_fixture(&tree),
            [Finding {
                tree: TreeId::ROOT,
                node: Some(NodeId(node)),
                kind
            }]
        );
    }
    let tree = fixture();
    let report = inspect(
        TreeId::ROOT,
        &[tree],
        |(_, id)| id != NodeId(2),
        |_, action| action == Action::Focus,
    );
    assert_eq!(
        report.findings.iter().map(|f| f.kind).collect::<Vec<_>>(),
        [MissingEnvelope, UnroutedAction(Action::Click),]
    );
}

#[test]
fn stitching_the_same_path_twice_reports_the_duplicate() {
    let one = || crate::stitch("shared-path", node(Role::Group), Vec::new());
    let tree = crate::stitch("outer", node(Role::Window), vec![one(), one()]).to_tree_update(None);
    assert!(inspect_fixture(&tree).iter().any(|f| {
        f.node == Some(crate::node_id_for_path("shared-path"))
            && f.kind == FindingKind::Structure(StructureIssue::DuplicateNode)
    }));
}

#[test]
fn graft_walk_uses_tree_ids_and_finds_malformed_edges() {
    let mut host = fixture();
    let mut guest = fixture();
    let guest_id = crate::tree_id_for_path("lint/guest");
    host.nodes[1].1 = crate::graft_node(Role::Group, guest_id, (0.0, 0.0), (10.0, 10.0));
    guest.tree_id = guest_id;
    assert!(
        inspect(
            TreeId::ROOT,
            &[host.clone(), guest.clone()],
            |_| true,
            |_, _| true
        )
        .findings
        .is_empty()
    );
    let absent = structure(TreeId::ROOT, std::slice::from_ref(&host));
    assert_eq!(
        absent,
        [Finding {
            tree: TreeId::ROOT,
            node: Some(NodeId(2)),
            kind: FindingKind::Structure(StructureIssue::MissingTree(guest_id)),
        }]
    );
    guest.nodes[1].1.set_tree_id(TreeId::ROOT);
    assert!(
        structure(TreeId::ROOT, &[host, guest])
            .iter()
            .any(|f| f.kind == FindingKind::Structure(StructureIssue::Cycle)
                && f.tree == TreeId::ROOT)
    );
}

#[cfg(feature = "registry-lint")]
#[test]
fn real_registry_sink_receives_all_five_channels_without_node_text() {
    use registry::diagnostics::channels::*;
    use registry::diagnostics::{
        DiagnosticEvent, IngressLimits, diagnostic_channel, scoped_thread_sender,
    };
    let (sender, receiver) = diagnostic_channel(IngressLimits::default());
    let _guard = scoped_thread_sender(sender);
    let mut tree = fixture();
    tree.nodes[1].1.set_label("private fixture");
    tree.nodes[1].1.clear_bounds();
    tree.nodes.push(tree.nodes[1].clone());
    let report = inspect(
        TreeId::ROOT,
        &[tree],
        |(_, id)| id != NodeId(2),
        |_, _| true,
    );
    assert!(receiver.try_drain(10).unwrap().packets.is_empty());
    report.send_to(&mut RegistrySink);
    let batch = receiver.try_drain(10).unwrap();
    let channels: Vec<_> = batch
        .packets
        .iter()
        .map(|packet| {
            let DiagnosticEvent::MessageSentStructured {
                channel_id, fields, ..
            } = &packet.event
            else {
                panic!("expected registry channel record")
            };
            assert!(
                fields
                    .iter()
                    .all(|field| !field.value.contains("private fixture"))
            );
            *channel_id
        })
        .collect();
    assert_eq!(
        channels,
        [
            CHANNEL_UX_TREE_BUILD,
            CHANNEL_UX_STRUCTURAL_VIOLATION,
            CHANNEL_UX_CONTRACT_WARNING,
            CHANNEL_UX_PRESENTATION_BOUNDS_MISSING,
            CHANNEL_UX_TREE_SNAPSHOT_BUILT,
        ]
    );
}
