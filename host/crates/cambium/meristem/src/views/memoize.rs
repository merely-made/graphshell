// Copyright 2024 the Xilem Authors
// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! A view rebuilt only when its data changes. Restored from the 2026-08-12
//! scope cut for the data grid's rows (table plan D82).

use alloc::vec::Vec;
use core::fmt::Debug;
use core::marker::PhantomData;
use core::mem::size_of;

use crate::{MessageCtx, MessageResult, Mut, View, ViewId, ViewMarker, ViewPathTracker};

/// A view which is built from `data` and rebuilt only when `data` changes.
#[must_use = "View values do nothing unless provided to a Meristem driver."]
pub struct Memoize<Data, InitView, State, Action, Context> {
    data: Data,
    init_view: InitView,
    phantom: PhantomData<fn() -> (State, Action, Context)>,
}

impl<Data, InitView, State, Action, Context> Debug
    for Memoize<Data, InitView, State, Action, Context>
where
    State: 'static,
    Data: Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Memoize")
            .field("data", &self.data)
            .finish_non_exhaustive()
    }
}

const NON_CAPTURING_CLOSURE: &str = "
It's not possible to use function pointers or captured context in closures,
as this potentially messes up the logic of memoize or produces unwanted effects.

For example a different kind of view could be instantiated with a different callback, while the old one is still memoized, but it's not updated then.
It's not possible in Rust currently to check whether the (content of the) callback has changed with the `Fn` trait, which would make this otherwise possible.
";

/// Memoize the view until `data` changes, when `init_view` is called again.
///
/// The view is also rebuilt after a message to it returns
/// [`MessageResult::RequestRebuild`]. When equal data arrives at a different
/// view path (the subtree was adopted elsewhere), the retained view is
/// traversed against itself so its descendants re-register at the new path;
/// `init_view` is not called. `init_view` must capture nothing: all it reads
/// must be in `data`.
///
/// ```ignore
/// fn memoized_label(count: u32) -> impl View<...> {
///     memoize(count, |count| label(format!("clicked {count} times")))
/// }
/// ```
pub fn memoize<State, Action, Context, Data, V, InitView>(
    data: Data,
    init_view: InitView,
) -> Memoize<Data, InitView, State, Action, Context>
where
    State: 'static,
    Data: PartialEq + 'static,
    InitView: Fn(&Data) -> V + 'static,
    V: View<State, Action, Context>,
    Context: ViewPathTracker,
{
    const {
        assert!(size_of::<InitView>() == 0, "{}", NON_CAPTURING_CLOSURE);
    }
    Memoize {
        data,
        init_view,
        phantom: PhantomData,
    }
}

#[expect(
    unnameable_types,
    reason = "Implementation detail, public because of trait visibility rules"
)]
#[derive(Debug)]
pub struct MemoizeState<V, VState> {
    view: V,
    view_state: VState,
    dirty: bool,
    /// The view path of the last build or rebuild.
    path: Vec<ViewId>,
}

impl<Data, ViewFn, State, Action, Context> ViewMarker
    for Memoize<Data, ViewFn, State, Action, Context>
{
}
impl<State, Action, Context, Data, V, ViewFn> View<State, Action, Context>
    for Memoize<Data, ViewFn, State, Action, Context>
where
    State: 'static,
    Action: 'static,
    Context: ViewPathTracker + 'static,
    Data: PartialEq + 'static,
    V: View<State, Action, Context>,
    ViewFn: Fn(&Data) -> V + 'static,
{
    type ViewState = MemoizeState<V, V::ViewState>;

    type Element = V::Element;

    fn build(&self, ctx: &mut Context, app_state: &mut State) -> (Self::Element, Self::ViewState) {
        let view = (self.init_view)(&self.data);
        let (element, view_state) = view.build(ctx, app_state);
        let memoize_state = MemoizeState {
            view,
            view_state,
            dirty: false,
            path: ctx.view_path().to_vec(),
        };
        (element, memoize_state)
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        if core::mem::take(&mut view_state.dirty) || prev.data != self.data {
            let view = (self.init_view)(&self.data);
            view.rebuild(
                &view_state.view,
                &mut view_state.view_state,
                ctx,
                element,
                app_state,
            );
            view_state.view = view;
        } else if ctx.view_path() != view_state.path.as_slice() {
            // Moved, not changed: a routing-only traversal of the retained view.
            let MemoizeState {
                view, view_state, ..
            } = view_state;
            view.rebuild(view, view_state, ctx, element, app_state);
        } else {
            return;
        }
        if ctx.view_path() != view_state.path.as_slice() {
            view_state.path = ctx.view_path().to_vec();
        }
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: Mut<'_, Self::Element>,
    ) {
        view_state
            .view
            .teardown(&mut view_state.view_state, ctx, element);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        message: &mut MessageCtx,
        element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        let message_result =
            view_state
                .view
                .message(&mut view_state.view_state, message, element, app_state);
        if matches!(message_result, MessageResult::RequestRebuild) {
            view_state.dirty = true;
        }
        message_result
    }
}
