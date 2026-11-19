// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The table realization (table projection plan, T4): a table recipe drawn
//! on the data grid.
//!
//! The recipe is compiled as every recipe is; the grid draws the compiled
//! table's rows in their placed order, one column per field in list order,
//! headed by the fields' names. A header click writes the sort onto the
//! recipe's reading through the projection editor, one undo step each, and
//! so do a filter and a grouping; a change to the reading alone is read
//! again through the compiler's own reading order, with no compile. A grouped
//! table draws a heading band per group, which collapses as view state
//! (table D25, D29). The table reads and never writes a cell (table D4).
//! [`ProjectionTable::frozen`] is the same table as an accessible HTML table.

use std::collections::HashMap;

use graphshell_client::frozen_table::{FrozenTable, FrozenTableGroup, FrozenTableRow};
use sceno::InstanceId;
use scenograph::dataset::ProjectionDataset;
use scenograph::group::GroupBand;
use scenograph::table::CollapsedGroups;
use scenograph::{ProjectionDefinition, ProjectionDraft, SortKey};
use scenomise::authoring::{EditorAction, ProjectionEditor};
use scenomise::projection::{CompileIssue, ProjectionCompiler};
use scenomise::table::TableLayout;

use crate::grid::GridFocus;

/// An issue in the compiler's shape: the recipe field it names and why.
fn issue(field: impl Into<String>, message: impl Into<String>) -> CompileIssue {
    CompileIssue {
        field: field.into(),
        message: message.into(),
    }
}

mod bands;
mod cells;
mod keys;
mod view;

pub use bands::{TableBandView, TableHeadingView};

pub use cells::{CellContent, ColumnFacts, TableViewers, Viewer, plain_text};
pub use view::{PROJECTION_TABLE_CSS, projection_table};

/// One column as the grid draws it.
#[derive(Clone, Debug)]
pub struct TableColumnView {
    /// The field's path, the dataset key.
    pub field: String,
    /// The field's name, the header.
    pub name: String,
    pub width: f32,
    pub viewer: Viewer,
    pub facts: ColumnFacts,
}

/// A table recipe open on the data grid, with its editor and its dataset.
pub struct ProjectionTable {
    editor: ProjectionEditor,
    compiler: ProjectionCompiler,
    viewers: TableViewers,
    dataset: ProjectionDataset,
    /// Row rank to the dataset's occurrence index: the rows read, grouped.
    rows: Vec<u32>,
    /// The groups' headings, outer before inner.
    groups: Vec<GroupBand>,
    /// What the grid draws, top to bottom: visible headings and rows.
    bands: Vec<bands::Band>,
    /// Collapsed groups: view state, never the recipe's (table D29).
    collapsed: CollapsedGroups,
    columns: Vec<TableColumnView>,
    row_height: f32,
    header_height: f32,
    /// The scroll offset into the body, in px.
    pub scroll: f32,
    /// The body's height, in px; the header sits above it.
    pub viewport_height: f32,
    focus: GridFocus,
    /// The definition the rows were last placed from.
    placed: Option<ProjectionDefinition>,
    /// Why the last change was refused, worded for a person.
    pub notice: Option<String>,
}

impl ProjectionTable {
    /// Open a recipe over a dataset. Refused when the recipe does not compile
    /// to a table or names a viewer this host cannot draw.
    pub fn open(
        draft: ProjectionDraft,
        dataset: ProjectionDataset,
        compiler: ProjectionCompiler,
        viewers: TableViewers,
    ) -> Result<Self, Vec<CompileIssue>> {
        let mut table = Self {
            editor: ProjectionEditor::new(draft),
            compiler,
            viewers,
            dataset,
            rows: Vec::new(),
            groups: Vec::new(),
            bands: Vec::new(),
            collapsed: CollapsedGroups::default(),
            columns: Vec::new(),
            row_height: 0.0,
            header_height: 0.0,
            scroll: 0.0,
            viewport_height: 480.0,
            focus: GridFocus::default(),
            placed: None,
            notice: None,
        };
        table.rebuild()?;
        Ok(table)
    }

    /// Take the rows and columns from the draft. A change to the sort, the
    /// filter or the grouping alone reads the rows again through the
    /// compiler's own reading order, which is the order a compile places
    /// them in; anything else compiles again.
    fn rebuild(&mut self) -> Result<(), Vec<CompileIssue>> {
        self.rebuild_rows()?;
        self.rebuild_bands();
        Ok(())
    }

