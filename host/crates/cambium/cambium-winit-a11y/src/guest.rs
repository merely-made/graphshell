// Copyright 2026 Mark Alan Boykin
// SPDX-License-Identifier: MPL-2.0

//! Retained foreign trees around Mere's validated AccessKit graft composition.
//! Retention keeps source TreeIds and local IDs for observations and admission. Missing declared subtrees
//! wait for their initial update; malformed known trees withdraw their surface.

use accesskit::{ActionRequest, Node, NodeId, Rect, Role, TreeId, TreeInfo, TreeUpdate, Uuid};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::Hash;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use uxtree::envelope::TreeAddress;
use uxtree::graft::{ActionTarget, Composition, GraftTable, Grafts, graft_node};

#[derive(Clone)]
struct RetainedTree {
    tree: TreeInfo,
    nodes: std::sync::Arc<BTreeMap<NodeId, Node>>,
    lives: Arc<BTreeMap<NodeId, u64>>,
    focus: NodeId,
    publication: Grafts,
}

enum MaterializationError {
    Incomplete(String),
    Invalid(String),
}

#[derive(Clone)]
struct SurfaceForest {
    root: TreeId,
    trees: HashMap<TreeId, RetainedTree>,
    declared: HashSet<TreeId>,
    lineages: HashMap<TreeId, u64>,
}

/// Retains supplier trees and each window's source-side publication.
pub struct ForeignA11y<K = Uuid> {
    surfaces: HashMap<K, SurfaceForest>,
    owners: GraftTable<K>,
    publication: Grafts,
    mounted: HashSet<K>,
    dirty: bool,
    // Exact full-node snapshots emitted by the last successful composition.
    // This is an in-process publication witness, not platform activation or
    // assistive-technology acceptance. Pending supplier deltas do not alter it.
    published: HashMap<K, PublishedSurface>,
    published_forest: Vec<TreeUpdate>,
}

/// A supplier's exact composed source updates.
pub struct PublishedSurface {
    lineage: u64,
    root: TreeId,
    trees: BTreeMap<TreeId, TreeUpdate>,
    ordered_updates: Vec<TreeUpdate>,
}

impl PublishedSurface {
    /// The root supplier tree.
    pub fn root(&self) -> TreeId {
        self.root
    }

    /// Full source snapshots retained from the composed batch.
    pub fn trees(&self) -> &BTreeMap<TreeId, TreeUpdate> {
        &self.trees
    }

    /// Source updates in parent-first installation and focus order.
    pub fn ordered_updates(&self) -> &[TreeUpdate] {
        &self.ordered_updates
    }
}

/// Identifies the current supplier, tree attachment and reachable node life.
/// Carries no text or supplier-issued stability claim.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GuestAdmission<K> {
    owner: K,
    lineage: u64,
    life: u64,
}

fn next_lineage() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
        .expect("guest accessibility lifetimes exhausted")
}

#[cfg(test)]
#[path = "guest/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "guest/activation_safe_update.rs"]
mod activation_safe_update;
#[cfg(test)]
use activation_safe_update::activation_safe_update;
#[path = "guest/expand.rs"]
mod expand;
#[path = "guest/impl_std_fmt_display_for_materialization.rs"]
mod impl_std_fmt_display_for_materialization;
#[path = "guest/retained_tree_update.rs"]
mod retained_tree_update;
#[path = "guest/surface_forest_reachable.rs"]
mod surface_forest_reachable;
use expand::expand;
#[path = "guest/foreign_a11y_new.rs"]
mod foreign_a11y_new;
#[path = "guest/foreign_a11y_publication_diagnostic.rs"]
mod foreign_a11y_publication_diagnostic;
#[path = "guest/impl_default_for_foreign_a11y.rs"]
mod impl_default_for_foreign_a11y;
