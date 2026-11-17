/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use cambium::{
    AnyView, ContainedSession, GenetAppRunner, GenetCtx, GenetElement, RetainedSurfaceSession,
    RunnerSurfaceSession, custom_leaf, el,
};
use cambium_rootstock::{
    A11yOwner, OwnedLayout, ProducedTexture, ProducerAction, ProducerActionDispatch,
    ProducerChildLifetime, ProducerChildToken, ProducerContext, ProducerNode, ProducerRegistry,
    ProducerRole, ProducerSemantics, TextureProducer, WindowDom, project_semantics,
};
use genet_scripted_dom::ScriptedDom;
use layout_dom_api::{LayoutDom, LocalName, Namespace};
use mere_surface_api::SurfaceAvailability;
use sprigging::LeafRegistry;
use uxtree::tree_id_for_path;

use super::{Address, Envelope, Owner, Provenance, Publication, Stability};

const SLOT: u64 = 7;
const CHILD: u64 = 40;

pub(super) struct Model {
    pub(super) count: usize,
    lifetime: ProducerChildLifetime,
    member: String,
}

impl Model {
    fn new() -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            count: 0,
            lifetime: ProducerChildLifetime::new(),
            member: "member/40".into(),
        }))
    }
}

struct Drawing(Rc<RefCell<Model>>);

impl TextureProducer for Drawing {
    fn render(&mut self, _: &ProducerContext<'_>) -> Option<ProducedTexture> {
        panic!("the semantic probe must not need a GPU draw")
    }

    fn semantics(&self) -> Option<ProducerSemantics> {
        Some(ProducerSemantics {
            role: Some(ProducerRole::Group),
            name: Some("Drawing".into()),
            children: vec![ProducerNode {
                key: CHILD,
                role: ProducerRole::GraphicsObject,
                name: "Item".into(),
                rect: [2.0, 3.0, 40.0, 20.0],
                actions: vec![ProducerAction {
                    id: "pin".into(),
                    label: "Pin item".into(),
                    description: "Keep this member".into(),
                }],
            }],
        })
    }

    fn child_lifetime(&self, key: u64) -> Option<ProducerChildToken> {
        (key == CHILD).then(|| self.0.borrow().lifetime.token())
    }

    fn act_checked(
        &mut self,
        key: u64,
        id: &str,
        expected: Option<&ProducerChildToken>,
    ) -> ProducerActionDispatch {
        let mut model = self.0.borrow_mut();
        if !ProducerChildToken::matches_current(expected, Some(&model.lifetime.token())) {
            return ProducerActionDispatch::StaleTarget;
        }
        if key != CHILD || id != "pin" {
            return ProducerActionDispatch::Declined;
        }
        model.count += 1;
        ProducerActionDispatch::Dispatched
    }
}

fn view(_: &()) -> Box<dyn AnyView<(), (), GenetCtx, GenetElement>> {
    Box::new(el(
        "main",
        custom_leaf::<(), ()>(SLOT, 160, 80).attr("id", "drawing"),
    ))
}

pub(super) struct Session {
    retained: Box<dyn RetainedSurfaceSession>,
    registry: ProducerRegistry,
    model: Rc<RefCell<Model>>,
    live: Rc<Cell<bool>>,
    rebind_old_request: bool,
}

impl Session {
    pub(super) fn new(rebind_old_request: bool) -> Self {
        let runner = GenetAppRunner::new(Rc::new(RefCell::new(ScriptedDom::new())), view, ());
        let retained = ContainedSession::new(RunnerSurfaceSession::new(
            super::super::descriptor("p0.drawing"),
            runner,
            |_: &()| SurfaceAvailability::Available,
            |_: &mut (), _| {},
            |_: ()| Vec::new(),
        ));
        let model = Model::new();
        let mut registry = ProducerRegistry::new();
        registry
            .register(SLOT, Drawing(model.clone()), &[])
            .unwrap();
        Self {
            retained: Box::new(retained),
            registry,
            model,
            live: Rc::new(Cell::new(true)),
            rebind_old_request,
        }
    }

