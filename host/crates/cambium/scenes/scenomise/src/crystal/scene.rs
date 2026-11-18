// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Crystal edits the live scene, using the same pinned hold that other arrangements report.
use super::*;
use crate::projection::CompiledProjection;
use sceno::{Crystal, Footprint, HeldPlacement, HonoredHold, InstanceId, Scene};

#[derive(Debug, Clone, Copy)]
pub enum CrystalEdit {
    Pin(InstanceId),
    Unpin(InstanceId),
    /// Drop at the requested world center, or slide around the current nucleus.
    Drop {
        tile: InstanceId,
        at: Vec2,
    },
}

/// Apply a complete edit to the live projection. The base placement is kept
/// for the next compiler refresh and the relationship reports follow the edit.
pub fn edit(projection: &mut CompiledProjection, edit: CrystalEdit) -> Result<(), &'static str> {
    let sceno::Arrangement::Crystal(config) = &projection.score.arrangement else {
        return Err("Choose the Crystal arrangement");
    };
    let scene = &projection.scene;
    let tile = match edit {
        CrystalEdit::Pin(tile) | CrystalEdit::Unpin(tile) | CrystalEdit::Drop { tile, .. } => {
            tile.0 as usize
        },
    };
    if tile >= scene.items.len() {
        return Err("The Crystal tile is absent");
    }
    let tiles = scene
        .items
        .iter()
        .map(item_rect)
        .collect::<Option<Vec<_>>>()
        .ok_or("Crystal needs rectangular tiles in world space")?;
    let links = scene.relation_constraints.iter().map(|report| {
        (
            report.constraint.from.0 as usize,
            report.constraint.to.0 as usize,
        )
    });
    let pins = scene
        .honored_holds
        .iter()
        .filter(|hold| hold.placement.hold == sceno::Hold::Pinned)
        .map(|hold| hold.instance.0 as usize)
        .collect();
    let mut layout = Layout::new(tiles, links, pins, config.smallest_side, config.tile)
        .ok_or("The tile sizes do not fit this Crystal recipe")?;
    match edit {
        CrystalEdit::Pin(_) => layout.pin(tile),
        CrystalEdit::Unpin(_) => layout.unpin(tile),
        CrystalEdit::Drop { at, .. } => {
            if !layout.drop_at(tile, at) {
                return Err("The drop would move a pin or overlap another tile");
            }
        },
    }
    apply_layout(&mut projection.base_scene, &layout);
    apply_layout(&mut projection.scene, &layout);
    refresh(
        &mut projection.scene,
        config.smallest_side,
        projection.selected,
    );
    Ok(())
}

fn item_rect(item: &sceno::ProjectedItem) -> Option<Rect> {
    let Footprint::Rect { size } = item.footprint else {
        return None;
    };
    (item.space == Scene::WORLD && item.transform.scale == 1.0 && item.transform.rotate == 0.0)
        .then(|| centered(item.transform.translate, size))
}

fn apply_layout(scene: &mut Scene, layout: &Layout) {
    for (item, tile) in scene.items.iter_mut().zip(&layout.tiles) {
        item.transform.translate = center(*tile);
        item.footprint = Footprint::Rect { size: tile.size };
    }
    scene.honored_holds = layout
        .pins
        .iter()
        .map(|&i| HonoredHold {
            instance: InstanceId(i as u32),
            placement: HeldPlacement::pinned(
                scene.sources[scene.items[i].source.0 as usize].clone(),
                center(layout.tiles[i]),
            ),
        })
        .collect();
    if let Some(bounds) = layout.tiles.iter().copied().reduce(Rect::union) {
        scene.bounds = bounds;
    }
}

pub(crate) fn initialize(
    scene: &mut Scene,
    config: &Crystal,
    constraints: Vec<sceno::RelationConstraint>,
    selected: Option<InstanceId>,
) {
    scene.relation_constraints = constraints
        .into_iter()
        .map(|constraint| sceno::RelationConstraintReport {
            constraint,
            honored: false,
            reason: String::new(),
        })
        .collect();
    refresh(scene, config.smallest_side, selected);
}

fn refresh(scene: &mut Scene, smallest: f32, selected: Option<InstanceId>) {
    let minimum = smallest / 2.0;
    let pinned = !scene.honored_holds.is_empty();
    // The compiler orders the routes and reports together by disclosure id.
    for (relation, report) in scene
        .relations
        .iter_mut()
        .zip(&mut scene.relation_constraints)
    {
        let ends = scene
            .items
            .get(relation.from.0 as usize)
            .and_then(item_rect)
            .zip(scene.items.get(relation.to.0 as usize).and_then(item_rect));
        let contact = ends.and_then(|(a, b)| border(a, b, minimum));
        report.honored = contact.is_some();
        report.reason = if report.honored {
            "shared border".into()
        } else {
            format!("no shared border of {minimum} px")
        };
        let selected_end = selected == Some(relation.from) || selected == Some(relation.to);
        relation.points = if let Some(points) = contact {
            if pinned || selected_end {
                points.into()
            } else {
                Vec::new()
            }
        } else if selected_end {
            ends.map_or_else(Vec::new, |(a, b)| {
                vec![edge_toward(a, center(b)), edge_toward(b, center(a))]
            })
        } else {
            Vec::new()
        };
    }
}

fn edge_toward(rect: Rect, destination: Vec2) -> Vec2 {
    let at = center(rect);
    let offset = Vec2::new(destination.x - at.x, destination.y - at.y);
    if offset == Vec2::ZERO {
        return at;
    }
    let scale = (rect.size.w / (2.0 * offset.x.abs())).min(rect.size.h / (2.0 * offset.y.abs()));
    Vec2::new(at.x + offset.x * scale, at.y + offset.y * scale)
}
