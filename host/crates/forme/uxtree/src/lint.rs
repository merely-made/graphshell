// Copyright 2026 Mark Alan Boykin
// SPDX-License-Identifier: MPL-2.0

//! Advisory checks over full AccessKit snapshots and owner-supplied bindings.

use accesskit::{Action, Node, NodeId, Role, TreeId, TreeUpdate};
use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::envelope::TreeAddress;

#[cfg(test)]
mod tests;

/// AccessKit's public action vocabulary. Check each advertised action
/// independently so a focus route cannot hide an unrouted click.
pub const ADVERTISED_ACTIONS: &[Action] = &[
    Action::Click,
    Action::Focus,
    Action::Blur,
    Action::Collapse,
    Action::Expand,
    Action::CustomAction,
    Action::Decrement,
    Action::Increment,
    Action::HideTooltip,
    Action::ShowTooltip,
    Action::ReplaceSelectedText,
    Action::ScrollDown,
    Action::ScrollLeft,
    Action::ScrollRight,
    Action::ScrollUp,
    Action::ScrollIntoView,
    Action::ScrollToPoint,
    Action::SetScrollOffset,
    Action::SetTextSelection,
    Action::SetSequentialFocusNavigationStartingPoint,
    Action::SetValue,
    Action::ShowContextMenu,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StructureIssue {
    MissingTree(TreeId),
    DuplicateTree,
    MissingRoot,
    DuplicateNode,
    Cycle,
    RepeatedChild,
    DanglingNode,
    UnreachableNode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FindingKind {
    Structure(StructureIssue),
    MissingLabel,
    MissingBounds,
    UnroutedAction(Action),
    MissingEnvelope,
}

/// An address and check result. Findings carry no page text or node values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Finding {
    pub tree: TreeId,
    pub node: Option<NodeId>,
    pub kind: FindingKind,
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} in tree {}", self.kind, self.tree.0)?;
        if let Some(node) = self.node {
            write!(f, " at node {}", node.0)?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SnapshotSize {
    pub trees: usize,
    pub nodes: usize,
}

#[derive(Debug)]
pub struct Report {
    pub size: SnapshotSize,
    pub findings: Vec<Finding>,
}

/// Explicit output boundary. Computing a report never emits diagnostics.
pub trait Sink {
    fn begin(&mut self, size: SnapshotSize);
    fn finding(&mut self, finding: &Finding);
    fn end(&mut self, size: SnapshotSize, findings: usize);
}

impl Report {
    pub fn send_to(&self, sink: &mut impl Sink) {
        sink.begin(self.size);
        for finding in &self.findings {
            sink.finding(finding);
        }
        sink.end(self.size, self.findings.len());
    }
}

struct Forest<'a> {
    roots: HashMap<TreeId, Option<NodeId>>,
    nodes: HashMap<TreeAddress, &'a Node>,
    order: Vec<TreeAddress>,
    findings: Vec<Finding>,
}

impl<'a> Forest<'a> {
    fn new(trees: &'a [TreeUpdate]) -> Self {
        use std::collections::hash_map::Entry;

        let mut forest = Self {
            roots: HashMap::new(),
            nodes: HashMap::new(),
            order: Vec::new(),
            findings: Vec::new(),
        };
        for tree in trees {
            let id = tree.tree_id;
            match forest.roots.entry(id) {
                Entry::Vacant(entry) => {
                    entry.insert(tree.tree.as_ref().map(|root| root.root));
                },
                Entry::Occupied(_) => {
                    forest.issue(id, None, StructureIssue::DuplicateTree);
                    continue;
                },
            }
            for (local, node) in &tree.nodes {
                let address = (id, *local);
                match forest.nodes.entry(address) {
                    Entry::Vacant(entry) => {
                        entry.insert(node);
                        forest.order.push(address);
                    },
                    Entry::Occupied(_) => {
                        forest.issue(id, Some(*local), StructureIssue::DuplicateNode);
                    },
                }
            }
        }
        forest
    }

    fn issue(&mut self, tree: TreeId, node: Option<NodeId>, issue: StructureIssue) {
        self.findings.push(Finding {
            tree,
            node,
            kind: FindingKind::Structure(issue),
        });
    }

    fn root(&mut self, tree: TreeId, from: Option<TreeAddress>) -> Option<TreeAddress> {
        match self.roots.get(&tree) {
            Some(Some(local)) => Some((tree, *local)),
            Some(None) => {
                self.issue(tree, None, StructureIssue::MissingRoot);
                None
            },
            None => {
                let (owner, node) = from.map_or((tree, None), |(id, node)| (id, Some(node)));
                self.issue(owner, node, StructureIssue::MissingTree(tree));
                None
            },
        }
    }

    fn check_structure(&mut self, root: TreeId) {
        let mut visited = HashSet::new();
        if let Some(root) = self.root(root, None) {
            self.walk(root, &mut visited);
        }
        let unreachable: Vec<_> = self
            .order
            .iter()
            .copied()
            .filter(|address| !visited.contains(address))
            .collect();
        for &(tree, node) in &unreachable {
            self.issue(tree, Some(node), StructureIssue::UnreachableNode);
        }
        // Also find bad edges in orphaned components.
        for address in unreachable {
            if !visited.contains(&address) {
                self.walk(address, &mut visited);
            }
        }
    }

    fn walk(&mut self, root: TreeAddress, visited: &mut HashSet<TreeAddress>) {
        enum Step {
            Enter(TreeAddress),
            Exit(TreeAddress),
        }
        let mut active = HashSet::new();
        let mut pending = vec![Step::Enter(root)];
        while let Some(step) = pending.pop() {
            let address @ (tree, local) = match step {
                Step::Enter(address) => address,
                Step::Exit(address) => {
                    active.remove(&address);
                    continue;
                },
            };
            if active.contains(&address) {
                self.issue(tree, Some(local), StructureIssue::Cycle);
                continue;
            }
            if !visited.insert(address) {
                self.issue(tree, Some(local), StructureIssue::RepeatedChild);
                continue;
            }
            let Some(node) = self.nodes.get(&address).copied() else {
                self.issue(tree, Some(local), StructureIssue::DanglingNode);
                continue;
            };
            active.insert(address);
            pending.push(Step::Exit(address));
            pending.extend(node.children().iter().map(|id| Step::Enter((tree, *id))));
            if let Some(guest) = node.tree_id()
                && let Some(guest_root) = self.root(guest, Some(address))
            {
                pending.push(Step::Enter(guest_root));
            }
        }
    }
}

/// The structural checks shared with native publication. The caller chooses
/// which findings affect its existing admission policy.
pub fn structure(root: TreeId, trees: &[TreeUpdate]) -> Vec<Finding> {
    let mut forest = Forest::new(trees);
    forest.check_structure(root);
    forest.findings
}

/// Inspect full snapshots, with bindings from the same observation. Route
/// checks use the owner's actual admission per advertised action.
pub fn inspect(
    root: TreeId,
    trees: &[TreeUpdate],
    has_envelope: impl Fn(TreeAddress) -> bool,
    has_route: impl Fn(TreeAddress, Action) -> bool,
) -> Report {
    let mut forest = Forest::new(trees);
    forest.check_structure(root);
    for address @ (tree, local) in forest.order {
        let node = forest.nodes[&address];
        let mut add = |kind| {
            forest.findings.push(Finding {
                tree,
                node: Some(local),
                kind,
            })
        };
        if !has_envelope(address) {
            add(FindingKind::MissingEnvelope);
        }
        if !node.is_hidden() {
            if node.bounds().is_none() {
                add(FindingKind::MissingBounds);
            }
            if requires_label(node) && !has_label(address, node, &forest.nodes) {
                add(FindingKind::MissingLabel);
            }
        }
        for &action in ADVERTISED_ACTIONS {
            if node.supports_action(action) && !has_route(address, action) {
                add(FindingKind::UnroutedAction(action));
            }
        }
    }
    Report {
        size: SnapshotSize {
            trees: trees.len(),
            nodes: trees.iter().map(|tree| tree.nodes.len()).sum(),
        },
        findings: forest.findings,
    }
}

fn requires_label(node: &Node) -> bool {
    node.supports_action(Action::Click)
        || matches!(
            node.role(),
            Role::Button
                | Role::CheckBox
                | Role::Heading
                | Role::Image
                | Role::Link
                | Role::MenuItem
                | Role::PasswordInput
                | Role::RadioButton
                | Role::SearchInput
                | Role::Slider
                | Role::SpinButton
                | Role::Switch
                | Role::Tab
                | Role::TextInput
        )
}

fn has_label((tree, _): TreeAddress, node: &Node, nodes: &HashMap<TreeAddress, &Node>) -> bool {
    let text = |value: Option<&str>| value.is_some_and(|s| !s.trim().is_empty());
    text(node.label())
        || node
            .labelled_by()
            .iter()
            .any(|&id| nodes.contains_key(&(tree, id)))
        || (!matches!(
            node.role(),
            Role::PasswordInput | Role::SearchInput | Role::TextInput
        ) && node.children().iter().any(|&id| {
            nodes.get(&(tree, id)).is_some_and(|child| {
                matches!(child.role(), Role::Label | Role::TextRun)
                    && (text(child.label()) || text(child.value()))
            })
        }))
}
