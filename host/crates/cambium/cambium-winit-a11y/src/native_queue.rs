// Copyright 2026 Mark Alan Boykin
// SPDX-License-Identifier: MPL-2.0

//! Capture translated targets and admission in the native action callback.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, MutexGuard};

use accesskit::{ActionData, ActionRequest, NodeId};
use uxtree::envelope::TreeAddress;
use uxtree::generation::{GenerationMap, Published};

type Addresses = HashMap<TreeAddress, TreeAddress>;

pub struct QueuedAction<A> {
    pub request: ActionRequest,
    pub admission: Option<Published<A>>,
    pub native_address: Option<TreeAddress>,
    native_selection: Option<[NodeId; 2]>,
}

struct QueueState<A> {
    requests: Vec<QueuedAction<A>>,
    sources: Addresses,
    admissions: GenerationMap<TreeAddress, A>,
}

/// Per-window queue and the bindings that were served to the adapter.
pub struct ActionQueue<A>(Arc<Mutex<QueueState<A>>>);

impl<A> Clone for ActionQueue<A> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<A> Default for ActionQueue<A> {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(QueueState {
            requests: Vec::new(),
            sources: HashMap::new(),
            admissions: GenerationMap::default(),
        })))
    }
}

impl<A> ActionQueue<A> {
    fn state(&self) -> MutexGuard<'_, QueueState<A>> {
        self.0
            .lock()
            .expect("native accessibility action queue poisoned")
    }

    pub fn take(&self) -> Vec<QueuedAction<A>> {
        std::mem::take(&mut self.state().requests)
    }

    pub fn clear_publication(&self) {
        let mut state = self.state();
        state.sources.clear();
        state.admissions.publish(HashMap::new(), |_, _| false);
    }

    pub fn is_current(&self, request: &ActionRequest, captured: &Published<A>) -> bool {
        self.state()
            .admissions
            .is_current(&(request.target_tree, request.target_node), captured)
    }

    /// Whether the native target still denotes the source captured by the
    /// callback. Explicit source requests have no native target.
    pub fn native_target_is_current(&self, queued: &QueuedAction<A>) -> bool {
        queued.native_address.is_none_or(|address| {
            let state = self.state();
            if state.sources.get(&address).copied()
                != Some((queued.request.target_tree, queued.request.target_node))
            {
                return false;
            }
            match queued.native_selection {
                Some([anchor, focus]) => {
                    state.sources.contains_key(&(address.0, anchor))
                        && state.sources.contains_key(&(address.0, focus))
                },
                None => true,
            }
        })
    }

    /// Supply source admissions without a native publication, for in-process
    /// controls. Native callbacks require a successful publisher submission.
    pub fn publish_source(
        &self,
        admissions: HashMap<TreeAddress, A>,
        same: impl FnMut(&A, &A) -> bool,
    ) {
        let mut state = self.state();
        state.sources.clear();
        state.admissions.publish(admissions, same);
    }

    pub(super) fn changed_admissions(
        &self,
        candidate: &HashMap<TreeAddress, A>,
        mut same: impl FnMut(&A, &A) -> bool,
    ) -> HashSet<TreeAddress> {
        let state = self.state();
        let mut changed: HashSet<_> = candidate
            .iter()
            .filter(|(address, next)| {
                state
                    .admissions
                    .get(address)
                    .is_none_or(|old| !same(&old.admission, next))
            })
            .map(|(address, _)| *address)
            .collect();
        changed.extend(
            state
                .admissions
                .entries()
                .keys()
                .filter(|address| !candidate.contains_key(address)),
        );
        changed
    }

    pub(super) fn publish_native(
        &self,
        sources: Addresses,
        admissions: HashMap<TreeAddress, A>,
        changed: &HashSet<TreeAddress>,
    ) {
        let mut state = self.state();
        state.sources = sources;
        // Use the same equivalence result as the allocator. In particular
        // different bindings of a reused source address cannot share an ID.
        state
            .admissions
            .publish_by_key(admissions, |address, _, _| !changed.contains(address));
    }
}

impl<A: Clone> ActionQueue<A> {
    pub fn admission(&self, request: &ActionRequest) -> Option<Published<A>> {
        self.state()
            .admissions
            .get(&(request.target_tree, request.target_node))
            .cloned()
    }

    /// Capture an explicit action expressed in source IDs, such as a fixture
    /// input. The source still needs to bind the action to its owner.
    pub fn enqueue_source(&self, request: ActionRequest) {
        let mut state = self.state();
        let admission = state
            .admissions
            .get(&(request.target_tree, request.target_node))
            .cloned();
        state.requests.push(QueuedAction {
            request,
            admission,
            native_address: None,
            native_selection: None,
        });
    }

    pub(super) fn enqueue_native(&self, mut request: ActionRequest) {
        let mut state = self.state();
        let native = (request.target_tree, request.target_node);
        let Some(&source) = state.sources.get(&native) else {
            return;
        };
        let native_selection =
            if let Some(ActionData::SetTextSelection(selection)) = request.data.as_mut() {
                let native_selection = [selection.anchor.node, selection.focus.node];
                for position in [&mut selection.anchor, &mut selection.focus] {
                    let Some(&(_, id)) = state.sources.get(&(native.0, position.node)) else {
                        return;
                    };
                    position.node = id;
                }
                Some(native_selection)
            } else {
                None
            };
        (request.target_tree, request.target_node) = source;
        let admission = state.admissions.get(&source).cloned();
        state.requests.push(QueuedAction {
            request,
            admission,
            native_address: Some(native),
            native_selection,
        });
    }
}
