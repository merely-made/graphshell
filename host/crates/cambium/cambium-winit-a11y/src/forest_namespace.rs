// Copyright 2026 Mark Alan Boykin
// SPDX-License-Identifier: MPL-2.0

//! Two-way native address translation for one window's complete forest.

use std::collections::{HashMap, HashSet};

use accesskit::{NodeId, TreeId, TreeUpdate};
use uxtree::envelope::TreeAddress;

use crate::forest_replay::ForestReplay;
use crate::{native_addresses::NativeAddresses, remap::try_map_node_ids};

#[derive(Default)]
pub(super) struct ForestNamespace {
    nodes: NativeAddresses<Option<u64>>,
    replay: ForestReplay,
    // The window root keeps one native ID for the window's life, so the
    // provider a reader receives at activation survives the republication.
    host_root: Option<NodeId>,
}

pub(super) struct PreparedForest {
    pub namespace: ForestNamespace,
    pub updates: Vec<TreeUpdate>,
    pub sources: HashMap<TreeAddress, TreeAddress>,
}

impl ForestNamespace {
    pub fn hosted(host_root: NodeId) -> Self {
        Self {
            host_root: Some(host_root),
            ..Self::default()
        }
    }

    pub fn native_address(&self, source: TreeAddress) -> Option<TreeAddress> {
        self.nodes.get(source).map(|node| (source.0, node))
    }

    pub fn has_native_id(&self, node: NodeId) -> bool {
        self.nodes.has_native_id(node)
    }

    pub fn prepare(
        &self,
        mut updates: Vec<TreeUpdate>,
        tree_lineages: &HashMap<TreeId, u64>,
        changed: &mut HashSet<TreeAddress>,
    ) -> Result<PreparedForest, String> {
        validate_forest(&updates)?;
        let host_root = self
            .host_root
            .zip(updates[0].tree.as_ref())
            .map(|(native, tree)| ((updates[0].tree_id, tree.root), native));
        let mut nodes = NativeAddresses::default();
        let mut sources = HashMap::new();
        for update in &updates {
            for (id, _) in &update.nodes {
                let address = (update.tree_id, *id);
                let tree_lineage = tree_lineages.get(&update.tree_id).copied();
                if let Some((_, native)) = host_root.filter(|(root, _)| *root == address) {
                    if nodes.pin(&self.nodes, address, native, tree_lineage) {
                        changed.insert(address);
                    }
                    sources.insert((update.tree_id, native), address);
                    continue;
                }
                let (native, assigned) = nodes.bind(
                    &self.nodes,
                    address,
                    tree_lineage,
                    changed.contains(&address),
                );
                if assigned {
                    changed.insert(address);
                }
                sources.insert((update.tree_id, native), address);
            }
        }
        for update in &mut updates {
            let tree_id = update.tree_id;
            let map = |local| {
                nodes
                    .get((tree_id, local))
                    .ok_or_else(|| format!("unknown native source address {tree_id:?}/{local:?}"))
            };
            if let Some(tree) = update.tree.as_mut() {
                tree.root = map(tree.root)?;
            }
            update.focus = map(update.focus)?;
            for (local, node) in &mut update.nodes {
                *local = map(*local)?;
                try_map_node_ids(node, &map)?;
            }
        }
        let replay = self.replay.prepare(&updates)?;
        Ok(PreparedForest {
            namespace: Self {
                nodes,
                replay,
                host_root: self.host_root,
            },
            updates,
            sources,
        })
    }
}

/// The first update for each tree is a complete snapshot. The only subsequent
/// update for that tree is the focus move that follows its focused guest.
fn validate_forest(ordered: &[TreeUpdate]) -> Result<(), String> {
    if ordered
        .first()
        .is_none_or(|first| first.tree_id != TreeId::ROOT || first.tree.is_none())
    {
        return Err("a complete root tree must begin native publication".into());
    }
    let mut snapshots = Vec::new();
    let mut declared = HashSet::new();
    for update in ordered {
        if let Some(info) = &update.tree {
            if update.tree_id != TreeId::ROOT && !declared.contains(&update.tree_id) {
                return Err(format!("guest {:?} must follow its graft", update.tree_id));
            }
            for (id, node) in &update.nodes {
                if let Some(guest) = node.tree_id() {
                    if *id == info.root || guest == TreeId::ROOT || !node.children().is_empty() {
                        return Err(format!("invalid graft at {:?}/{id:?}", update.tree_id));
                    }
                    if !declared.insert(guest) {
                        return Err(format!("guest {guest:?} has multiple grafts"));
                    }
                }
            }
            snapshots.push(update.clone());
        } else if !update.nodes.is_empty()
            || !snapshots
                .iter()
                .any(|snapshot| snapshot.tree_id == update.tree_id)
        {
            return Err(format!("incomplete native update for {:?}", update.tree_id));
        }
    }
    if let Some(finding) = uxtree::lint::structure(TreeId::ROOT, &snapshots).first() {
        return Err(format!("invalid native forest: {finding:?}"));
    }
    Ok(())
}
