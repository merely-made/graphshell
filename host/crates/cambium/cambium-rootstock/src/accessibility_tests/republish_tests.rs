// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The tree is republished when something its projection reads moved, and
//! not otherwise (table plan D33).

use super::*;
use crate::{
    A11yAdapterState as Adapter, A11yPublication as Publication, A11yPublicationOutcome as Outcome,
    A11yPublicationRefusal as Refusal,
};
use std::cell::Cell;

/// A publisher that drains on its own and reports what each sync projected.
struct Counting {
    syncs: Rc<Cell<usize>>,
    drains: Rc<Cell<usize>>,
    state: Rc<Cell<Adapter>>,
    outcome: Rc<Cell<Outcome>>,
    /// Every label, value and role of the last projected tree.
    texts: Rc<RefCell<Vec<String>>>,
}

impl Accessibility for Counting {
    fn adapter_state(&self) -> Adapter {
        self.state.get()
    }

    fn drain_actions(&mut self, _: &crate::A11yOwner) -> Option<Vec<A11yRequest>> {
        self.drains.set(self.drains.get() + 1);
        Some(Vec::new())
    }

    fn sync(
        &mut self,
        _: &crate::A11yOwner,
        dom: &WindowDom<'_>,
        layout: &OwnedLayout,
        leaves: &sprigging::LeafRegistry<u64>,
        producers: &ProducerRegistry,
        focus: Option<u64>,
        _: f64,
    ) -> crate::A11ySync {
        self.syncs.set(self.syncs.get() + 1);
        let projection = crate::project_semantics(dom, layout, leaves, producers, focus);
        *self.texts.borrow_mut() = projection
            .tree
            .nodes
            .iter()
            .flat_map(|(_, node)| {
                [
                    Some(format!("{:?}", node.role())),
                    node.label().map(str::to_owned),
                    node.value().map(str::to_owned),
                ]
            })
            .flatten()
            .collect();
        crate::A11ySync {
            requests: Vec::new(),
            publication: Publication {
                adapter: self.state.get(),
                outcome: self.outcome.get(),
            },
        }
    }
}

struct Probe {
    syncs: Rc<Cell<usize>>,
    drains: Rc<Cell<usize>>,
    state: Rc<Cell<Adapter>>,
    outcome: Rc<Cell<Outcome>>,
    texts: Rc<RefCell<Vec<String>>>,
}

impl Probe {
    fn new() -> Self {
        Self {
            syncs: Rc::new(Cell::new(0)),
            drains: Rc::new(Cell::new(0)),
            state: Rc::new(Cell::new(Adapter::Installed)),
            outcome: Rc::new(Cell::new(Outcome::Submitted {
                nodes: 1,
                contribution_issues: 0,
            })),
            texts: Rc::new(RefCell::new(Vec::new())),
        }
    }

    fn adapter(&self) -> Box<dyn Accessibility> {
        Box::new(Counting {
            syncs: self.syncs.clone(),
            drains: self.drains.clone(),
            state: self.state.clone(),
            outcome: self.outcome.clone(),
            texts: self.texts.clone(),
        })
    }

    fn has(&self, text: &str) -> bool {
        self.texts.borrow().iter().any(|t| t == text)
    }
}

struct Rows {
    first: usize,
    count: usize,
}

type RowsChild = Box<dyn AnyView<Rows, (), GenetCtx, GenetElement>>;
type RowsLogic = fn(&Rows) -> RowsChild;

fn rows_view(state: &Rows) -> RowsChild {
    let rows: Vec<_> = (state.first..state.first + state.count)
        .map(|i| button(format!("row {i}"), |_: &mut Rows, _| {}))
        .collect();
    Box::new(el("main", rows))
}

type RowsHost = Host<Rows, RowsLogic, RowsChild>;

fn rows_host(probe: &Probe, count: usize) -> RowsHost {
    let mut state: HostState<Rows, RowsLogic, RowsChild> = HostState::new();
    state.shared.sheet =
        "main { display: block; } button { display: block; width: 100px; height: 30px; }".into();
    state.runner = Some(Runner::new(
        Rc::new(RefCell::new(ScriptedDom::new())),
        rows_view as RowsLogic,
        Rows { first: 0, count },
    ));
    state.a11y = Some(probe.adapter());
    let wake = HostWake::new(state.wake_pending.clone(), Arc::new(|| {}));
    let mut host = Host::new(
        HostOptions::default(),
        None,
        HostHooks::inert(),
        state,
        wake,
    );
    host.relayout(320.0, 200.0);
    host
}

