// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! A compiled table read back (table projection plan, T3): its columns from
//! the recipe's fields list, a point picked to the row and field under it, and
//! a saved selection placed on the current rows.
//!
//! Rows are the scene's instances, so a row is found where the scene put it,
//! holds included. Columns are the fields in list order (table D14); a recipe
//! with no fields list has one column, its label.

use sceno::{Frame, InstanceId, Rect, Size2, Table, Vec2};
use scenograph::ProjectionDefinition;
use scenograph::table::{TableSelection, TableTarget};

use crate::projection::CompiledProjection;

/// One column: the field it shows and where it sits in a row.
#[derive(Clone, Debug, PartialEq)]
pub struct TableColumn {
    /// The field's path, the dataset key.
    pub field: String,
    /// The field's name, for a header.
    pub name: String,
    /// The left edge, from the row's left edge.
    pub x: f32,
    pub width: f32,
}

/// What lies under a point.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TableHit {
    /// A column's header.
    Header { field: String },
    /// A cell: a row and the field under the point.
    Cell(TableTarget),
}

/// A compiled table's geometry, with the frame applied.
#[derive(Clone, Debug, PartialEq)]
pub struct TableLayout {
    pub table: Table,
    pub columns: Vec<TableColumn>,
}

impl TableLayout {
    /// The layout of a compiled table; `None` for any other arrangement.
    pub fn of(compiled: &CompiledProjection, definition: &ProjectionDefinition) -> Option<Self> {
        let sceno::Arrangement::Table(config) = &compiled.score.arrangement else {
            return None;
        };
        let mut table = config.clone();
        if let Some(Frame::Flow { origin, .. } | Frame::Unbounded { origin }) =
            &compiled.score.frame
        {
            table.origin = *origin;
        }
        let listed: Vec<(String, String)> = if definition.fields.is_empty() {
            let label = match &definition.encoding.label {
                Some(scenograph::Channel::Field(path)) => path.clone(),
                _ => String::new(),
            };
            vec![(label.clone(), label)]
        } else {
            definition
                .fields
                .iter()
                .map(|field| (field.path.clone(), field.name.clone()))
                .collect()
        };
        let columns = listed
            .into_iter()
            .zip(&table.columns)
            .enumerate()
            .map(|(index, ((field, name), width))| TableColumn {
                field,
                name,
                x: table.column_x(index),
                width: *width,
            })
            .collect();
        Some(Self { table, columns })
    }

    /// The row and field under `world`, or the header of the column there.
    /// Rows outrank the header band; a miss is `None`, never the nearest.
    pub fn pick(&self, compiled: &CompiledProjection, world: Vec2) -> Option<TableHit> {
        let hit_row = compiled
            .scene
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| item.visible)
            .filter_map(|(index, item)| {
                let to_world = compiled.scene.to_world(item.space)?.then(&item.transform);
                let local = to_world.inverse()?.apply(world);
                item.footprint
                    .contains(local)
                    .then(|| (item.layer, index, local, item.footprint.bounds()))
            })
            .max_by_key(|(layer, index, ..)| (*layer, *index));
        if let Some((_, index, local, Some(bounds))) = hit_row {
            let column = self.table.column_at(local.x - bounds.origin.x)?;
            let row = compiled
                .occurrence_by_instance
                .get(&InstanceId(index as u32))?;
            return Some(TableHit::Cell(TableTarget::cell(
                row.clone(),
                self.columns.get(column)?.field.clone(),
            )));
        }
        let header = self.table.header_rect();
        let inside = world.y >= header.origin.y && world.y < header.origin.y + header.size.h;
        let column = self.table.column_at(world.x - header.origin.x);
        match (inside, column) {
            (true, Some(column)) => Some(TableHit::Header {
                field: self.columns.get(column)?.field.clone(),
            }),
            _ => None,
        }
    }

    /// Where a target's row, or its cell, is in the world now.
    pub fn target_rect(&self, compiled: &CompiledProjection, target: &TableTarget) -> Option<Rect> {
        let instance = compiled.instance_by_occurrence.get(&target.row)?;
        let item = compiled.scene.items.get(instance.0 as usize)?;
        let bounds = item.footprint.bounds()?;
        let (left, width) = match &target.field {
            None => (0.0, bounds.size.w),
            Some(field) => {
                let column = self.columns.iter().find(|column| &column.field == field)?;
                (column.x, column.width)
            },
        };
        let to_world = compiled.scene.to_world(item.space)?.then(&item.transform);
        let corners = [
            Vec2::new(bounds.origin.x + left, bounds.origin.y),
            Vec2::new(
                bounds.origin.x + left + width,
                bounds.origin.y + bounds.size.h,
            ),
        ]
        .map(|corner| to_world.apply(corner));
        let min = Vec2::new(
            corners[0].x.min(corners[1].x),
            corners[0].y.min(corners[1].y),
        );
        let max = Vec2::new(
            corners[0].x.max(corners[1].x),
            corners[0].y.max(corners[1].y),
        );
        Some(Rect::new(min, Size2::new(max.x - min.x, max.y - min.y)))
    }

    /// Keep the targets whose row this compile still reads and whose field
    /// is still a column, returning the rest.
    pub fn restore(
        &self,
        compiled: &CompiledProjection,
        selection: &mut TableSelection,
    ) -> Vec<TableTarget> {
        selection.retain_present(
            |row| compiled.instance_by_occurrence.contains_key(row),
            |field| self.columns.iter().any(|column| column.field == field),
        )
    }
}

#[cfg(test)]
#[path = "table_tests.rs"]
mod tests;
