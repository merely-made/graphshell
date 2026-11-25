// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The retained composition behind an adapter that consumes change sets
//! (table plan D64, D76): kept only while that adapter, owner and DOM stand.
use super::super::*;
use crate::semantics::retained::RetainedSemantics;

pub(crate) struct RetainedPublication {
    adapter: usize,
    owner: Rc<std::cell::Cell<bool>>,
    dom: std::rc::Weak<std::cell::RefCell<crate::ScriptedDom>>,
    document: NodeId,
    semantics: RetainedSemantics,
}

use std::rc::Rc;

impl<State, Logic, V, T> Host<State, Logic, V, T>
where
    T: crate::HostTree<State>,
    State: 'static,
    Logic: FnMut(&State) -> V + 'static,
    V: RootView<State>,
{
    /// Whether this publication goes through the retained composition: the
    /// adapter consumes change sets, no contributor describes itself through
    /// callbacks (`stable`), and the window shows the whole document.
    pub(super) fn retains_a11y(&self, stable: bool) -> bool {
        let (Some(adapter), Some(runner)) = (self.s.a11y.as_deref(), self.s.runner.as_ref()) else {
            return false;
        };
        let dom = runner.dom();
        let dom = dom.borrow();
        stable
            && adapter.consumes_changes()
            && crate::WindowDom::new(&dom, runner.mount()).document() == dom.document()
    }

    pub(super) fn sync_a11y_composed(&mut self, layout_scale: f64) -> crate::A11ySync {
        let s = &mut self.s;
        let runner = s.runner.as_ref().expect("publication requires a runner");
        let layout = s.layout.as_mut().expect("publication requires layout");
        let adapter = s.a11y.as_mut().expect("publication requires an adapter");
        let (dom, mount) = (runner.dom(), runner.mount());
        let dom_ref = dom.borrow();
        let view = crate::WindowDom::new(&dom_ref, mount);
        let focus = runner.focus().map(|node| view.opaque_id(node));
        let (drained, pending) = dom_ref.pending_mutations();
        let pending = pending.len();
        let journal = layout
            .take_a11y_journal()
            .filter(|journal| journal.is_complete(drained, pending));
        let address = std::ptr::from_ref(&**adapter).cast::<()>() as usize;
        let (leaves, producers) = (&s.shared.leaves, &s.shared.producers);
        let kept = s.a11y_retained.as_mut().filter(|kept| {
            kept.adapter == address
                && Rc::ptr_eq(&kept.owner, &s.a11y_owner)
                && kept.dom.ptr_eq(&Rc::downgrade(&dom))
                && kept.document == view.document()
        });
        if let Some(kept) = kept {
            kept.semantics
                .update(&view, layout, focus, leaves, producers, journal);
        } else {
            s.a11y_retained = Some(RetainedPublication {
                adapter: address,
                owner: s.a11y_owner.clone(),
                dom: Rc::downgrade(&dom),
                document: view.document(),
                semantics: RetainedSemantics::new(&view, layout, focus, leaves, producers),
            });
        }
        layout.begin_a11y_journal(drained);
        let composed = s
            .a11y_retained
            .as_ref()
            .expect("retained above")
            .semantics
            .composed(leaves, producers);
        adapter.sync_composed(
            &crate::A11yOwner::new(&s.a11y_owner),
            &view,
            composed,
            layout_scale,
        )
    }
}
