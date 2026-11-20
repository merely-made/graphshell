// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

use std::cell::RefCell;
use std::rc::Rc;

use genet_scripted_dom::{NodeId, ScriptedDom};
use layout_dom_api::{LayoutDom, NodeKind};

use super::*;
use crate::{
    ComponentView, CompositionEvent, Key, PreparationPurpose, TextInput, text_field_typed,
};

fn attr(dom: &ScriptedDom, node: NodeId, name: &str) -> Option<String> {
    dom.attribute(node, &Namespace::from(""), &LocalName::from(name))
        .map(str::to_owned)
}

fn row_node(dom: &ScriptedDom, root: NodeId, id: &str) -> Option<NodeId> {
    if dom.kind(root) == NodeKind::Element && attr(dom, root, "data-row").as_deref() == Some(id) {
        return Some(root);
    }
    dom.dom_children(root)
        .find_map(|child| row_node(dom, child, id))
}

struct PrepState {
    transaction: CompositionTransaction,
    factory: RetainedComponentFactory<PrepState, ()>,
    sink: LifecycleEventSink,
}

/// Every row is a text field marked `data-row`; `stubborn` refuses a
/// synthetic IME commit, and `wrapped` puts the field inside a marked div.
fn prep_factory() -> RetainedComponentFactory<PrepState, ()> {
    Rc::new(|descriptor| {
        let id = descriptor.key.0.clone();
        let stubborn = descriptor.kind == ComponentKind::from("stubborn");
        let wrapped = descriptor.kind == ComponentKind::from("wrapped");
        Box::new(
            component(
                id,
                |_: &String| TextInput::new(""),
                |_: &String, _: &String, _: &mut TextInput| {},
                move |id: &String, input: &TextInput| -> ComponentView<TextInput, ()> {
                    if wrapped {
                        return Box::new(
                            el::<_, TextInput, ()>("div", text_field_typed(input))
                                .attr("data-row", id.clone()),
                        );
                    }
                    let field = text_field_typed(input).attr("data-row", id.clone());
                    if stubborn {
                        Box::new(
                            on_key(field, |_: &mut TextInput, event: KeyEvent| {
                                if matches!(
                                    &event.key,
                                    Key::Composition(CompositionEvent::Commit(_))
                                ) {
                                    event.prevent_default();
                                    event.stop_immediate_propagation();
                                }
                            })
                            .capture(true),
                        )
                    } else {
                        Box::new(field)
                    }
                },
                |_: &mut PrepState, _: ()| {},
            )
            .memo(),
        )
    })
}

fn prep_state(source: Composition) -> PrepState {
    PrepState {
        transaction: transaction(source),
        factory: prep_factory(),
        sink: LifecycleEventSink::default(),
    }
}

fn prep_logic(state: &PrepState) -> RetainedComposition<PrepState, ()> {
    RetainedComposition::new(&state.transaction, state.factory.clone())
        .with_lifecycle_sink(state.sink.clone())
}

type PrepRunner = GenetAppRunner<
    PrepState,
    fn(&PrepState) -> RetainedComposition<PrepState, ()>,
    RetainedComposition<PrepState, ()>,
>;

fn runner_for(source: Composition) -> (DomHandle, PrepRunner) {
    let dom: DomHandle = Rc::new(RefCell::new(ScriptedDom::new()));
    let runner = GenetAppRunner::new(dom.clone(), prep_logic as fn(&_) -> _, prep_state(source));
    (dom, runner)
}

fn rows(kinds: &[(&str, &str)]) -> Composition {
    model(
        kinds.iter().map(|(key, _)| row(key)).collect(),
        kinds
            .iter()
            .map(|(key, kind)| descriptor(key, kind))
            .collect(),
    )
}

fn node_of(dom: &DomHandle, runner: &PrepRunner, key: &str) -> NodeId {
    row_node(&dom.borrow(), runner.root(), key).unwrap()
}

fn keys(names: &[&str]) -> BTreeSet<ComponentKey> {
    names.iter().map(|name| ComponentKey::from(*name)).collect()
}

fn text_value(dom: &DomHandle, node: NodeId) -> String {
    attr(&dom.borrow(), node, "data-cambium-text-value").unwrap_or_default()
}

fn start_preedit(runner: &mut PrepRunner, node: NodeId) {
    runner.set_focus(Some(node));
    runner.dispatch_key(KeyEvent::new(Key::Composition(CompositionEvent::Preedit {
        text: "漢".into(),
        selection: None,
    })));
}

#[test]
fn route_node_equals_the_element_after_create_move_and_replace() {
    let (dom, mut runner) = runner_for(model(
        vec![group("left", vec![row("a")]), group("right", vec![])],
        vec![descriptor("a", "plain")],
    ));
    let a = ComponentKey::from("a");
    let created = node_of(&dom, &runner, "a");
    assert_eq!(runner.state().sink.route(&a).unwrap().node, Some(created));

    runner.update(|state| {
        let moved = model(
            vec![group("left", vec![]), group("right", vec![row("a")])],
            vec![descriptor("a", "plain")],
        );
        state.transaction.apply(moved, &AllowBindings).unwrap();
    });
    assert_eq!(
        node_of(&dom, &runner, "a"),
        created,
        "a move keeps the node"
    );
    assert_eq!(runner.state().sink.route(&a).unwrap().node, Some(created));

    runner.update(|state| {
        let replaced = model(
            vec![group("left", vec![]), group("right", vec![row("a")])],
            vec![descriptor("a", "stubborn")],
        );
        state.transaction.apply(replaced, &AllowBindings).unwrap();
    });
    let replaced = node_of(&dom, &runner, "a");
    assert_ne!(replaced, created, "a kind change builds a new element");
    assert_eq!(runner.state().sink.route(&a).unwrap().node, Some(replaced));
}

