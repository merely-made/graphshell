// Copyright 2026 Mark Alan Boykin
// SPDX-License-Identifier: MPL-2.0

//! Process-wide native node IDs, retained by source address and owner identity.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use accesskit::NodeId;
use uxtree::envelope::TreeAddress;

struct Entry<I> {
    native: NodeId,
    identity: I,
}

pub(super) struct NativeAddresses<I>(HashMap<TreeAddress, Entry<I>>);

impl<I> Default for NativeAddresses<I> {
    fn default() -> Self {
        Self(HashMap::new())
    }
}

impl<I> NativeAddresses<I> {
    pub fn clear(&mut self) {
        self.0.clear();
    }

    pub fn get(&self, source: TreeAddress) -> Option<NodeId> {
        self.0.get(&source).map(|entry| entry.native)
    }

    pub fn has_native_id(&self, node: NodeId) -> bool {
        self.0.values().any(|entry| entry.native == node)
    }

    pub fn remove(&mut self, source: TreeAddress) {
        self.0.remove(&source);
    }

    pub fn insert(&mut self, source: TreeAddress, native: NodeId, identity: I) {
        self.0.insert(source, Entry { native, identity });
    }

    /// Every source address with its native id.
    pub fn iter(&self) -> impl Iterator<Item = (TreeAddress, NodeId)> + '_ {
        self.0.iter().map(|(source, entry)| (*source, entry.native))
    }

    /// The identity a source address is bound under.
    #[cfg(test)]
    pub fn identity(&self, source: TreeAddress) -> Option<&I> {
        self.0.get(&source).map(|entry| &entry.identity)
    }
}

impl<I: PartialEq> NativeAddresses<I> {
    /// The native id `source` keeps under `identity`, as `bind` retains it,
    /// or a fresh one; `true` when kept.
    pub fn lookup(&self, source: TreeAddress, identity: &I) -> (NodeId, bool) {
        self.0
            .get(&source)
            .filter(|entry| entry.identity == *identity)
            .map_or_else(|| (allocate_id(), false), |entry| (entry.native, true))
    }
}

impl<I: PartialEq> NativeAddresses<I> {
    /// Bind a source to a native ID chosen by the caller. True if it is new.
    pub fn pin(
        &mut self,
        previous: &Self,
        source: TreeAddress,
        native: NodeId,
        identity: I,
    ) -> bool {
        self.0.insert(source, Entry { native, identity });
        previous.get(source) != Some(native)
    }

    pub fn bind(
        &mut self,
        previous: &Self,
        source: TreeAddress,
        identity: I,
        renew: bool,
    ) -> (NodeId, bool) {
        let retained = previous
            .0
            .get(&source)
            .filter(|entry| !renew && entry.identity == identity);
        let native = retained.map_or_else(allocate_id, |entry| entry.native);
        self.0.insert(source, Entry { native, identity });
        (native, retained.is_none())
    }
}

pub(super) fn allocate_id() -> NodeId {
    // Unpublished drafts may consume IDs; no ended publication may recycle
    // one. Zero is reserved for the typed host's legacy activation bootstrap.
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NodeId(
        NEXT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
            .expect("native accessibility IDs exhausted"),
    )
}
