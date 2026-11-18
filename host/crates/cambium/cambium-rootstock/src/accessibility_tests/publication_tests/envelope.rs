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
    let request = publication.request_by_action_id("pin").unwrap();
    assert_eq!(request.target, lifetime_produced().target);
    assert_eq!(request.action, crate::A11yAction::Click);
    assert!(publication.request_by_action_id("Pin").is_none());
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

#[test]
fn action_id_refuses_a_replaced_producer_at_the_same_address() {
    let mut host = counter_host();
    let calls = Rc::new(Cell::new(0));
    let board = || LifetimeBoard {
        calls: calls.clone(),
        revision: Rc::new(Cell::new(0)),
    };
    host.s.shared.producers.register(7, board(), &[]).unwrap();
    let captured = frozen(&host);
    host.s.shared.producers.remove(7);
    host.s.shared.producers.register(7, board(), &[]).unwrap();
    let replacement = frozen(&host);
    assert_eq!(captured.target, replacement.target);
    assert_eq!(
        host.apply_a11y_requests(&[captured]),
        [D::Rejected(R::StaleTarget)]
    );
    assert_eq!(calls.get(), 0);
    assert_eq!(host.apply_a11y_requests(&[replacement]), [D::Dispatched]);
    assert_eq!(calls.get(), 1);
}
