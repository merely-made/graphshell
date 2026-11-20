/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

//! Read-only questions over an authored placement tree.

use std::collections::BTreeSet;

use super::{ComponentKey, ContainerKey, ContainerKind, PlacementNode, container};

/// The container and children of a placement node, or `None` for a component.
fn as_container(node: &PlacementNode) -> Option<(ContainerKey, &[PlacementNode])> {
    match node {
        PlacementNode::Component { .. } => None,
        PlacementNode::Group { key, children } => {
            Some((container(ContainerKind::Group, &key.0), children))
        },
        PlacementNode::Drawer { key, children } => {
            Some((container(ContainerKind::Drawer, &key.0), children))
        },
    }
}

/// Every group and drawer in the tree, at any depth.
pub fn container_keys(nodes: &[PlacementNode]) -> BTreeSet<ContainerKey> {
    let mut keys = BTreeSet::new();
    for (key, children) in nodes.iter().filter_map(as_container) {
        keys.insert(key);
        keys.extend(container_keys(children));
    }
    keys
}

/// Every component inside `container`, nested containers included. Empty when
/// the tree has no such container.
pub fn keys_in_container(
    nodes: &[PlacementNode],
    container: &ContainerKey,
) -> BTreeSet<ComponentKey> {
    fn all(nodes: &[PlacementNode], out: &mut BTreeSet<ComponentKey>) {
        for node in nodes {
            match as_container(node) {
                None => {
                    if let PlacementNode::Component { key } = node {
                        out.insert(key.clone());
                    }
                },
                Some((_, children)) => all(children, out),
            }
        }
    }
    let mut result = BTreeSet::new();
    for (key, children) in nodes.iter().filter_map(as_container) {
        if &key == container {
            all(children, &mut result);
        } else {
            result.extend(keys_in_container(children, container));
        }
    }
    result
}

/// Every component with a collapsed container among its ancestors.
pub fn hidden_keys(
    nodes: &[PlacementNode],
    collapsed: &BTreeSet<ContainerKey>,
) -> BTreeSet<ComponentKey> {
    fn walk(
        nodes: &[PlacementNode],
        collapsed: &BTreeSet<ContainerKey>,
        inherited: bool,
        out: &mut BTreeSet<ComponentKey>,
    ) {
        for node in nodes {
            match as_container(node) {
                None => {
                    if let (true, PlacementNode::Component { key }) = (inherited, node) {
                        out.insert(key.clone());
                    }
                },
                Some((key, children)) => walk(
                    children,
                    collapsed,
                    inherited || collapsed.contains(&key),
                    out,
                ),
            }
        }
    }
    let mut result = BTreeSet::new();
    walk(nodes, collapsed, false, &mut result);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composition::{DrawerKey, GroupKey};

    fn comp(key: &str) -> PlacementNode {
        PlacementNode::Component { key: key.into() }
    }

    fn group(key: &str, children: Vec<PlacementNode>) -> PlacementNode {
        PlacementNode::Group {
            key: GroupKey::from(key),
            children,
        }
    }

    fn drawer(key: &str, children: Vec<PlacementNode>) -> PlacementNode {
        PlacementNode::Drawer {
            key: DrawerKey::from(key),
            children,
        }
    }

    fn g(key: &str) -> ContainerKey {
        container(ContainerKind::Group, key)
    }

    fn d(key: &str) -> ContainerKey {
        container(ContainerKind::Drawer, key)
    }

    fn keys(names: &[&str]) -> BTreeSet<ComponentKey> {
        names.iter().map(|name| (*name).into()).collect()
    }

    /// root: a, group outer { b, drawer inner { c, group deep { d } }, e }, group side { f }
    fn tree() -> Vec<PlacementNode> {
        vec![
            comp("a"),
            group(
                "outer",
                vec![
                    comp("b"),
                    drawer("inner", vec![comp("c"), group("deep", vec![comp("d")])]),
                    comp("e"),
                ],
            ),
            group("side", vec![comp("f")]),
        ]
    }

    #[test]
    fn container_keys_lists_every_depth_and_kind() {
        assert_eq!(
            container_keys(&tree()),
            BTreeSet::from([g("outer"), d("inner"), g("deep"), g("side")])
        );
        assert!(container_keys(&[comp("a")]).is_empty());
        assert!(container_keys(&[]).is_empty());
    }

    #[test]
    fn container_kind_is_part_of_the_key() {
        let nodes = vec![group("x", vec![]), drawer("x", vec![])];
        assert_eq!(container_keys(&nodes), BTreeSet::from([g("x"), d("x")]));
    }

    #[test]
    fn keys_in_container_includes_nested_components() {
        let nodes = tree();
        assert_eq!(
            keys_in_container(&nodes, &g("outer")),
            keys(&["b", "c", "d", "e"])
        );
        assert_eq!(keys_in_container(&nodes, &d("inner")), keys(&["c", "d"]));
        assert_eq!(keys_in_container(&nodes, &g("deep")), keys(&["d"]));
        assert_eq!(keys_in_container(&nodes, &g("side")), keys(&["f"]));
    }

    #[test]
    fn keys_in_container_is_empty_for_unknown_wrong_kind_root_or_empty() {
        let nodes = tree();
        assert!(keys_in_container(&nodes, &g("missing")).is_empty());
        assert!(keys_in_container(&nodes, &d("outer")).is_empty());
        assert!(keys_in_container(&nodes, &container(ContainerKind::Root, "")).is_empty());
        assert!(keys_in_container(&[group("empty", vec![])], &g("empty")).is_empty());
    }

    #[test]
    fn hidden_keys_follow_any_collapsed_ancestor() {
        let nodes = tree();
        assert_eq!(
            hidden_keys(&nodes, &BTreeSet::from([g("outer")])),
            keys(&["b", "c", "d", "e"])
        );
        assert_eq!(
            hidden_keys(&nodes, &BTreeSet::from([d("inner")])),
            keys(&["c", "d"])
        );
        assert_eq!(
            hidden_keys(&nodes, &BTreeSet::from([g("deep")])),
            keys(&["d"])
        );
        assert_eq!(
            hidden_keys(&nodes, &BTreeSet::from([g("deep"), g("side")])),
            keys(&["d", "f"])
        );
    }

    #[test]
    fn hidden_keys_is_empty_without_collapsed_containers() {
        assert!(hidden_keys(&tree(), &BTreeSet::new()).is_empty());
        assert!(hidden_keys(&tree(), &BTreeSet::from([g("missing")])).is_empty());
        assert!(hidden_keys(&[], &BTreeSet::from([g("outer")])).is_empty());
    }
}
