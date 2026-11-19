// Copyright 2026 Mark Alan Boykin
// SPDX-License-Identifier: MPL-2.0

//! Check transient focus and graft order before the platform consumes a batch.

use std::collections::HashMap;

use accesskit::{NodeId, TreeId, TreeUpdate};

#[derive(Clone)]
struct MountedTree {
    grafts: HashMap<NodeId, TreeId>,
    focus: NodeId,
}

#[derive(Clone, Default)]
pub(super) struct ForestReplay {
    trees: HashMap<TreeId, MountedTree>,
    parents: HashMap<TreeId, TreeId>,
}

impl ForestReplay {
    pub fn prepare(&self, updates: &[TreeUpdate]) -> Result<Self, String> {
        let mut next = self.clone();
        for update in updates {
            next.apply(update)?;
        }
        Ok(next)
    }

    fn apply(&mut self, update: &TreeUpdate) -> Result<(), String> {
        let id = update.tree_id;
        if id != TreeId::ROOT && !self.parents.contains_key(&id) {
            return Err(format!("guest {id:?} has no mounted graft"));
        }
        if update.tree.is_some() {
            let grafts: HashMap<_, _> = update
                .nodes
                .iter()
                .filter_map(|(local, node)| Some((*local, node.tree_id()?)))
                .collect();
            for &guest in grafts.values() {
                if self.parents.get(&guest).is_some_and(|parent| *parent != id) {
                    return Err(format!(
                        "guest {guest:?} must detach before it can change parents"
                    ));
                }
            }
            let previous = self.trees.insert(
                id,
                MountedTree {
                    grafts,
                    focus: update.focus,
                },
            );
            if let Some(previous) = previous {
                let current = &self.trees[&id].grafts;
                let forgotten: Vec<_> = previous
                    .grafts
                    .into_values()
                    .filter(|guest| !current.values().any(|current| current == guest))
                    .collect();
                for guest in forgotten {
                    self.retire(guest);
                }
            }
            for &guest in self.trees[&id].grafts.values() {
                self.parents.insert(guest, id);
            }
        } else {
            self.trees
                .get_mut(&id)
                .ok_or("focus update precedes its native tree")?
                .focus = update.focus;
        }
        self.check_focus()
    }

    fn retire(&mut self, tree: TreeId) {
        let mut pending = vec![tree];
        while let Some(tree) = pending.pop() {
            self.parents.remove(&tree);
            if let Some(removed) = self.trees.remove(&tree) {
                pending.extend(removed.grafts.into_values());
            }
        }
    }

    fn check_focus(&self) -> Result<(), String> {
        let mut focus = TreeId::ROOT;
        loop {
            let mounted = self
                .trees
                .get(&focus)
                .ok_or_else(|| format!("native focus enters uninstalled guest {focus:?}"))?;
            let Some(&guest) = mounted.grafts.get(&mounted.focus) else {
                return Ok(());
            };
            focus = guest;
        }
    }
}
