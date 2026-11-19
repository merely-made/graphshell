// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! A table recipe in a desktop window (table projection plan, T4), and the
//! receipt for its speed gate (table D5, D8, D10).
//!
//! ```text
//! TABLE_SCENARIO=crates/cambium/cambium-genet-winit-host/examples/table.scn \
//! TABLE_CAPTURE_DIR=<dir> TABLE_ROWS=10000 \
//!   cargo run --release -p cambium-desktop --example table
//! ```
//!
//! The scenario clicks a header and turns the wheel through the host's own
//! routing, then times two things: `autoscroll <px> <frames>` moves the body
//! by `px` at the top of each frame and records each frame's UI time, and
//! `sort-next <column>` sorts at the top of one frame and records the time
//! until that frame is drawn. `TABLE_STALL_MS` adds a sleep to every timed
//! frame: the positive control that shows the gate can fail.

#[path = "../../cambium/examples/support/table_fixture.rs"]
mod table_fixture;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use cambium::projection_table::{ProjectionTable, projection_table};
use cambium::{AnyView, GenetCtx, GenetElement, el, lens};
use cambium_genet_winit_host::{AppCtx, HostHooks, HostOptions, Init, Runner, run};
use mandrake::{CaptureRecord, LaneConfig};
use taproot::ProbeSnapshot;

/// One 60 Hz frame (table D8).
const FRAME_BUDGET_US: u64 = 16_700;
/// A sort or filter shows its result within this (table D8).
const SORT_BUDGET_US: u64 = 100_000;

struct TableApp {
    table: ProjectionTable,
    /// Pixels per frame and frames left.
    autoscroll: Option<(f32, u32)>,
    /// A column to sort at the top of the next frame.
    pending_sort: Option<usize>,
    /// When the frame carrying a sort began.
    sort_started: Option<Instant>,
    /// Frames left to draw with nothing changed: the control.
    idle: u32,
    stall: Duration,
    events: Vec<String>,
}

type Child = Box<dyn AnyView<TableApp, (), GenetCtx, GenetElement>>;
type Logic = fn(&TableApp) -> Child;

fn root(_: &TableApp) -> Child {
    Box::new(
        el(
            "div",
            lens(
                |table: &mut ProjectionTable| projection_table::<()>(table),
                |app: &mut TableApp| &mut app.table,
            ),
        )
        .attr("class", "app"),
    )
}

/// The p95 by nearest rank, as mandrake's cost receipt takes it.
fn p95(values: &[u64]) -> u64 {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    sorted
        .get((sorted.len() * 95).div_ceil(100).saturating_sub(1))
        .copied()
        .unwrap_or(0)
}

#[derive(Default)]
struct Timings {
    /// UI time of each autoscrolled frame, in µs.
    scroll_frames: Vec<u64>,
    /// Sort start to its frame drawn, in µs.
    sorts: Vec<u64>,
    /// UI time of frames drawn with nothing changed, in µs.
    idle_frames: Vec<u64>,
}

struct TableLane {
    timings: Rc<RefCell<Timings>>,
    rows: usize,
    sheet: String,
}

impl mandrake::Product for TableLane {
    type State = TableApp;
    type Logic = Logic;
    type View = Child;
    const KIND: &'static str = "table";
    const SURFACE: &'static str = "app";
    const LOG_PREFIX: &'static str = "table";

    fn sheet(&self) -> &str {
        &self.sheet
    }

    fn snapshot(
        &self,
        ctx: &AppCtx<'_, TableApp, Logic, Child>,
        _: usize,
        _: f32,
    ) -> ProbeSnapshot {
        let app = ctx.runner.state();
        let table = &app.table;
        let sort = table
            .draft()
            .reading
            .sort
            .iter()
            .map(|key| format!("{}{}", key.field, if key.descending { " desc" } else { "" }))
            .collect::<Vec<_>>()
            .join(",");
        let focus = table.focus();
        ProbeSnapshot::default()
            .with_field("rows", table.row_count().to_string())
            .with_field("sort", sort)
            .with_field("undo", table.editor().can_undo().to_string())
            .with_field("scroll", (table.scroll.round() as i64).to_string())
            .with_field("first", table.row_id(0).unwrap_or_default())
            .with_field(
                "focus",
                format!(
                    "{}:{}",
                    focus.row.map_or("header".into(), |r| r.to_string()),
                    focus.col
                ),
            )
            .with_field("autoscroll", app.autoscroll.is_some().to_string())
    }

