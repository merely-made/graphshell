/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

//! uxtree P0: a disposable envelope sketch beside three real projections.
//! Production types, publication changes and the full lint remain P1 onward.

mod consumer;
mod document;
mod paths;
mod texture;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use accesskit::{NodeId, TreeId, TreeUpdate};
use cambium_rootstock::{A11yOwner, A11yRequest, ProducerChildToken, SemanticNodeIdentity};
use uxtree::Composition;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Address {
    instance: TreeId,
    local: NodeId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stability {
    Life,
    Scoped(u64),
    Derived,
}

#[derive(Clone, Debug)]
struct Provenance {
    owner: A11yOwner,
    request: Option<A11yRequest>,
    child: Option<ProducerChildToken>,
    identity: Option<SemanticNodeIdentity>,
}

/// Exactly the proposed six fields; AccessKit retains all node semantics.
#[derive(Clone, Debug)]
struct Envelope {
    address: Address,
    stability: Stability,
    provenance: Provenance,
    action_ids: Vec<String>,
    domain_ref: Option<String>,
    authored_key: Option<String>,
}

struct Publication {
    update: TreeUpdate,
    envelopes: Vec<Envelope>,
}

trait Owner {
    fn instance(&self) -> TreeId;
    fn publish(&self) -> Publication;
    fn current(&self, observation: &Envelope) -> bool;
    fn invoke(&mut self, observation: &Envelope, action_id: &str) -> bool;
}

type Owners = HashMap<TreeId, Rc<RefCell<dyn Owner>>>;

fn read(owners: &Owners) -> consumer::Read {
    let mut root = None;
    let mut guests = Vec::new();
    let mut envelopes = HashMap::new();
    for owner in owners.values() {
        let publication = owner.borrow().publish();
        for envelope in publication.envelopes {
            assert!(envelopes.insert(envelope.address, envelope).is_none());
        }
        if publication.update.tree_id == TreeId::ROOT {
            root = Some(publication.update);
        } else {
            guests.push((publication.update.tree_id, publication.update));
        }
    }
    consumer::Read::new(
        Composition::new(root.unwrap(), guests).unwrap(),
        envelopes,
        owners.clone(),
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Control {
    Clean,
    RebindOldRequest,
    DisableMissingCheck,
    FreezeScope,
}

fn gate(control: Control) {
    let drawing = Rc::new(RefCell::new(texture::Session::new(
        control == Control::RebindOldRequest,
    )));
    let page = Rc::new(RefCell::new(document::Session::new(
        control == Control::FreezeScope,
    )));
    let chrome = Rc::new(RefCell::new(paths::Session::new(
        drawing.borrow().instance(),
        page.borrow().instance(),
    )));
    let sources: Vec<Rc<RefCell<dyn Owner>>> = vec![drawing.clone(), page.clone(), chrome];
    let owners: Owners = sources
        .into_iter()
        .map(|owner| {
            let id = owner.borrow().instance();
            (id, owner)
        })
        .collect();
    let mut first = read(&owners);
    assert!(first.missing().is_empty());
    let addresses = first.addresses();
    for address in &addresses {
        let observed = first
            .envelope(*address)
            .expect("every published node has an envelope");
        assert_eq!(observed.address, *address);
        assert!(observed.provenance.owner.is_live());
        assert!(first.current(observed));
    }

    let command = first.named("Pin item").clone();
    let field = first.named("Document value").clone();
    let menu = first.named("Host menu").clone();
    assert_eq!(command.stability, Stability::Life);
    assert_eq!(command.domain_ref.as_deref(), Some("member/40"));
    assert_eq!(command.action_ids, ["pin"]);
    assert_eq!(field.stability, Stability::Scoped(page.borrow().revision()));
    assert!(field.authored_key.is_none());
    assert_eq!(
        first.named("Drawing").authored_key.as_deref(),
        Some("drawing")
    );
    assert_eq!(menu.stability, Stability::Derived);
    // Neither source advertises command IDs. Do not invent them from roles.
    assert!(field.action_ids.is_empty());
    assert!(menu.action_ids.is_empty());
    assert!(first.invoke(&command, "pin"));
    assert_eq!(drawing.borrow().count(), 1);
    assert!(!first.invoke(&command, "unadvertised"));
    assert!(!first.invoke(&field, "pin"));
    assert!(!first.invoke(&menu, "pin"));

    // The missing-envelope check walks the actual consumer tree, not the map.
    let removed = first.envelopes.remove(&field.address).unwrap();
    let missing = if control == Control::DisableMissingCheck {
        Vec::new()
    } else {
        first.missing()
    };
    assert_eq!(
        missing,
        [field.address],
        "missing-envelope detector must report the planted node"
    );
    first.envelopes.insert(removed.address, removed);
    assert!(first.missing().is_empty());

    // Genet, rather than this sketch, advances and checks the semantic revision.
    page.borrow_mut().change_value();
    let second = read(&owners);
    let current_field = second.named("Document value");
    assert_eq!(current_field.address, field.address);
    assert_ne!(
        current_field.stability, field.stability,
        "scope must follow the document revision"
    );
    assert!(!second.current(&field));
    assert!(second.current(current_field));

    // Retire the texture guest and replace it under its existing DOM slot.
    // Keep the old model alive: an old token's mere liveness is insufficient.
    let retired = drawing.borrow_mut().replace_guest();
    let replacement = read(&owners);
    let new_command = replacement.named("Pin item").clone();
    assert_eq!(
        new_command.address, command.address,
        "the control must really reuse NodeId"
    );
    assert!(
        !replacement.invoke(&command, "pin"),
        "retired guest request must be refused at its owner"
    );
    assert_eq!(retired.borrow().count, 1);
    assert_eq!(drawing.borrow().count(), 0);
    assert!(replacement.invoke(&new_command, "pin"));
    assert_eq!(drawing.borrow().count(), 1);
    assert!(replacement.missing().is_empty());
    println!(
        "P0: {} addressed nodes; Life/Scoped/Derived read through one consumer; command dispatched; retired reused target refused; missing/clean controls passed",
        addresses.len()
    );
}

#[test]
fn p0_federated_gate() {
    gate(Control::Clean);
}

#[test]
#[should_panic(expected = "retired guest request must be refused at its owner")]
fn p0_control_rebinding_an_old_request_retargets_the_replacement() {
    gate(Control::RebindOldRequest);
}

#[test]
#[should_panic(expected = "missing-envelope detector must report the planted node")]
fn p0_control_disabled_lint_misses_the_planted_node() {
    gate(Control::DisableMissingCheck);
}

#[test]
#[should_panic(expected = "scope must follow the document revision")]
fn p0_control_frozen_scope_misses_a_document_change() {
    gate(Control::FreezeScope);
}
