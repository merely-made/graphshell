// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! `right_click_on` refuses a covered target the way `click_on` does (see
//! `click_on_covered.rs`): it hit-tests the centre it is about to press and
//! panics when the topmost node there is neither the target nor inside it.
//!
//! The overlay is a state switch, so one target is pressed uncovered and then
//! refused covered.

use std::panic::{AssertUnwindSafe, catch_unwind};

use cambium::{AnyView, GenetCtx, GenetElement, PointerButton, PointerEvent, el, on_pointer};
use cambium_genet_winit_host::{Harness, Init, inert_hooks};
use taproot::Selector;

struct App {
    presses: Vec<PointerButton>,
    overlay: bool,
}

type Child = Box<dyn AnyView<App, (), GenetCtx, GenetElement>>;
type Logic = fn(&App) -> Child;

const SHEET: &str = "body { margin:0; } \
     .menu { display:block; width:120px; height:40px; } \
     .menu .inner { display:block; width:120px; height:40px; } \
     #overlay { position:absolute; top:0; left:0; width:300px; height:120px; } \
     #overlay.off { display:none; }";

fn record(app: &mut App, event: PointerEvent) {
    app.presses.push(event.button);
}

fn root(app: &App) -> Child {
    Box::new(el(
        "div",
        (
            on_pointer(el("div", "Plain").attr("class", "menu plain"), record),
            on_pointer(
                el("div", el("span", "Wrapped").attr("class", "inner"))
                    .attr("class", "menu wrapped"),
                record,
            ),
            el("div", "Search results")
                .attr("id", "overlay")
                .attr("class", if app.overlay { "on" } else { "off" }),
        ),
    ))
}

fn harness() -> Harness<App, Logic, Child> {
    let mut host = Harness::with_hooks(
        Init {
            state: App {
                presses: Vec::new(),
                overlay: false,
            },
            logic: root as Logic,
            sheet: SHEET.to_string(),
            fonts: Vec::new(),
            images: Vec::new(),
        },
        inert_hooks(),
    );
    host.layout_at(400.0, 300.0);
    host
}

/// The panic message `right_click_on` raised, or `None` when it returned.
fn refusal(host: &mut Harness<App, Logic, Child>, selector: &Selector) -> Option<String> {
    let payload = catch_unwind(AssertUnwindSafe(|| host.right_click_on(selector))).err()?;
    Some(
        payload
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_default(),
    )
}

#[test]
fn an_uncovered_target_still_right_clicks() {
    let mut host = harness();
    assert!(host.right_click_on(&Selector::class("plain")));
    assert_eq!(host.state().presses, vec![PointerButton::Secondary]);
}

#[test]
fn a_covered_target_is_refused_naming_both_elements_and_the_point() {
    let mut host = harness();
    // Positive control: the same target takes the press while the overlay is off.
    assert!(host.right_click_on(&Selector::class("plain")));
    assert_eq!(host.state().presses.len(), 1);

    host.update(|app| app.overlay = true);
    host.relayout();
    let (x, y) = host
        .resolve(&Selector::class("plain"))
        .expect("the target still resolves");

    let message =
        refusal(&mut host, &Selector::class("plain")).expect("a covered press is refused");
    assert!(message.starts_with("right_click_on("), "{message}");
    assert!(message.contains("<div class=\"menu plain\">"), "{message}");
    assert!(
        message.contains("<div id=\"overlay\" class=\"on\">"),
        "{message}"
    );
    assert!(message.contains(&format!("({x:.1}, {y:.1})")), "{message}");
    assert_eq!(
        host.state().presses.len(),
        1,
        "the refused press was not delivered"
    );
}

#[test]
fn a_hit_on_a_child_of_the_target_counts_as_the_target() {
    let mut host = harness();
    assert!(host.right_click_on(&Selector::class("wrapped")));
    assert_eq!(host.state().presses, vec![PointerButton::Secondary]);
}

#[test]
fn a_missing_target_is_still_false_not_a_refusal() {
    let mut host = harness();
    host.update(|app| app.overlay = true);
    host.relayout();
    assert_eq!(refusal(&mut host, &Selector::class("absent")), None);
    assert!(!host.right_click_on(&Selector::class("absent")));
}