/// Sync once and report whether the adapter was asked to publish.
fn published(host: &mut RowsHost, probe: &Probe) -> bool {
    let before = probe.syncs.get();
    host.sync_a11y();
    probe.syncs.get() > before
}

#[test]
fn an_unchanged_frame_drains_without_republishing() {
    let probe = Probe::new();
    let mut host = rows_host(&probe, 3);
    assert!(published(&mut host, &probe), "the first sync publishes");
    let observed = host.a11y_publication();
    let drains = probe.drains.get();
    for _ in 0..3 {
        host.relayout(320.0, 200.0);
        assert!(!published(&mut host, &probe), "nothing moved");
    }
    assert_eq!(probe.drains.get(), drains + 3, "actions are still drained");
    assert_eq!(
        host.a11y_publication(),
        observed,
        "the submitted tree stays the latest observation"
    );
}

#[test]
fn each_input_the_projection_reads_republishes() {
    let probe = Probe::new();
    let mut host = rows_host(&probe, 12);
    assert!(published(&mut host, &probe));
    assert!(!published(&mut host, &probe));

    // A DOM change, before and after layout has drained it.
    host.s
        .runner
        .as_mut()
        .unwrap()
        .update(|rows| rows.first = 1);
    assert!(published(&mut host, &probe), "a pending DOM mutation");
    assert!(probe.has("row 12"), "the new row is read");
    host.relayout(320.0, 200.0);
    assert!(published(&mut host, &probe), "the layout that drained it");
    assert!(!published(&mut host, &probe));

    // Layout alone: a new viewport.
    host.relayout(300.0, 200.0);
    assert!(published(&mut host, &probe), "a relayout");
    assert!(!published(&mut host, &probe));

    // Scroll alone moves every box without a relayout.
    host.s
        .layout
        .as_mut()
        .unwrap()
        .set_viewport_scroll((0.0, 40.0));
    assert!(published(&mut host, &probe), "a document scroll");
    assert!(!published(&mut host, &probe));

    // Focus.
    let target = host.s.runner.as_ref().unwrap().focusables()[2];
    host.s.runner.as_mut().unwrap().set_focus(Some(target));
    assert!(published(&mut host, &probe), "a focus move");
    assert!(!published(&mut host, &probe));

    // The window scale the boxes ride into physical pixels.
    host.s.user_zoom = 1.5;
    assert!(published(&mut host, &probe), "a scale change");
    assert!(!published(&mut host, &probe));

    // The adapter's own attachment.
    probe.state.set(Adapter::Detached);
    assert!(published(&mut host, &probe), "an adapter state change");
    assert!(!published(&mut host, &probe));

    // A publication that did not reach the platform is retried.
    probe.outcome.set(Outcome::Refused {
        reason: Refusal::InvalidTree,
    });
    assert!(!published(&mut host, &probe), "the outcome is not an input");
    host.s.runner.as_mut().unwrap().set_focus(None);
    assert!(published(&mut host, &probe), "refused");
    assert!(published(&mut host, &probe), "a refusal is retried");
    probe.outcome.set(Outcome::Submitted {
        nodes: 1,
        contribution_issues: 0,
    });
    assert!(published(&mut host, &probe));
    assert!(!published(&mut host, &probe));

    // A replaced adapter has seen nothing.
    host.s.a11y = Some(probe.adapter());
    assert!(published(&mut host, &probe), "a new adapter");
    assert!(!published(&mut host, &probe));
}

