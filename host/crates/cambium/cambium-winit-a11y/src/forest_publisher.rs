// Copyright 2026 Mark Alan Boykin
// SPDX-License-Identifier: MPL-2.0

//! A window's native accessibility publication and cross-thread admission.

use std::collections::HashMap;

use accesskit::{ActionRequest, Node, NodeId, Role, TreeId, TreeInfo, TreeUpdate};
use genet_winit_host::{AccessKitBridge, BridgeStatus};
use uxtree::envelope::TreeAddress;
use winit::window::Window;

use crate::ActionQueue;
use crate::forest_namespace::ForestNamespace;
use crate::native_addresses::allocate_id;

#[cfg(test)]
#[path = "forest_publisher_tests.rs"]
mod tests;

/// Publishes the assembled source forest in a process-wide native namespace.
/// The application owns admission equality and the final source-side check.
pub struct ForestPublisher<A> {
    bridge: AccessKitBridge,
    queue: ActionQueue<A>,
    namespace: ForestNamespace,
    epoch: Option<u64>,
    host_root: NodeId,
}

impl<A: Clone + Send + 'static> ForestPublisher<A> {
    pub fn install(
        window: &dyn Window,
        queue: ActionQueue<A>,
        wake: impl Fn() + Send + Sync + 'static,
        label: &str,
    ) -> Self {
        queue.clear_publication();
        let actions = queue.clone();
        let bridge = AccessKitBridge::with_action_handler(wake, move |request| {
            actions.enqueue_native(ActionRequest {
                action: request.action,
                target_tree: request.target_tree,
                target_node: request.target_node,
                data: request.data,
            });
        });
        let host_root = allocate_id();
        let mut publisher = Self {
            bridge,
            queue,
            namespace: ForestNamespace::hosted(host_root),
            epoch: None,
            host_root,
        };
        let mut root = Node::new(Role::Window);
        root.set_label(label);
        if let Err(error) = install_native(
            &mut publisher.bridge,
            window,
            root_only(TreeInfo::new(host_root), root),
        ) {
            eprintln!("[cambium-winit] accessibility install failed: {error}");
        }
        publisher
    }

    pub fn status(&self) -> BridgeStatus {
        self.bridge.status()
    }

    pub fn activation_epoch(&self) -> u64 {
        self.bridge.activation_epoch()
    }

    /// Where an address from the current source forest was submitted to the
    /// platform. This does not attest that an AT client has read the node.
    pub fn native_address(&self, source: TreeAddress) -> Option<TreeAddress> {
        self.is_current_activation()
            .then(|| self.namespace.native_address(source))
            .flatten()
    }

    /// Whether the current native publication contains this node ID.
    pub fn has_published_native_id(&self, node: NodeId) -> bool {
        self.is_current_activation() && self.namespace.has_native_id(node)
    }

    fn is_current_activation(&self) -> bool {
        self.epoch == Some(self.activation_epoch())
    }

    pub fn update_window_bounds(&mut self, window: &dyn Window) {
        self.bridge.update_window_bounds(window);
    }

    pub fn update_window_focus(&mut self, focused: bool) {
        self.bridge.update_window_focus(focused);
    }

    /// Submit complete snapshots in graft order, interleaving any necessary
    /// focus-only updates. An inactive or reset adapter returns `Ok(false)`.
    ///
    /// # Errors
    ///
    /// Returns a refusal if the forest, admissions, or native submission are
    /// invalid. A refused or inactive publication admits no actions.
    pub fn publish_at(
        &mut self,
        epoch: u64,
        updates: Vec<TreeUpdate>,
        tree_lineages: HashMap<TreeId, u64>,
        admissions: HashMap<TreeAddress, A>,
        same: impl FnMut(&A, &A) -> bool,
    ) -> Result<bool, String> {
        if self.epoch != Some(epoch) {
            self.clear_publication();
            self.epoch = Some(epoch);
        }
        let result = self.publish_candidate(epoch, updates, &tree_lineages, admissions, same);
        if !matches!(result, Ok(true)) {
            self.clear_publication();
        }
        result
    }

    fn publish_candidate(
        &mut self,
        epoch: u64,
        updates: Vec<TreeUpdate>,
        tree_lineages: &HashMap<TreeId, u64>,
        admissions: HashMap<TreeAddress, A>,
        same: impl FnMut(&A, &A) -> bool,
    ) -> Result<bool, String> {
        let mut changed = self.queue.changed_admissions(&admissions, same);
        let candidate = self
            .namespace
            .prepare(updates, tree_lineages, &mut changed)?;
        let sources: std::collections::HashSet<_> = candidate.sources.values().copied().collect();
        if let Some(address) = admissions.keys().find(|address| !sources.contains(address)) {
            return Err(format!(
                "action admission is outside the native forest: {address:?}"
            ));
        }
        let initial = safe_initial(&candidate.updates[0])?;
        if !submit_native(&mut self.bridge, Some(epoch), initial, candidate.updates)? {
            return Ok(false);
        }
        self.queue
            .publish_native(candidate.sources, admissions, &changed);
        self.namespace = candidate.namespace;
        Ok(true)
    }

    fn clear_publication(&mut self) {
        self.namespace = ForestNamespace::hosted(self.host_root);
        self.queue.clear_publication();
    }
}

pub(super) fn install_native(
    bridge: &mut AccessKitBridge,
    window: &dyn Window,
    initial: TreeUpdate,
) -> Result<(), String> {
    bridge.install(window, initial)
}

/// The single-root host can hand back its whole semantic tree at activation.
/// An assembled forest instead needs a guest-free activation root and epoch.
pub(super) fn submit_root(bridge: &mut AccessKitBridge, tree: TreeUpdate) -> Result<(), String> {
    let mut initial = tree.clone();
    initial.focus = tree
        .tree
        .as_ref()
        .ok_or("missing native activation root")?
        .root;
    submit_native(bridge, None, initial, vec![tree]).map(|_| ())
}

fn submit_native(
    bridge: &mut AccessKitBridge,
    epoch: Option<u64>,
    initial: TreeUpdate,
    ordered: Vec<TreeUpdate>,
) -> Result<bool, String> {
    match epoch {
        Some(epoch) => bridge.update_batch_at(epoch, initial, ordered),
        None => bridge.update_batch(initial, ordered),
    }
}

fn root_only(tree: TreeInfo, node: Node) -> TreeUpdate {
    let root = tree.root;
    TreeUpdate {
        tree_id: TreeId::ROOT,
        nodes: vec![(root, node)],
        focus: root,
        tree: Some(tree),
    }
}

fn safe_initial(update: &TreeUpdate) -> Result<TreeUpdate, String> {
    let tree = update
        .tree
        .as_ref()
        .ok_or("missing native activation root")?;
    let (_, source) = update
        .nodes
        .iter()
        .find(|(id, _)| *id == tree.root)
        .ok_or("missing native activation root node")?;
    // The bootstrap is independent of every guest and every other local ID.
    let mut node = Node::new(source.role());
    if let Some(label) = source.label() {
        node.set_label(label);
    }
    if let Some(bounds) = source.bounds() {
        node.set_bounds(bounds);
    }
    if let Some(transform) = source.transform() {
        node.set_transform(*transform);
    }
    Ok(root_only(tree.clone(), node))
}