    fn drain_events(&mut self, ctx: &mut AppCtx<'_, TableApp, Logic, Child>) -> Vec<String> {
        let mut drained = Vec::new();
        ctx.runner
            .update(|app| drained = std::mem::take(&mut app.events));
        drained
    }

    fn app_step(
        &mut self,
        ctx: &mut AppCtx<'_, TableApp, Logic, Child>,
        _checkpoints: mandrake::Checkpoints<'_>,
        line: &str,
    ) -> Result<(), String> {
        let words: Vec<_> = line.split_whitespace().collect();
        match words.as_slice() {
            ["autoscroll", px, frames] => {
                let px: f32 = px.parse().map_err(|_| format!("bad px {px}"))?;
                let frames: u32 = frames.parse().map_err(|_| format!("bad frames {frames}"))?;
                ctx.runner.update(|app| app.autoscroll = Some((px, frames)));
                Ok(())
            },
            ["sort-next", column] => {
                let column: usize = column.parse().map_err(|_| format!("bad column {column}"))?;
                ctx.runner.update(|app| app.pending_sort = Some(column));
                Ok(())
            },
            ["idle", frames] => {
                let frames: u32 = frames.parse().map_err(|_| format!("bad frames {frames}"))?;
                ctx.runner.update(|app| app.idle = frames);
                Ok(())
            },
            ["undo"] => {
                ctx.runner.update(|app| {
                    let undone = app.table.undo();
                    app.events.push(format!("undo {undone}"));
                });
                Ok(())
            },
            _ => Err(format!("unknown verb: {line}")),
        }
    }

    fn busy_mut(
        &mut self,
        ctx: &mut AppCtx<'_, TableApp, Logic, Child>,
        _capture_pending: bool,
    ) -> Option<bool> {
        let app = ctx.runner.state();
        Some(
            app.autoscroll.is_some()
                || app.idle > 0
                || app.pending_sort.is_some()
                || app.sort_started.is_some(),
        )
    }

    fn receipt_lines(&self) -> Vec<String> {
        let timings = self.timings.borrow();
        let max = |v: &[u64]| v.iter().copied().max().unwrap_or(0);
        vec![
            format!(
                "rows {} scroll frames={} p95_us={} max_us={}",
                self.rows,
                timings.scroll_frames.len(),
                p95(&timings.scroll_frames),
                max(&timings.scroll_frames)
            ),
            format!(
                "rows {} idle frames={} p95_us={} max_us={}",
                self.rows,
                timings.idle_frames.len(),
                p95(&timings.idle_frames),
                max(&timings.idle_frames)
            ),
            format!(
                "rows {} sorts={} p95_us={} max_us={} each_us={:?}",
                self.rows,
                timings.sorts.len(),
                p95(&timings.sorts),
                max(&timings.sorts),
                timings.sorts
            ),
        ]
    }

    /// The gate (table D8): scrolling holds 16.7 ms at the 95th percentile
    /// and a sort shows within 100 ms. Gated only where `TABLE_GATE=1`.
    fn receipt_checks(&self, captures: &[CaptureRecord]) -> Vec<String> {
        let timings = self.timings.borrow();
        let mut failures = Vec::new();
        if timings.scroll_frames.len() < 60 {
            failures.push(format!(
                "only {} scroll frames timed",
                timings.scroll_frames.len()
            ));
        }
        if timings.sorts.is_empty() {
            failures.push("no sort timed".into());
        }
        if captures
            .iter()
            .map(|c| c.digest)
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            < 2
        {
            failures.push("frames must differ across a sort and a scroll".into());
        }
        if std::env::var("TABLE_GATE").is_ok_and(|v| v == "1") {
            let scroll = p95(&timings.scroll_frames);
            if scroll > FRAME_BUDGET_US {
                failures.push(format!(
                    "scroll p95 {scroll} us is over {FRAME_BUDGET_US} us"
                ));
            }
            let sort = p95(&timings.sorts);
            if sort > SORT_BUDGET_US {
                failures.push(format!("sort p95 {sort} us is over {SORT_BUDGET_US} us"));
            }
        }
        failures
    }
}

