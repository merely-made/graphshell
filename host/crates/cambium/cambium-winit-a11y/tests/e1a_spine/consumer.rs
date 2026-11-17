/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

//! An address reader and command consumer, without source-specific policy.

use std::collections::HashMap;

use accesskit_consumer::{NodeRef, Tree, TreeChangeHandler};
use uxtree::{Composition, Grafts};

use super::{Address, Envelope, Owners};

struct Ignore;
impl TreeChangeHandler for Ignore {
    fn node_added(&mut self, _: &NodeRef<'_>) {}
    fn node_updated(&mut self, _: &NodeRef<'_>, _: &NodeRef<'_>) {}
    fn focus_moved(&mut self, _: Option<&NodeRef<'_>>, _: Option<&NodeRef<'_>>) {}
    fn node_removed(&mut self, _: &NodeRef<'_>) {}
}

pub(super) struct Read {
    tree: Tree,
    pub(super) envelopes: HashMap<Address, Envelope>,
    owners: Owners,
}

impl Read {
    pub(super) fn new(
        composition: Composition,
        envelopes: HashMap<Address, Envelope>,
        owners: Owners,
    ) -> Self {
        let mut grafts = Grafts::new();
        let mut tree = Tree::new(grafts.activate(&composition), true);
        for update in grafts.frame(composition) {
            tree.update_and_process_changes(update, &mut Ignore);
        }
        Self {
            tree,
            envelopes,
            owners,
        }
    }

    pub(super) fn addresses(&self) -> Vec<Address> {
        fn walk(node: NodeRef<'_>, output: &mut Vec<Address>) {
            let (local, instance) = node.locate();
            output.push(Address { instance, local });
            for child in node.children() {
                walk(child, output);
            }
        }
        let mut output = Vec::new();
        walk(self.tree.state().root(), &mut output);
        output
    }

    pub(super) fn envelope(&self, address: Address) -> Option<&Envelope> {
        self.addresses()
            .contains(&address)
            .then(|| self.envelopes.get(&address))
            .flatten()
    }

    pub(super) fn named(&self, name: &str) -> &Envelope {
        fn find(node: NodeRef<'_>, name: &str) -> Option<Address> {
            if node.label().as_deref() == Some(name) {
                let (local, instance) = node.locate();
                return Some(Address { instance, local });
            }
            node.children().find_map(|child| find(child, name))
        }
        self.envelope(
            find(self.tree.state().root(), name)
                .unwrap_or_else(|| panic!("named node {name:?} exists")),
        )
        .unwrap()
    }

    pub(super) fn missing(&self) -> Vec<Address> {
        self.addresses()
            .into_iter()
            .filter(|address| !self.envelopes.contains_key(address))
            .collect()
    }

    pub(super) fn current(&self, observation: &Envelope) -> bool {
        self.owners
            .get(&observation.address.instance)
            .is_some_and(|owner| owner.borrow().current(observation))
    }

    pub(super) fn invoke(&self, observation: &Envelope, action_id: &str) -> bool {
        observation.action_ids.iter().any(|id| id == action_id)
            && self
                .owners
                .get(&observation.address.instance)
                .is_some_and(|owner| owner.borrow_mut().invoke(observation, action_id))
    }
}
