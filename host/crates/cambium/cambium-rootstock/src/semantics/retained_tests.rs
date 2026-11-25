// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Each patched tree must equal a full composition of the same document,
//! node for node (table plan D59, D66).
use super::*;
use crate::WindowDom;
use genet_scripted_dom::ScriptedDom;
use layout_dom_api::{LayoutDomMut, LocalName, Namespace, QualName};

/// The scroll-batch route's shape: one absolute strip translated by `top`,
/// absolute rows entering and leaving it.
const SHEET: &str = "
    .view { display: block; position: relative; width: 400px; height: 200px; overflow: hidden; }
    .strip { position: absolute; left: 0; top: 0; width: 400px; height: 4000px; }
    .row { position: absolute; left: 0; width: 400px; height: 20px; display: flex; }
    .row span { display: block; width: 60px; height: 20px; }";

fn name(local: &str) -> QualName {
    QualName::new(None, Namespace::from(""), LocalName::from(local))
}

/// The table's shape: a clipped view, a strip moved by `top`, and rows.
struct Table {
    dom: ScriptedDom,
    strip: NodeId,
    rows: Vec<NodeId>,
    next: usize,
    layout: OwnedLayout,
    retained: RetainedSemantics,
    leaves: LeafRegistry<u64>,
    producers: ProducerRegistry,
    focus: Option<u64>,
}

impl Table {
    fn new(rows: usize) -> Self {
        let mut dom = ScriptedDom::new();
        let view = dom.create_element(name("div"));
        dom.set_attribute(view, name("class"), "view");
        dom.set_attribute(view, name("role"), "grid");
        dom.set_attribute(view, name("aria-label"), "Rows");
        let strip = dom.create_element(name("div"));
        dom.set_attribute(strip, name("class"), "strip");
        dom.set_attribute(strip, name("role"), "rowgroup");
        dom.append_child(dom.document(), view);
        dom.append_child(view, strip);
        let mut table_rows = Vec::new();
        for index in 0..rows {
            let row = row(&mut dom, index);
            dom.append_child(strip, row);
            table_rows.push(row);
        }
        dom.drain_mutations(&mut Vec::new());
        let layout = OwnedLayout::new(
            &WindowDom::document(&dom),
            &[SHEET],
            400.0,
            300.0,
            &[],
            &Default::default(),
        );
        let leaves = LeafRegistry::default();
        let producers = ProducerRegistry::default();
        let retained = RetainedSemantics::new(
            &WindowDom::document(&dom),
            &layout,
            None,
            &leaves,
            &producers,
        );
        let mut table = Self {
            dom,
            strip,
            rows: table_rows,
            next: rows,
            layout,
            retained,
            leaves,
            producers,
            focus: None,
        };
        let drained = table.dom.pending_mutations().0;
        table.layout.begin_a11y_journal(drained);
        table
    }

    /// Lay out the drained batch and update the retained tree, as a frame does.
    fn frame(&mut self) -> Option<SemanticChanges> {
        let mut mutations = Vec::new();
        self.dom.drain_mutations(&mut mutations);
        let view = WindowDom::document(&self.dom);
        if !mutations.is_empty() {
            self.layout.rebuild(&view, 400.0, 300.0, &mutations, true);
        }
        let journal = self.layout.take_a11y_journal();
        self.retained.update(
            &view,
            &self.layout,
            self.focus,
            &self.leaves,
            &self.producers,
            journal,
        );
        self.layout
            .begin_a11y_journal(self.dom.pending_mutations().0);
        self.assert_equal();
        self.retained.changes.clone()
    }

    fn assert_equal(&self) {
        let view = WindowDom::document(&self.dom);
        let full = super::super::project_semantics(
            &view,
            &self.layout,
            &self.leaves,
            &self.producers,
            self.focus,
        );
        let patched = &self.retained.projection;
        let nodes = |projection: &SemanticProjection| -> HashMap<_, _> {
            projection.tree.nodes.iter().cloned().collect()
        };
        let (want, got) = (nodes(&full), nodes(patched));
        for (id, node) in &want {
            assert_eq!(got.get(id), Some(node), "node {id:?}");
        }
        assert_eq!(got.len(), want.len());
        assert_eq!(patched.tree.tree, full.tree.tree);
        assert_eq!(patched.tree.focus, full.tree.focus);
        assert_eq!(patched.dom_targets, full.dom_targets);
    }