    pub(super) fn count(&self) -> usize {
        self.model.borrow().count
    }

    pub(super) fn replace_guest(&mut self) -> Rc<RefCell<Model>> {
        assert!(self.registry.remove(SLOT));
        let old = std::mem::replace(&mut self.model, Model::new());
        self.registry
            .register(SLOT, Drawing(self.model.clone()), &[])
            .unwrap();
        old
    }
}

impl Owner for Session {
    fn instance(&self) -> accesskit::TreeId {
        tree_id_for_path("p0/drawing")
    }

    fn publish(&self) -> Publication {
        let handle = self.retained.dom();
        let dom = handle.borrow();
        let layout = OwnedLayout::new(&*dom, &[""], 200.0, 120.0, &[], &HashMap::new());
        let leaves = LeafRegistry::new();
        let projection = project_semantics(&*dom, &layout, &leaves, &self.registry, None);
        assert!(projection.producer_issues.is_empty());
        let owner = A11yOwner::new(&self.live);
        let bindings = projection
            .publication_bindings(&owner, &WindowDom::document(&dom), &leaves, &self.registry)
            .unwrap();
        let envelopes = projection
            .tree
            .nodes
            .iter()
            .map(|(id, _)| {
                let action = projection.produced_targets.get(id);
                let key = projection
                    .dom_targets
                    .get(id)
                    .and_then(|node| {
                        dom.attribute(*node, &Namespace::default(), &LocalName::from("id"))
                    })
                    .map(str::to_owned);
                Envelope {
                    address: Address {
                        instance: self.instance(),
                        local: *id,
                    },
                    stability: Stability::Life,
                    provenance: Provenance {
                        owner: owner.clone(),
                        request: bindings.requests.get(&id.0).cloned(),
                        child: action.map(|_| self.model.borrow().lifetime.token()),
                        identity: Some(bindings.identities[id].clone()),
                    },
                    action_ids: action
                        .map(|action| vec![action.id.clone()])
                        .unwrap_or_default(),
                    domain_ref: action
                        .filter(|action| action.key == CHILD)
                        .map(|_| self.model.borrow().member.clone()),
                    authored_key: key,
                }
            })
            .collect();
        let mut update = projection.tree;
        update.tree_id = self.instance();
        Publication { update, envelopes }
    }

    fn current(&self, observation: &Envelope) -> bool {
        observation.provenance.owner.is_live()
            && self.publish().envelopes.iter().any(|fresh| {
                fresh.address == observation.address
                    && fresh.provenance.owner == observation.provenance.owner
                    && fresh.provenance.identity == observation.provenance.identity
                    && match (&fresh.provenance.request, &observation.provenance.request) {
                        (Some(a), Some(b)) => a.same_publication_target(b),
                        (None, None) => true,
                        _ => false,
                    }
            })
    }

    fn invoke(&mut self, observation: &Envelope, action_id: &str) -> bool {
        let fresh = self.publish();
        let Some(current) = fresh
            .envelopes
            .iter()
            .find(|e| e.address == observation.address)
        else {
            return false;
        };
        // Deliberately broken control: resolve an old observation as today's target.
        let admitted = if self.rebind_old_request {
            current
        } else {
            observation
        };
        if !self.current(admitted) {
            return false;
        }
        let handle = self.retained.dom();
        let dom = handle.borrow();
        let layout = OwnedLayout::new(&*dom, &[""], 200.0, 120.0, &[], &HashMap::new());
        let projection =
            project_semantics(&*dom, &layout, &LeafRegistry::new(), &self.registry, None);
        let Some(action) = projection.produced_targets.get(&observation.address.local) else {
            return false;
        };
        action.id == action_id
            && self
                .registry
                .act_checked(action, admitted.provenance.child.as_ref())
                == ProducerActionDispatch::Dispatched
    }
}
