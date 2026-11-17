/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::cell::Cell;
use std::rc::Rc;

use accesskit::{Node, Rect, Role, TreeId};
use cambium_rootstock::A11yOwner;
use uxtree::{UxTree, graft_node, node_id_for_path, stitch};

use super::{Address, Envelope, Owner, Provenance, Publication, Stability};

pub(super) struct Session {
    guests: [TreeId; 2],
    // One route-owner life, never a frame count or a semantic-read allocation.
    live: Rc<Cell<bool>>,
}

impl Session {
    pub(super) fn new(first: TreeId, second: TreeId) -> Self {
        Self {
            guests: [first, second],
            live: Rc::new(Cell::new(true)),
        }
    }
}

impl Owner for Session {
    fn instance(&self) -> TreeId {
        TreeId::ROOT
    }

    fn publish(&self) -> Publication {
        let mut menu = Node::new(Role::Button);
        menu.set_label("Host menu");
        menu.set_bounds(Rect::new(0.0, 0.0, 80.0, 30.0));
        let subtrees = [
            ("p0/chrome/menu", menu),
            (
                "p0/chrome/first",
                graft_node(Role::Pane, self.guests[0], (0.0, 40.0), (200.0, 120.0)),
            ),
            (
                "p0/chrome/second",
                graft_node(Role::Pane, self.guests[1], (200.0, 40.0), (200.0, 120.0)),
            ),
        ]
        .into_iter()
        .map(|(path, node)| {
            let root = node_id_for_path(path);
            UxTree {
                root,
                nodes: vec![(root, node)],
            }
        })
        .collect();
        let mut root = Node::new(Role::Window);
        root.set_label("P0 window");
        root.set_bounds(Rect::new(0.0, 0.0, 400.0, 160.0));
        let tree = stitch("p0/chrome", root, subtrees);
        let envelopes = tree
            .nodes
            .iter()
            .map(|(id, _)| Envelope {
                address: Address {
                    instance: self.instance(),
                    local: *id,
                },
                stability: Stability::Derived,
                provenance: Provenance {
                    owner: A11yOwner::new(&self.live),
                    request: None,
                    child: None,
                    identity: None,
                },
                action_ids: Vec::new(),
                domain_ref: None,
                authored_key: None,
            })
            .collect();
        Publication {
            update: tree.to_tree_update(None),
            envelopes,
        }
    }

    fn current(&self, observation: &Envelope) -> bool {
        observation.provenance.owner.is_live()
            && observation.provenance.owner == A11yOwner::new(&self.live)
            && observation.stability == Stability::Derived
            && self
                .publish()
                .envelopes
                .iter()
                .any(|fresh| fresh.address == observation.address)
    }

    fn invoke(&mut self, _: &Envelope, _: &str) -> bool {
        false
    }
}