#[test]
fn an_active_preedit_is_committed_once() {
    let (dom, mut runner) = runner_for(rows(&[("a", "plain")]));
    let node = node_of(&dom, &runner, "a");
    start_preedit(&mut runner, node);
    assert!(runner.subtree_move_has_active_interaction(node));

    let sink = runner.state().sink.clone();
    for purpose in [PreparationPurpose::Move, PreparationPurpose::Move] {
        assert_eq!(
            runner.prepare_components(&sink, &keys(&["a"]), purpose),
            Ok(())
        );
    }
    assert!(!runner.subtree_move_has_active_interaction(node));
    assert_eq!(text_value(&dom, node), "漢");

    runner.dispatch_key(KeyEvent::new(Key::Composition(CompositionEvent::Commit(
        "漢".into(),
    ))));
    assert_eq!(
        text_value(&dom, node),
        "漢",
        "the matching late commit is dropped"
    );
}

#[test]
fn an_active_interaction_refuses_at_once_and_leaves_focus_alone() {
    let (dom, mut runner) = runner_for(rows(&[("a", "stubborn"), ("b", "plain")]));
    let (a, b) = (node_of(&dom, &runner, "a"), node_of(&dom, &runner, "b"));
    start_preedit(&mut runner, a);
    let sink = runner.state().sink.clone();

    for purpose in [
        PreparationPurpose::Move,
        PreparationPurpose::Hide,
        PreparationPurpose::Retire,
    ] {
        assert_eq!(
            runner.prepare_components(&sink, &keys(&["a", "b"]), purpose),
            Err(LifecycleRefusal::ActiveInteraction)
        );
        assert_eq!(runner.focus(), Some(a), "refusal precedes any focus change");
        assert!(runner.subtree_move_has_active_interaction(a));
    }
    assert!(!runner.subtree_move_has_active_interaction(b));
}

#[test]
fn hide_and_retire_clear_focus_inside_the_subtree_only() {
    for purpose in [PreparationPurpose::Hide, PreparationPurpose::Retire] {
        let (dom, mut runner) = runner_for(rows(&[("a", "plain"), ("b", "plain")]));
        let (a, b) = (node_of(&dom, &runner, "a"), node_of(&dom, &runner, "b"));
        let sink = runner.state().sink.clone();

        runner.set_focus(Some(b));
        assert_eq!(
            runner.prepare_components(&sink, &keys(&["a"]), purpose),
            Ok(())
        );
        assert_eq!(
            runner.focus(),
            Some(b),
            "focus elsewhere is kept ({purpose:?})"
        );

        runner.set_focus(Some(a));
        assert_eq!(
            runner.prepare_components(&sink, &keys(&["a", "b"]), purpose),
            Ok(())
        );
        assert_eq!(
            runner.focus(),
            None,
            "focus inside is cleared ({purpose:?})"
        );
    }
}

#[test]
fn focus_in_a_descendant_of_the_component_node_counts_as_inside() {
    for (purpose, cleared) in [
        (PreparationPurpose::Move, false),
        (PreparationPurpose::Hide, true),
        (PreparationPurpose::Retire, true),
    ] {
        let (dom, mut runner) = runner_for(rows(&[("a", "wrapped")]));
        let a = node_of(&dom, &runner, "a");
        let inner = dom.borrow().dom_children(a).next().expect("wrapped field");
        assert_ne!(inner, a);
        runner.set_focus(Some(inner));
        let sink = runner.state().sink.clone();

        assert_eq!(
            runner.prepare_components(&sink, &keys(&["a"]), purpose),
            Ok(())
        );
        assert_eq!(runner.focus(), (!cleared).then_some(inner), "{purpose:?}");
    }
}

#[test]
fn move_keeps_focus_and_unbuilt_keys_are_skipped() {
    let (dom, mut runner) = runner_for(rows(&[("a", "plain")]));
    let a = node_of(&dom, &runner, "a");
    let sink = runner.state().sink.clone();
    runner.set_focus(Some(a));

    assert_eq!(
        runner.prepare_components(&sink, &keys(&["a"]), PreparationPurpose::Move),
        Ok(())
    );
    assert_eq!(runner.focus(), Some(a));
    for purpose in [
        PreparationPurpose::Move,
        PreparationPurpose::Hide,
        PreparationPurpose::Retire,
    ] {
        assert_eq!(
            runner.prepare_components(&sink, &keys(&["ghost"]), purpose),
            Ok(()),
            "a key with no route has no node"
        );
        assert_eq!(runner.focus(), Some(a));
    }
}
