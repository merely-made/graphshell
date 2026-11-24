// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The harness refuses to click a covered target. `click_on` hit-tests the
//! target's centre through the hit test the click routes by, and panics when
//! the topmost node there is neither the target nor inside it, so a test fails
//! at the click instead of at a later assertion about what the click missed.
//!
//! One fixture serves both outcomes: the overlay is a state switch, so the
//! same button is clicked uncovered and then refused covered.

use std::panic::{AssertUnwindSafe, catch_unwind};

use cambium::{AnyView, GenetCtx, GenetElement, button, button_with, el};
use cambium_genet_winit_host::{Harness, Init, inert_hooks};
use genet_scripted_dom::{NodeId, ScriptedDom};
use layout_dom_api::{LayoutDom, LocalName, Namespace};
use taproot::Selector;

struct Clicks {
    count: u32,
    overlay: bool,
}

type Child = Box<dyn AnyView<Clicks, (), GenetCtx, GenetElement>>;
type Logic = fn(&Clicks) -> Child;

const SHEET: &str = "body { margin:0; } \
     button { display:block; width:120px; height:40px; padding:0; border:none; } \
     #inner { display:block; width:120px; height:40px; } \
     #overlay { position:absolute; top:0; left:0; width:300px; height:120px; } \
     #overlay.off { display:none; }";

fn root(clicks: &Clicks) -> Child {
    Box::new(el(
        "div",
        (
            button("Plain", |clicks: &mut Clicks, _| clicks.count += 1).attr("id", "plain"),
            button_with(
                el("span", "Wrapped").attr("id", "inner"),
                |clicks: &mut Clicks, _| clicks.count += 1,
            )
            .attr("id", "wrapped"),
            el("div", "Search results")
                .attr("id", "overlay")
                .attr("class", if clicks.overlay { "on" } else { "off" }),
        ),
    ))
}

fn harness() -> Harness<Clicks, Logic, Child> {
    let mut host = Harness::with_hooks(
        Init {
            state: Clicks {
                count: 0,
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

fn node(host: &Harness<Clicks, Logic, Child>, id: &str) -> NodeId {
    fn find(dom: &ScriptedDom, node: NodeId, id: &str) -> Option<NodeId> {
        if dom.attribute(node, &Namespace::from(""), &LocalName::from("id")) == Some(id) {
            return Some(node);
        }
        dom.dom_children(node)
            .find_map(|child| find(dom, child, id))
    }
    let dom = host.runner().dom();
    let dom = dom.borrow();
    find(&dom, dom.document(), id).expect("the element is in the DOM")
}

fn plain() -> Selector {
    Selector::role("button").containing("Plain")
}

/// The panic message `click_on` raised, or `None` when it returned.
fn refusal(host: &mut Harness<Clicks, Logic, Child>, selector: &Selector) -> Option<String> {
    let payload = catch_unwind(AssertUnwindSafe(|| host.click_on(selector))).err()?;
    Some(
        payload
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_default(),
    )
}

#[test]
fn an_uncovered_target_still_clicks() {
    let mut host = harness();
    assert!(host.click_on(&plain()));
    assert_eq!(host.state().count, 1);
    assert_eq!(host.hit(), Some(node(&host, "plain")));
}

#[test]
fn a_covered_target_is_refused_naming_both_elements_and_the_point() {
    let mut host = harness();
    // Positive control: the same button clicks while the overlay is off...
    assert!(host.click_on(&plain()));
    assert_eq!(host.state().count, 1);
    // ...and the overlay, once shown, really is what the point hits.
    host.update(|clicks| clicks.overlay = true);
    host.relayout();
    let (x, y) = host.resolve(&plain()).expect("the button still resolves");
    host.move_to(x, y);
    assert_eq!(host.hit(), Some(node(&host, "overlay")));

    let message = refusal(&mut host, &plain()).expect("a covered click is refused");
    assert!(message.contains("<button id=\"plain\">"), "{message}");
    assert!(
        message.contains("<div id=\"overlay\" class=\"on\">"),
        "{message}"
    );
    assert!(message.contains(&format!("({x:.1}, {y:.1})")), "{message}");
    assert_eq!(host.state().count, 1, "the refused click was not delivered");
}

#[test]
fn a_hit_on_a_child_of_the_target_counts_as_the_target() {
    let mut host = harness();
    let wrapped = Selector::role("button").containing("Wrapped");
    assert!(host.click_on(&wrapped));
    assert_eq!(host.state().count, 1);
    // The point really did land on the child, not on the button itself.
    assert_eq!(host.hit(), Some(node(&host, "inner")));
}

#[test]
fn a_missing_target_is_still_false_not_a_refusal() {
    let mut host = harness();
    host.update(|clicks| clicks.overlay = true);
    host.relayout();
    assert_eq!(
        refusal(&mut host, &Selector::role("button").containing("Absent")),
        None
    );
    assert!(!host.click_on(&Selector::role("button").containing("Absent")));
}