    /// One scroll step: the first row leaves, a row enters, the strip moves.
    fn scroll(&mut self) {
        let leaving = self.rows.remove(0);
        self.dom.remove(leaving);
        let entering = row(&mut self.dom, self.next);
        self.next += 1;
        self.dom.append_child(self.strip, entering);
        self.rows.push(entering);
        let top = format!("top: -{}px", self.next * 3);
        self.dom.set_attribute(self.strip, name("style"), &top);
    }
}

fn row(dom: &mut ScriptedDom, index: usize) -> NodeId {
    let row = dom.create_element(name("div"));
    dom.set_attribute(row, name("class"), "row");
    dom.set_attribute(row, name("role"), "row");
    dom.set_attribute(row, name("data-key"), &format!("row-{index}"));
    dom.set_attribute(row, name("style"), &format!("top: {}px", index * 20));
    for column in 0..3 {
        let cell = dom.create_element(name("span"));
        dom.set_attribute(cell, name("role"), "cell");
        let text = dom.create_text(&format!("r{index}c{column}"));
        dom.append_child(cell, text);
        dom.append_child(row, cell);
    }
    row
}

fn id(dom: &ScriptedDom, node: NodeId) -> A11yNodeId {
    A11yNodeId(WindowDom::document(dom).opaque_id(node))
}

#[test]
fn a_scroll_batch_patches_the_rows_and_moves_only_the_strip() {
    let mut table = Table::new(12);
    for _ in 0..3 {
        table.scroll();
        let changes = table.frame().expect("a scroll batch patches");
        // A row is the row and its three cells.
        assert_eq!(changes.added.len(), 4, "{changes:?}");
        assert_eq!(changes.removed.len(), 4, "{changes:?}");
        let strip = id(&table.dom, table.strip);
        assert!(changes.changed.contains(&strip), "{changes:?}");
        for row in &table.rows[..table.rows.len() - 1] {
            assert!(!changes.changed.contains(&id(&table.dom, *row)));
        }
    }
}

#[test]
fn an_attribute_write_on_a_kept_row_changes_that_row() {
    let mut table = Table::new(6);
    let row = table.rows[2];
    table.dom.set_attribute(row, name("aria-label"), "Picked");
    let changes = table.frame().expect("a kept write patches");
    assert_eq!(changes.changed, vec![id(&table.dom, row)]);
    assert!(changes.added.is_empty() && changes.removed.is_empty());
}

#[test]
fn a_focus_move_changes_the_two_rows() {
    let mut table = Table::new(6);
    for row in table.rows.clone() {
        table.dom.set_attribute(row, name("tabindex"), "0");
    }
    table.frame();
    let (a, b) = (table.rows[1], table.rows[3]);
    table.focus = Some(id(&table.dom, a).0);
    table.frame().expect("a focus move patches");
    table.focus = Some(id(&table.dom, b).0);
    let changes = table.frame().expect("a focus move patches");
    assert_eq!(table.retained.projection.tree.focus, id(&table.dom, b));
    for row in [a, b] {
        assert!(
            changes.changed.contains(&id(&table.dom, row)),
            "{changes:?}"
        );
    }
}

#[test]
fn an_unchanged_frame_patches_nothing() {
    let mut table = Table::new(6);
    let changes = table.frame().expect("nothing to refuse");
    assert_eq!(changes, SemanticChanges::default());
}

#[test]
fn a_host_read_attribute_on_a_kept_node_composes_everything() {
    let mut table = Table::new(6);
    for name_read in ["data-cambium-set-value", "data-key", "role"] {
        let row = table.rows[1];
        table.dom.set_attribute(row, name(name_read), "true");
        assert_eq!(table.frame(), None, "{name_read}");
    }
}

#[test]
fn a_text_edit_composes_everything_and_still_matches() {
    let mut table = Table::new(6);
    let cell = table.dom.dom_children(table.rows[0]).next().unwrap();
    let text = table.dom.dom_children(cell).next().unwrap();
    table.dom.set_text(text, "edited");
    assert_eq!(table.frame(), None);
    table.scroll();
    assert!(table.frame().is_some(), "patching resumes after a rebuild");
}

#[test]
fn a_full_layout_in_between_composes_everything() {
    let mut table = Table::new(6);
    table.layout.journal_full_layout();
    assert_eq!(table.frame(), None);
}
