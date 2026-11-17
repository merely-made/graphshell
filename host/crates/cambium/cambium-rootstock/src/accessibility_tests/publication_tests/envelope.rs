// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

use super::*;

fn frozen(host: &Host<u32, CounterLogic, CounterChild>) -> A11yRequest {
    let runner = host.s.runner.as_ref().unwrap();
    let dom = runner.dom();
    let dom = dom.borrow();
    let window = WindowDom::new(&dom, runner.mount());
    let projection = crate::project_semantics(
        &window,
        host.s.layout.as_ref().unwrap(),
        &host.s.shared.leaves,
        &host.s.shared.producers,
        None,
    );
    let publication = projection
        .publication_envelopes(
            &crate::A11yOwner::new(&host.s.a11y_owner),
            &window,
            &host.s.shared.leaves,
            &host.s.shared.producers,
        )
        .unwrap();
    let raw = lifetime_produced();
    let (id, mut request) = publication
        .requests
        .into_iter()
        .find(|(_, r)| r.target == raw.target)
        .unwrap();
    let envelope = &publication.envelopes[&accesskit::NodeId(id)];
    assert_eq!(request.provenance.as_ref(), Some(&envelope.provenance.0));
    assert_eq!(envelope.action_ids, ["pin"]);
    request.action = raw.action;
    request
}

#[test]
fn envelope_route_refuses_ended_child_even_when_old_marker_is_retained() {
    let mut host = counter_host();
    let life = Rc::new(RefCell::new(Some(ProducerChildLifetime::new())));
    let calls = Rc::new(Cell::new(0));
    host.s
        .shared
        .producers
        .register(
            7,
            ChildBoard {
                life: life.clone(),
                calls: calls.clone(),
                replace_on_read: Rc::new(Cell::new(false)),
            },
            &[],
        )
        .unwrap();
    let old = frozen(&host);
    let retained_old_marker = life.borrow_mut().take();
    *life.borrow_mut() = Some(ProducerChildLifetime::new());
    assert_eq!(
        host.apply_a11y_requests(&[old]),
        [D::Rejected(R::StaleTarget)]
    );
    assert_eq!(calls.get(), 0);
    assert_eq!(host.apply_a11y_requests(&[frozen(&host)]), [D::Dispatched]);
    assert_eq!(calls.get(), 1);
    drop(retained_old_marker);
}
