// Copyright 2026 Mark Alan Boykin
// SPDX-License-Identifier: MPL-2.0

use std::{cell::RefCell, rc::Rc};

use genet_scripted_dom::{NodeId, ScriptedDom};
use layout_dom_api::{LayoutDom, LocalName, Namespace};

use super::*;
use crate::{DomHandle, GenetAppRunner, Key, KeyEvent, NamedKey, PointerClick};

struct State {
    evidence: Vec<ValidationEvidence<u32>>,
    selected: Option<u32>,
}

type TestView = Box<dyn AnyView<State, (), GenetCtx, GenetElement>>;

fn view(state: &State) -> TestView {
    let finding = ValidationFinding {
        focus_node: "https://example.test/place".into(),
        path: Some("https://example.test/name".into()),
        severity: "Violation".into(),
        message: None,
    };
    Box::new(validation_finding_panel(
        &finding,
        &state.evidence,
        state.selected.as_ref(),
        |state: &mut State, id| state.selected = Some(id),
    ))
}

fn fixture(titles: &[(u32, &str)]) -> (DomHandle, State) {
    let evidence = titles
        .iter()
        .map(|(id, title)| ValidationEvidence {
            id: *id,
            detail: DetailSection::new(*title, vec![DetailRow::new("Source", "Saved page")]),
        })
        .collect();
    (
        Rc::new(RefCell::new(ScriptedDom::new())),
        State {
            evidence,
            selected: None,
        },
    )
}

fn nodes(dom: &DomHandle, class: &str) -> Vec<NodeId> {
    let dom = dom.borrow();
    dom.all_with_class(dom.document(), class)
}

fn values(dom: &DomHandle) -> Vec<String> {
    let dom = dom.borrow();
    dom.all_with_class(dom.document(), "detail-value")
        .into_iter()
        .flat_map(|node| dom.dom_children(node))
        .filter_map(|child| dom.text(child).map(str::to_string))
        .collect()
}

#[test]
fn finding_opens_its_only_evidence_via_keyboard() {
    let (dom, state) = fixture(&[(42, "Claim in block 2")]);
    let mut runner = GenetAppRunner::new(dom.clone(), view as fn(&State) -> TestView, state);
    assert_eq!(
        values(&dom),
        [
            "https://example.test/place",
            "https://example.test/name",
            "Violation",
            "Saved page",
        ]
    );
    let buttons = nodes(&dom, "validation-finding-open");
    assert_eq!(buttons.len(), 1);
    runner.set_focus(Some(buttons[0]));
    runner.dispatch_key(KeyEvent::new(Key::Named(NamedKey::Enter)));
    assert_eq!(runner.state().selected, Some(42));
}

#[test]
fn multiple_evidence_actions_select_the_exact_claim() {
    let (dom, state) = fixture(&[(42, "Claim in block 2"), (17, "Claim in block 5")]);
    let mut runner = GenetAppRunner::new(dom.clone(), view as fn(&State) -> TestView, state);
    let buttons = nodes(&dom, "validation-finding-open");
    assert_eq!(buttons.len(), 2);
    runner.dispatch_click(buttons[1], PointerClick::at((1.0, 1.0)));
    assert_eq!(runner.state().selected, Some(17));
    let sections = nodes(&dom, "validation-finding-evidence");
    let dom = dom.borrow();
    let current =
        |node| dom.attribute(node, &Namespace::from(""), &LocalName::from("aria-current"));
    assert_eq!(current(sections[0]), Some("false"));
    assert_eq!(current(sections[1]), Some("true"));
}

#[test]
fn unmatched_finding_keeps_its_facts_and_offers_no_open() {
    let (dom, state) = fixture(&[]);
    let _runner = GenetAppRunner::new(dom.clone(), view as fn(&State) -> TestView, state);
    assert_eq!(values(&dom).len(), 3);
    assert!(nodes(&dom, "validation-finding-open").is_empty());
    assert!(nodes(&dom, "validation-finding-evidence").is_empty());
}