    fn rebuild_rows(&mut self) -> Result<(), Vec<CompileIssue>> {
        let definition = self.editor.draft().to_definition().map_err(|issues| {
            issues
                .into_iter()
                .map(|found| issue(found.field, found.message))
                .collect::<Vec<_>>()
        })?;
        let reading_only = self.placed.as_ref().is_some_and(|placed| {
            let mut reading = placed.reading.clone();
            reading.sort.clone_from(&definition.reading.sort);
            reading.filter.clone_from(&definition.reading.filter);
            reading.group.clone_from(&definition.reading.group);
            placed.reading != definition.reading
                && ProjectionDefinition {
                    reading,
                    ..placed.clone()
                } == definition
        });
        if reading_only {
            let reading = self.compiler.table_reading(&definition, &self.dataset)?;
            self.rows = reading.rows.into_iter().map(|index| index as u32).collect();
            self.groups = reading.groups;
            self.placed = Some(definition);
            return Ok(());
        }
        let compiled = self.compiler.compile(&definition, &self.dataset)?;
        let layout = TableLayout::of(&compiled, &definition).ok_or_else(|| {
            vec![issue(
                "arrangement.kind",
                "the table realization draws only the table arrangement",
            )]
        })?;
        let mut columns = Vec::with_capacity(layout.columns.len());
        let mut issues = Vec::new();
        for (index, column) in layout.columns.iter().enumerate() {
            let viewer_id = definition
                .fields
                .get(index)
                .map_or(scenograph::fields::DEFAULT_VIEWER, |field| {
                    field.viewer_id()
                });
            let Some(viewer) = self.viewers.get(viewer_id) else {
                issues.push(issue(
                    format!("fields[{index}].viewer"),
                    format!(
                        "the field {:?} asks for the viewer {viewer_id:?}, which this table cannot draw",
                        column.name
                    ),
                ));
                continue;
            };
            let facts = ColumnFacts::of(
                self.dataset
                    .occurrences
                    .iter()
                    .map(|occurrence| occurrence.values.get(&column.field)),
            );
            columns.push(TableColumnView {
                field: column.field.clone(),
                name: column.name.clone(),
                width: column.width,
                viewer,
                facts,
            });
        }
        if !issues.is_empty() {
            return Err(issues);
        }
        let index: HashMap<&str, u32> = self
            .dataset
            .occurrences
            .iter()
            .enumerate()
            .map(|(at, occurrence)| (occurrence.occurrence_id.as_str(), at as u32))
            .collect();
        let rows = (0..compiled.occurrence_by_instance.len())
            .map(|rank| {
                let id = &compiled.occurrence_by_instance[&InstanceId(rank as u32)];
                index[id.as_str()]
            })
            .collect();
        self.rows = rows;
        // The compile placed the headings; their groups, by value, come from
        // the same reading.
        self.groups = if definition.reading.group.is_empty() {
            Vec::new()
        } else {
            self.compiler
                .table_reading(&definition, &self.dataset)?
                .groups
        };
        self.placed = Some(definition);
        self.columns = columns;
        self.row_height = layout.table.row_height;
        self.header_height = layout.table.header;
        Ok(())
    }

    pub fn draft(&self) -> &ProjectionDraft {
        self.editor.draft()
    }

    pub fn editor(&self) -> &ProjectionEditor {
        &self.editor
    }

    pub fn columns(&self) -> &[TableColumnView] {
        &self.columns
    }

    /// How many rows the reading keeps, in every group, collapsed or not.
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// How many rows the dataset holds before the filter.
    pub fn source_row_count(&self) -> usize {
        self.dataset.occurrences.len()
    }

    pub fn row_height(&self) -> f32 {
        self.row_height
    }

    pub fn header_height(&self) -> f32 {
        self.header_height
    }

    pub fn focus(&self) -> GridFocus {
        self.focus
    }

    /// The caption: the recipe's title, else its label.
    pub fn caption(&self) -> &str {
        let draft = self.editor.draft();
        if draft.appearance.title.trim().is_empty() {
            &draft.label
        } else {
            &draft.appearance.title
        }
    }

    /// The occurrence id on row `rank`.
    pub fn row_id(&self, rank: usize) -> Option<&str> {
        let at = *self.rows.get(rank)?;
        Some(self.dataset.occurrences[at as usize].occurrence_id.as_str())
    }

    /// The cell at row `rank`, column `col`, through its viewer.
    pub fn cell(&self, rank: usize, col: usize) -> CellContent {
        let (Some(at), Some(column)) = (self.rows.get(rank), self.columns.get(col)) else {
            return CellContent::Empty;
        };
        let values = &self.dataset.occurrences[*at as usize].values;
        (column.viewer)(values.get(&column.field), &column.facts)
    }

    /// The column the reading's order leads with, and whether it runs
    /// descending: the outermost grouping when grouped, else the sort's lead
    /// key. `aria-sort` marks this column alone (table D35).
    pub fn sorted(&self) -> Option<(usize, bool)> {
        let reading = &self.editor.draft().reading;
        let key = reading.group.first().or_else(|| reading.sort.first())?;
        let col = self.columns.iter().position(|c| c.field == key.field)?;
        Some((col, key.descending))
    }

