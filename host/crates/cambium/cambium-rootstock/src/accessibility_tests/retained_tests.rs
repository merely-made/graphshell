// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! A host whose adapter consumes change sets publishes the retained
//! composition: each one equals a full composition, a scroll patches, and a
//! leaf or producer publishes the whole tree (table plan D64, D76).

use super::*;
use crate::{A11yAdapterState, A11yPublication, A11yPublicationOutcome, SemanticChanges};
use accesskit::NodeId as A11yNodeId;
use std::collections::HashMap;

#[derive(Default)]
struct Seen {
    whole_syncs: usize,
    /// Each composed publication: its change set, nodes, focus and routes.
    composed: Vec<Composed>,
}

struct Composed {
    changes: Option<SemanticChanges>,
    nodes: HashMap<A11yNodeId, accesskit::Node>,
    focus: A11yNodeId,
    targets: HashMap<A11yNodeId, crate::NodeId>,
}

struct Consuming(Rc<RefCell<Seen>>);

fn submitted() -> A11yPublication {
    A11yPublication {
        adapter: A11yAdapterState::Installed,
        outcome: A11yPublicationOutcome::Submitted {
            nodes: 1,
            contribution_issues: 0,
        },
    }
}

impl Accessibility for Consuming {
    fn adapter_state(&self) -> A11yAdapterState {
        A11yAdapterState::Installed
    }

    fn sync(
        &mut self,
        _: &crate::A11yOwner,
        _: &WindowDom<'_>,
        _: &OwnedLayout,
        _: &sprigging::LeafRegistry<u64>,
        _: &ProducerRegistry,
        _: Option<u64>,
        _: f64,
    ) -> crate::A11ySync {
        self.0.borrow_mut().whole_syncs += 1;
        crate::A11ySync {
            requests: Vec::new(),
            publication: submitted(),
        }
    }

    fn consumes_changes(&self) -> bool {
        true
    }

    fn sync_composed(
        &mut self,
        _: &crate::A11yOwner,
        _: &WindowDom<'_>,
        composed: crate::ComposedSemantics<'_>,
        _: f64,
    ) -> crate::A11ySync {
        let projection = composed.projection;
        self.0.borrow_mut().composed.push(Composed {
            changes: composed.changes.cloned(),
            nodes: projection.tree.nodes.iter().cloned().collect(),
            focus: projection.tree.focus,
            targets: projection.dom_targets.clone(),
        });
        crate::A11ySync {
            requests: Vec::new(),
            publication: submitted(),
        }
    }
}

struct Grid {
    scroll: f32,
}

type GridChild = Box<dyn AnyView<Grid, (), GenetCtx, GenetElement>>;
type GridLogic = fn(&Grid) -> GridChild;

fn grid_view(state: &Grid) -> GridChild {
    let spec = sprigging::GridSpec {
        columns: vec![
            sprigging::GridColumn::new("Name", 120.0),
            sprigging::GridColumn::new("Value", 80.0),
        ],
        row_height: 24.0,
        header_height: 28.0,
        overscan: 2,
    };
    cambium::data_grid(
        &spec,
        1000,
        240.0,
        state.scroll,
        |r, c| Box::new(el::<_, Grid, ()>("span", format!("r{r}c{c}"))),
        |_: &mut Grid, _| {},
        |_| None,
    )
}

type GridHost = Host<Grid, GridLogic, GridChild>;

fn grid_host(seen: &Rc<RefCell<Seen>>) -> GridHost {
    let mut state: HostState<Grid, GridLogic, GridChild> = HostState::new();
    state.shared.sheet = "span { display: block; }".into();
    state.runner = Some(Runner::new(
        Rc::new(RefCell::new(ScriptedDom::new())),
        grid_view as GridLogic,
        Grid { scroll: 0.0 },
    ));
    state.a11y = Some(Box::new(Consuming(seen.clone())));
    let wake = HostWake::new(state.wake_pending.clone(), Arc::new(|| {}));
    Host::new(
        HostOptions::default(),
        None,
        HostHooks::inert(),
        state,
        wake,
    )
}

/// Lay out, publish, and check the composed tree against a full one.
fn frame(host: &mut GridHost, seen: &Rc<RefCell<Seen>>) -> Option<SemanticChanges> {
    host.relayout(400.0, 300.0);
    host.sync_a11y();
    let runner = host.s.runner.as_ref().unwrap();
    let dom = runner.dom();
    let dom = dom.borrow();
    let view = WindowDom::new(&dom, runner.mount());
    let focus = runner.focus().map(|node| view.opaque_id(node));
    let full = crate::project_semantics(
        &view,
        host.s.layout.as_ref().unwrap(),
        &host.s.shared.leaves,
        &host.s.shared.producers,
        focus,
    );
    let seen = seen.borrow();
    let last = seen.composed.last().expect("a composed publication");
    let want: HashMap<_, _> = full.tree.nodes.iter().cloned().collect();
    assert_eq!(last.nodes.len(), want.len());
    for (id, node) in &want {
        assert_eq!(last.nodes.get(id), Some(node), "node {id:?}");
    }
    assert_eq!(last.focus, full.tree.focus);
    assert_eq!(last.targets, full.dom_targets);
    last.changes.clone()
}

#[test]
fn a_grid_scroll_publishes_a_patch_equal_to_the_whole_tree() {
    let seen = Rc::new(RefCell::new(Seen::default()));
    let mut host = grid_host(&seen);
    assert_eq!(frame(&mut host, &seen), None, "the first tree is whole");
    let mut patched = 0;
    for step in 1..=12 {
        let scroll = step as f32 * 24.0;
        host.s
            .runner
            .as_mut()
            .unwrap()
            .update(move |g| g.scroll = scroll);
        if let Some(changes) = frame(&mut host, &seen) {
            patched += 1;
            assert!(
                changes.added.len() + changes.changed.len() < 40,
                "{changes:?}"
            );
        }
    }
    assert_eq!(seen.borrow().whole_syncs, 0);
    assert!(patched >= 10, "{patched} of 12 scroll steps patched");
}

#[test]
fn a_leaf_or_producer_publishes_the_whole_tree() {
    let seen = Rc::new(RefCell::new(Seen::default()));
    let mut host = grid_host(&seen);
    frame(&mut host, &seen);
    host.s
        .shared
        .leaves
        .insert(9, Box::new(super::republish_tests::Flag));
    host.relayout(400.0, 300.0);
    host.sync_a11y();
    assert_eq!(
        seen.borrow().whole_syncs,
        1,
        "a leaf reads through callbacks"
    );
    assert!(host.s.a11y_retained.is_none(), "the session is dropped");
    host.s.shared.leaves.remove(&9);
    host.s
        .shared
        .producers
        .register(9, super::republish_tests::Blank, &[])
        .unwrap();
    host.relayout(400.0, 300.0);
    host.sync_a11y();
    assert_eq!(seen.borrow().whole_syncs, 2, "so does a producer");
    host.s.shared.producers.remove(9);
    host.s.runner.as_mut().unwrap().update(|g| g.scroll = 24.0);
    assert_eq!(frame(&mut host, &seen), None, "a new session starts whole");
}