struct Flag;
impl sprigging::Leaf for Flag {
    fn measure(&mut self, _: sprigging::SizeHint, _: sprigging::SizeHint) -> sprigging::Size {
        sprigging::Size {
            width: 1.0,
            height: 1.0,
        }
    }
    fn paint(&mut self, _: &mut sprigging::PaintCx<'_>) {}
    fn paint_dirty(&self) -> bool {
        false
    }
}

struct Blank;
impl crate::TextureProducer for Blank {
    fn render(&mut self, _: &crate::ProducerContext<'_>) -> Option<crate::ProducedTexture> {
        None
    }
}

/// Contributors describe themselves through callbacks with no change signal,
/// so while any is registered every frame is read again.
#[test]
fn registered_contributors_are_read_every_frame() {
    let probe = Probe::new();
    let mut host = rows_host(&probe, 3);
    assert!(published(&mut host, &probe));
    assert!(!published(&mut host, &probe));
    host.s.shared.leaves.insert(9, Box::new(Flag));
    assert!(published(&mut host, &probe));
    assert!(published(&mut host, &probe), "a leaf may change unseen");
    host.s.shared.leaves.remove(&9);
    assert!(published(&mut host, &probe));
    assert!(!published(&mut host, &probe));
    host.s.shared.producers.register(9, Blank, &[]).unwrap();
    assert!(published(&mut host, &probe));
    assert!(published(&mut host, &probe), "a producer may change unseen");
}

struct Grid {
    scroll: f32,
    descending: bool,
}

type GridChild = Box<dyn AnyView<Grid, (), GenetCtx, GenetElement>>;
type GridLogic = fn(&Grid) -> GridChild;

const GRID_ROWS: usize = 1000;

fn grid_view(state: &Grid) -> GridChild {
    let descending = state.descending;
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
        GRID_ROWS,
        240.0,
        state.scroll,
        move |r, c| {
            let i = if descending { GRID_ROWS - 1 - r } else { r };
            Box::new(el::<_, Grid, ()>("span", format!("r{i}c{c}")))
        },
        |s: &mut Grid, _| s.descending = !s.descending,
        |_| None,
    )
}

/// The published tree follows a window shift and a sort: what a reader hears
/// after each is the rows on screen, not the rows before.
#[test]
fn the_grid_tree_is_current_after_a_scroll_and_a_sort() {
    let probe = Probe::new();
    let mut state: HostState<Grid, GridLogic, GridChild> = HostState::new();
    state.shared.sheet = "span { display: block; }".into();
    state.runner = Some(Runner::new(
        Rc::new(RefCell::new(ScriptedDom::new())),
        grid_view as GridLogic,
        Grid {
            scroll: 0.0,
            descending: false,
        },
    ));
    state.a11y = Some(probe.adapter());
    let wake = HostWake::new(state.wake_pending.clone(), Arc::new(|| {}));
    let mut host = Host::new(
        HostOptions::default(),
        None,
        HostHooks::inert(),
        state,
        wake,
    );
    let frame = |host: &mut Host<Grid, GridLogic, GridChild>| {
        host.relayout(400.0, 300.0);
        let before = probe.syncs.get();
        host.sync_a11y();
        probe.syncs.get() > before
    };
    assert!(frame(&mut host));
    // Genet lowers `row` and `gridcell`; `grid` and `columnheader` are a gap
    // in its role vocabulary (table plan findings, T4).
    assert!(probe.has("Row") && probe.has("Cell") && probe.has("Name"));
    assert!(probe.has("r0c0") && probe.has("r5c1"), "{:?}", probe.texts);
    assert!(!frame(&mut host), "an unchanged grid is not republished");

    // A strip move inside the window: geometry only.
    host.s.runner.as_mut().unwrap().update(|g| g.scroll = 10.0);
    assert!(frame(&mut host), "the strip moved");
    assert!(probe.has("r0c0"));

    // A window shift: row 0 leaves, row 20 arrives.
    host.s.runner.as_mut().unwrap().update(|g| g.scroll = 240.0);
    assert!(frame(&mut host), "the window shifted");
    assert!(!probe.has("r0c0") && probe.has("r10c0") && probe.has("r20c1"));
    assert!(!frame(&mut host));

    // A sort: the same window shows the mirrored rows.
    host.s
        .runner
        .as_mut()
        .unwrap()
        .update(|g| g.descending = true);
    assert!(frame(&mut host), "the sort");
    assert!(
        !probe.has("r10c0") && probe.has("r989c0"),
        "{:?}",
        probe.texts
    );
    assert!(probe.has("Row") && probe.has("Cell") && probe.has("r989c0r989c1"));
}
