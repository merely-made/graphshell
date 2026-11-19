// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! A table recipe in a browser (table projection plan, T4): the desktop
//! table example's recipe and data, mounted on a canvas, driving itself.
//!
//! ```text
//! RUSTFLAGS='--cfg getrandom_backend="wasm_js"' cargo build --release \
//!     -p cambium-web --example table_page --features webgl \
//!     --target wasm32-unknown-unknown
//! wasm-bindgen --target web --no-typescript --out-dir <dir> \
//!     target/wasm32-unknown-unknown/release/examples/table_page.wasm
//! ```
//!
//! Serve `<dir>` with this directory's `index.html`. `?rows=` sets the row
//! count. Once mounted the page sorts by Pages, flips it, moves the keyboard
//! focus into the body and scrolls, each through the table's own calls at
//! the top of a frame, and [`report`] returns what it saw as JSON.
#![cfg(target_arch = "wasm32")]

#[path = "../../../cambium/examples/support/table_fixture.rs"]
mod table_fixture;

use std::cell::RefCell;

use cambium::projection_table::{ProjectionTable, projection_table};
use cambium::{AnyView, GenetCtx, GenetElement, Key, NamedKey, el, lens};
use cambium_genet_web_host::{mount, run_static_constructors_once};
use cambium_rootstock::{HostFont, HostHooks, HostOptions, Init, PresentationChoice};
use wasm_bindgen::prelude::*;

struct TableApp {
    table: ProjectionTable,
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

/// What the page did and saw, read back by [`report`].
#[derive(Default)]
struct Run {
    frame: u32,
    steps: Vec<String>,
    /// Frame-to-frame intervals while scrolling, in ms.
    scroll_intervals: Vec<f64>,
    last_frame_ms: Option<f64>,
    /// Sort start to the next frame, in ms.
    sorts: Vec<f64>,
    sort_started: Option<f64>,
    done: bool,
}

thread_local! { static RUN: RefCell<Run> = RefCell::new(Run::default()); }

fn now_ms() -> f64 {
    web_sys::window()
        .and_then(|window| window.performance())
        .map_or(0.0, |performance| performance.now())
}

/// One frame of the page's own script.
fn drive(app: &mut TableApp) -> bool {
    RUN.with(|run| {
        let mut run = run.borrow_mut();
        run.frame += 1;
        let now = now_ms();
        if let Some(started) = run.sort_started.take() {
            run.sorts.push(now - started);
        }
        let table = &mut app.table;
        match run.frame {
            30 | 60 => {
                run.sort_started = Some(now);
                table.click_header(1, 0);
                let sort = table.draft().reading.sort.first().cloned();
                run.steps.push(format!("sorted {sort:?}"));
            },
            90 => {
                for key in [NamedKey::ArrowDown, NamedKey::ArrowRight, NamedKey::ArrowRight] {
                    table.navigate(&Key::Named(key), false);
                }
                run.steps.push(format!("focus {:?}", table.focus()));
            },
            120..=300 => {
                if let Some(last) = run.last_frame_ms {
                    run.scroll_intervals.push(now - last);
                }
                table.scroll_by(6.0);
            },
            301 => {
                run.steps.push(format!("scrolled to {}", table.scroll));
                run.done = true;
            },
            _ => {},
        }
        run.last_frame_ms = Some(now);
        !run.done
    })
}

/// The run so far as JSON.
#[wasm_bindgen]
pub fn report() -> String {
    RUN.with(|run| {
        let run = run.borrow();
        let p95 = |values: &[f64]| {
            let mut sorted = values.to_vec();
            sorted.sort_by(f64::total_cmp);
            sorted
                .get((sorted.len() * 95).div_ceil(100).saturating_sub(1))
                .copied()
                .unwrap_or(0.0)
        };
        serde_json::json!({
            "done": run.done,
            "frames": run.frame,
            "steps": run.steps,
            "scroll_interval_p95_ms": p95(&run.scroll_intervals),
            "scroll_intervals": run.scroll_intervals.len(),
            "sorts_ms": run.sorts,
            "user_agent": web_sys::window().and_then(|w| w.navigator().user_agent().ok()),
        })
        .to_string()
    })
}

#[wasm_bindgen(start)]
pub fn start() {
    run_static_constructors_once();
    console_error_panic_hook::set_once();
}

/// Mount the table on the canvas `canvas_id` with `rows` books.
#[wasm_bindgen]
pub async fn mount_table(canvas_id: String, rows: u32) -> Result<(), JsValue> {
    let canvas = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.get_element_by_id(&canvas_id))
        .ok_or_else(|| JsValue::from_str("missing canvas"))?
        .dyn_into::<web_sys::HtmlCanvasElement>()?;
    let options = HostOptions {
        title: "Graphshell table".into(),
        presentation_choice: PresentationChoice::Auto,
        ..Default::default()
    };
    let table = table_fixture::open(rows as usize, 420.0);
    let mut hooks: HostHooks<TableApp, Logic, Child> = HostHooks::inert();
    hooks.frame = Box::new(|ctx| {
        let mut live = false;
        ctx.runner.update(|app: &mut TableApp| live = drive(app));
        live
    });
    let mounted = mount(
        canvas,
        options,
        move |_, _, _| Init {
            state: TableApp { table },
            logic: root as Logic,
            sheet: table_fixture::sheet(),
            // A browser lends genet no system faces, so the page brings one.
            fonts: vec![HostFont {
                family: None,
                bytes: include_bytes!("../../../../../testing/assets/fonts/Roboto-Regular.ttf")
                    .to_vec(),
            }],
            images: Vec::new(),
        },
        hooks,
    )
    .await
    .map_err(|error| JsValue::from_str(&error))?;
    // The page lives as long as the document.
    std::mem::forget(mounted);
    Ok(())
}
