// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Tests for [`memoize`]: built once, skipped on equal data, rebuilt on
//! changed data or after a message asks for a rebuild.

use meristem::{DynMessage, MessageResult, View, ViewId, ViewPathTracker, memoize};

mod common;
use common::*;

fn record(id: &u32) -> OperationView<0> {
    OperationView(*id)
}

#[test]
fn builds_from_data() {
    let view = memoize(3_u32, record);
    let mut ctx = TestCtx::default();
    let (element, _) = view.build(&mut ctx, &mut ());
    ctx.assert_empty();
    assert_eq!(element.operations, &[Operation::Build(3)]);
}

#[test]
fn equal_data_skips_rebuild() {
    let view1 = memoize(0_u32, record);
    let mut ctx = TestCtx::default();
    let (mut element, mut state) = view1.build(&mut ctx, &mut ());
    let view2 = memoize(0_u32, record);
    view2.rebuild(&view1, &mut state, &mut ctx, &mut element, &mut ());
    ctx.assert_empty();
    assert_eq!(element.operations, &[Operation::Build(0)]);
}

#[test]
fn changed_data_rebuilds() {
    let view1 = memoize(0_u32, record);
    let mut ctx = TestCtx::default();
    let (mut element, mut state) = view1.build(&mut ctx, &mut ());
    let view2 = memoize(1_u32, record);
    view2.rebuild(&view1, &mut state, &mut ctx, &mut element, &mut ());
    // The next rebuild diffs against the view made from the changed data.
    let view3 = memoize(2_u32, record);
    view3.rebuild(&view2, &mut state, &mut ctx, &mut element, &mut ());
    ctx.assert_empty();
    assert_eq!(
        element.operations,
        &[
            Operation::Build(0),
            Operation::Rebuild { from: 0, to: 1 },
            Operation::Rebuild { from: 1, to: 2 },
        ]
    );
}

#[test]
fn request_rebuild_message_forces_one_rebuild() {
    fn asks(id: &u32) -> impl View<(), Action, TestCtx, Element = TestElement> + use<> {
        OperationView::<0>(*id).map_message_result(|_: &mut (), _| MessageResult::RequestRebuild)
    }
    let view1 = memoize(0_u32, asks);
    let mut ctx = TestCtx::default();
    let (mut element, mut state) = view1.build(&mut ctx, &mut ());
    ctx.with_message_context(element.view_path.clone(), DynMessage::new(()), |message| {
        let result = view1.message(&mut state, message, &mut element, &mut ());
        assert!(matches!(result, MessageResult::RequestRebuild));
    });
    let view2 = memoize(0_u32, asks);
    view2.rebuild(&view1, &mut state, &mut ctx, &mut element, &mut ());
    // The dirty mark is spent: equal data skips again.
    let view3 = memoize(0_u32, asks);
    view3.rebuild(&view2, &mut state, &mut ctx, &mut element, &mut ());
    ctx.assert_empty();
    assert_eq!(
        element.operations,
        &[Operation::Build(0), Operation::Rebuild { from: 0, to: 0 }]
    );
}

#[test]
fn messages_reach_the_inner_view() {
    let view = memoize(5_u32, record);
    let mut ctx = TestCtx::default();
    let (mut element, mut state) = view.build(&mut ctx, &mut ());
    ctx.with_message_context(element.view_path.clone(), DynMessage::new(()), |message| {
        assert_action(view.message(&mut state, message, &mut element, &mut ()), 5);
    });
}

#[test]
fn teardown_reaches_the_inner_view() {
    let view = memoize(4_u32, record);
    let mut ctx = TestCtx::default();
    let (mut element, mut state) = view.build(&mut ctx, &mut ());
    view.teardown(&mut state, &mut ctx, &mut element);
    ctx.assert_empty();
    assert_eq!(
        element.operations,
        &[Operation::Build(4), Operation::Teardown(4)]
    );
}

#[test]
fn equal_data_at_a_new_path_traverses_without_init() {
    let view1 = memoize(0_u32, record);
    let mut ctx = TestCtx::default();
    let (mut element, mut state) = view1.build(&mut ctx, &mut ());
    // The subtree is adopted one level deeper.
    ctx.push_id(ViewId::new(7));
    element.view_path = ctx.view_path().to_vec();
    let view2 = memoize(0_u32, record);
    view2.rebuild(&view1, &mut state, &mut ctx, &mut element, &mut ());
    // Settled at the new path: skipped again.
    let view3 = memoize(0_u32, record);
    view3.rebuild(&view2, &mut state, &mut ctx, &mut element, &mut ());
    ctx.pop_id();
    ctx.assert_empty();
    assert_eq!(
        element.operations,
        &[Operation::Build(0), Operation::Rebuild { from: 0, to: 0 }]
    );
}
