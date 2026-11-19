// Copyright 2026 Mark Alan Boykin
// SPDX-License-Identifier: MPL-2.0

use super::*;

impl<K: Copy + Eq + Hash + Ord> ForeignA11y<K> {
    /// The full retained snapshots that a candidate would graft. This read
    /// leaves the native publication cursor and mounted owners alone.
    pub fn snapshots_for_lint(&self, visible: &[K]) -> Vec<TreeUpdate> {
        let mut snapshots = Vec::new();
        for owner in visible {
            let Some(forest) = self.surfaces.get(owner).filter(|forest| forest.ready()) else {
                continue;
            };
            let mut trees: Vec<_> = forest.declared.iter().copied().collect();
            trees.sort_unstable();
            snapshots.extend(
                trees
                    .into_iter()
                    .map(|tree| forest.trees[&tree].snapshot(tree)),
            );
        }
        snapshots
    }

    pub fn resolve_guest(&self, address: TreeAddress) -> Result<(K, &Node), String> {
        let ActionTarget::Guest(surface) = self.owners.resolve(address) else {
            return Err("action names a host, unknown or retired foreign tree".into());
        };
        if !self.mounted.contains(surface) || !self.ready(*surface) {
            return Err("action names a hidden or incomplete foreign surface".into());
        }
        let (tree, local) = address;
        let node = self.surfaces[surface]
            .trees
            .get(&tree)
            .and_then(|tree| tree.nodes.get(&local))
            .ok_or("action names an absent or retired foreign node")?;
        Ok((*surface, node))
    }

    pub fn action_target(&self, request: &ActionRequest) -> Result<K, String> {
        let (surface, node) = self.resolve_guest((request.target_tree, request.target_node))?;
        if !node.supports_action(request.action) {
            return Err("foreign node does not advertise the requested action".into());
        }
        Ok(surface)
    }
    /// Supplier tree attachment identities used by the native namespace.
    pub fn tree_lineages(&self) -> HashMap<TreeId, u64> {
        self.mounted
            .iter()
            .filter_map(|owner| self.surfaces.get(owner))
            .filter(|forest| forest.ready())
            .flat_map(|forest| {
                forest
                    .lineages
                    .iter()
                    .map(|(&tree, &lineage)| (tree, lineage))
            })
            .collect()
    }

    /// Capture a target's supplier and current reachable life.
    pub fn guest_admission(&self, address: TreeAddress) -> Option<GuestAdmission<K>> {
        let (owner, _) = self.resolve_guest(address).ok()?;
        let (tree, node) = address;
        let forest = self.surfaces.get(&owner)?;
        Some(GuestAdmission {
            owner,
            lineage: *forest.lineages.get(&tree)?,
            life: *forest.trees.get(&tree)?.lives.get(&node)?,
        })
    }

    /// Capture all mounted supplier targets from the latest composition.
    pub fn guest_admissions(&self) -> HashMap<TreeAddress, GuestAdmission<K>> {
        self.published_surfaces()
            .into_iter()
            .flat_map(|(_, surface)| {
                surface.trees().values().flat_map(|tree| {
                    tree.nodes.iter().filter_map(|(node, _)| {
                        let address = (tree.tree_id, *node);
                        self.guest_admission(address)
                            .map(|witness| (address, witness))
                    })
                })
            })
            .collect()
    }

    /// A queued target still belongs to the same reachable supplier node.
    pub fn matches_guest(&self, address: TreeAddress, witness: &GuestAdmission<K>) -> bool {
        self.guest_admission(address).as_ref() == Some(witness)
    }
}