    /// The sort keys that order rows within the groups, by column, in
    /// priority order: a key on a grouped field is left out, as the grouping
    /// orders it. A key whose field is not a column is left out too.
    pub fn sort_columns(&self) -> Vec<(usize, bool)> {
        let reading = &self.editor.draft().reading;
        self.key_columns(
            reading
                .sort
                .iter()
                .filter(|key| !reading.group.iter().any(|g| g.field == key.field)),
        )
    }

    /// The grouping's keys by column, outermost first.
    pub fn group_columns(&self) -> Vec<(usize, bool)> {
        self.key_columns(self.editor.draft().reading.group.iter())
    }

    fn key_columns<'a>(&self, keys: impl Iterator<Item = &'a SortKey>) -> Vec<(usize, bool)> {
        keys.filter_map(|key| {
            let col = self.columns.iter().position(|c| c.field == key.field)?;
            Some((col, key.descending))
        })
        .collect()
    }

    /// Sort by column `col`, or flip it if it already leads: one undo step
    /// on the recipe. `now_ms` is the host's clock.
    pub fn click_header(&mut self, col: usize, now_ms: u64) {
        self.press_header(col, false, now_ms);
    }

    /// A header pressed: alone, it replaces the sort as [`Self::click_header`]
    /// does; with Shift (`add`), it adds the column as the next key or flips
    /// it where it already is one (table D26).
    pub fn press_header(&mut self, col: usize, add: bool, now_ms: u64) {
        let Some(field) = self.columns.get(col).map(|c| c.field.clone()) else {
            return;
        };
        let current = &self.editor.draft().reading.sort;
        let keys = if add {
            scenograph::sort::header_add(current, &field)
        } else {
            scenograph::sort::header_click(current, &field)
        };
        self.apply_sort(keys, now_ms);
    }

    /// Replace the recipe's sort; a refused sort is undone and noticed.
    pub fn apply_sort(&mut self, keys: Vec<SortKey>, now_ms: u64) {
        self.editor.reduce(EditorAction::SetSort(keys), now_ms);
        self.recompile_or_step_back();
    }

    /// Replace the recipe's filter: one undo step; a refused one is undone
    /// and noticed.
    pub fn apply_filter(&mut self, conditions: Vec<scenograph::filter::Condition>, now_ms: u64) {
        self.editor
            .reduce(EditorAction::SetFilter(conditions), now_ms);
        self.recompile_or_step_back();
    }

    /// Replace the recipe's grouping: one undo step; a refused one is
    /// undone and noticed.
    pub fn apply_group(&mut self, keys: Vec<SortKey>, now_ms: u64) {
        self.editor.reduce(EditorAction::SetGroup(keys), now_ms);
        self.recompile_or_step_back();
    }

    pub fn undo(&mut self) -> bool {
        let undone = self.editor.undo();
        if undone {
            self.recompile_or_step_back();
        }
        undone
    }

    pub fn redo(&mut self) -> bool {
        let redone = self.editor.redo();
        if redone {
            self.recompile_or_step_back();
        }
        redone
    }

    fn recompile_or_step_back(&mut self) {
        match self.rebuild() {
            Ok(()) => self.notice = None,
            Err(issues) => {
                self.editor.undo();
                let _ = self.rebuild();
                self.notice = issues.first().map(|issue| issue.message.clone());
            },
        }
    }

    /// The largest scroll that still fills the body.
    pub fn max_scroll(&self) -> f32 {
        (self.bands.len() as f32 * self.row_height - self.viewport_height).max(0.0)
    }

    /// Scroll by `dy` px, clamped to the rows.
    pub fn scroll_by(&mut self, dy: f32) {
        self.scroll = (self.scroll + dy).clamp(0.0, self.max_scroll());
    }

    /// The same table frozen: a caption, one column header per field, a row
    /// per occurrence read in this order, each cell in words, and a heading
    /// per group. Every group is open: collapse is the view's, not the
    /// recipe's.
    pub fn frozen(&self) -> FrozenTable {
        let rows = (0..self.rows.len())
            .map(|rank| {
                let occurrence = &self.dataset.occurrences[self.rows[rank] as usize];
                FrozenTableRow {
                    source: occurrence.source.clone(),
                    cells: (0..self.columns.len())
                        .map(|col| self.cell(rank, col).words())
                        .collect(),
                }
            })
            .collect();
        FrozenTable {
            caption: self.caption().to_owned(),
            columns: self.columns.iter().map(|c| c.name.clone()).collect(),
            sorted: self.sorted(),
            rows,
            groups: self
                .groups
                .iter()
                .enumerate()
                .map(|(index, band)| FrozenTableGroup {
                    position: band.start,
                    level: band.level,
                    label: self.heading_label(index),
                    count: band.count,
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod group_tests;

#[cfg(test)]
mod key_tests;