fn main() {
    let rows: usize = std::env::var("TABLE_ROWS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10_000);
    let stall = Duration::from_millis(
        std::env::var("TABLE_STALL_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0),
    );
    let timings = Rc::new(RefCell::new(Timings::default()));
    let lane = LaneConfig::from_env("TABLE").map(|config| {
        mandrake::Lane::from_config(
            config,
            TableLane {
                timings: timings.clone(),
                rows,
                sheet: table_fixture::sheet(),
            },
            cambium_genet_winit_host::read_file,
        )
        .unwrap_or_else(|error| panic!("{error}"))
    });
    let lane = Rc::new(RefCell::new(lane));
    let after_frame_lane = lane.clone();
    let frame_timings = timings.clone();
    let opened = Instant::now();
    let body: f32 = std::env::var("TABLE_BODY")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(480.0);
    let table = table_fixture::open(rows, body);
    eprintln!(
        "[table] opened {rows} rows in {} ms",
        opened.elapsed().as_millis()
    );

    let hooks: HostHooks<TableApp, Logic, Child> = HostHooks {
        prepare_presentation: Box::new(|_, _| None),
        // The timed work starts at the top of the frame.
        frame: Box::new(move |ctx: &mut AppCtx<'_, TableApp, Logic, Child>| {
            let mut live = false;
            ctx.runner.update(|app| {
                if let Some(column) = app.pending_sort.take() {
                    app.sort_started = Some(Instant::now());
                    app.table.click_header(column, 0);
                    std::thread::sleep(app.stall);
                    app.events.push(format!("sorted {column}"));
                }
                if app.idle > 0 {
                    app.idle -= 1;
                    std::thread::sleep(app.stall);
                    live = true;
                }
                if let Some((px, frames)) = app.autoscroll {
                    app.table.scroll_by(px);
                    std::thread::sleep(app.stall);
                    app.autoscroll = (frames > 1).then_some((px, frames - 1));
                    live = true;
                }
            });
            live
        }),
        after_dispatch: Box::new(|_ctx| {}),
        after_frame: Box::new(move |ctx: &mut AppCtx<'_, TableApp, Logic, Child>| {
            // A frame that scrolled or sorted is timed once it is drawn.
            let scrolling = ctx.runner.state().autoscroll.is_some();
            let idling = ctx.runner.state().idle > 0;
            let total = ctx.frame_profile.map(|profile| profile.total_us);
            if ctx.presentation.is_some() {
                let mut timings = frame_timings.borrow_mut();
                if scrolling && let Some(total) = total {
                    timings.scroll_frames.push(total);
                    if timings.scroll_frames.len() % 100 == 50 {
                        if let Some(profile) = ctx.frame_profile {
                            eprintln!(
                                "[table] scroll frame: total {} us, style {} us, layout {} us, a11y {} us, emit {} us, raster {} us, mutations {}",
                                profile.total_us,
                                profile.style_resolve_us,
                                profile.layout_with_text_us,
                                profile.a11y_publish_us,
                                profile.emit_scene_us,
                                profile.raster_us,
                                profile.layout_mutations
                            );
                        }
                    }
                }
                if idling && let Some(total) = total {
                    timings.idle_frames.push(total);
                }
                let started = ctx.runner.state().sort_started;
                if let Some(started) = started {
                    timings.sorts.push(started.elapsed().as_micros() as u64);
                    ctx.runner.update(|app| app.sort_started = None);
                }
            }
            if let Some(lane) = after_frame_lane.borrow_mut().as_mut() {
                lane.after_frame(ctx);
            }
        }),
        after_wake: Box::new(|_ctx| {}),
        close_request: Box::new(|_ctx, _request| cambium_genet_winit_host::CloseDisposition::Exit),
        focused_text: Box::new(|_runner: &Runner<TableApp, Logic, Child>| None),
        key_intercept: Box::new(|_runner, _press| false),
    };
    let options = HostOptions {
        title: "Graphshell table".into(),
        initial_logical_size: (960.0, 640.0),
        size_env: Some(("TABLE_WIDTH".into(), "TABLE_HEIGHT".into())),
        ..Default::default()
    };
    run(
        options,
        move |_window, _commands, _wake| Init {
            state: TableApp {
                table,
                autoscroll: None,
                pending_sort: None,
                sort_started: None,
                idle: 0,
                stall,
                events: Vec::new(),
            },
            logic: root as Logic,
            sheet: table_fixture::sheet(),
            fonts: Vec::new(),
            images: Vec::new(),
        },
        hooks,
    )
    .expect("event loop");
}
