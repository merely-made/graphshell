// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Resolve relation requests through the arrangement, then route their drawn forms.

use sceno::{
    Footprint, InstanceId, Rect, RelationConstraint, RelationConstraintReport, RelationForm, Scene,
    Size2, Vec2,
};
use std::collections::BTreeMap;

mod placement;

/// Arrange placed forms. The caller validates declared capabilities before
/// calling this pass. Pins and occurrence identities remain authoritative.
pub fn arrange(
    score: &sceno::Score,
    registry: &crate::SolverRegistry,
    scene: &mut Scene,
    constraints: &[RelationConstraint],
    assigned: bool,
    gap: f32,
) -> Result<(), Vec<crate::projection::CompileIssue>> {
    scene.relation_constraints = constraints
        .iter()
        .cloned()
        .map(|constraint| RelationConstraintReport {
            constraint,
            honored: false,
            reason: String::new(),
        })
        .collect();
    placement::order(score, registry, scene, constraints)?;
    placement::containment(score, scene, constraints, assigned, gap);
    refresh_scene(scene);
    Ok(())
}

/// Route changed positions and refresh the outcomes of placed requests.
/// Matrix cells use the existing two-reading matrix pitch and occurrence order.
pub fn refresh_scene(scene: &mut Scene) {
    let rank = ranks(scene);
    let node_bounds = content_bounds(scene);
    let pitch = crate::matrix::MATRIX_CELL_SIZE;
    let origin = Vec2::new(
        node_bounds.origin.x + node_bounds.size.w + pitch,
        node_bounds.origin.y + pitch,
    );
    let positions: Vec<_> = scene
        .items
        .iter()
        .map(|item| {
            scene
                .to_world(item.space)
                .map(|world| world.then(&item.transform).apply(Vec2::ZERO))
        })
        .collect();
    for relation in &mut scene.relations {
        let endpoints = positions
            .get(relation.from.0 as usize)
            .copied()
            .flatten()
            .zip(positions.get(relation.to.0 as usize).copied().flatten());
        relation.points = match (relation.form, endpoints) {
            (RelationForm::Stroke, Some((from, to))) => vec![from, to],
            (RelationForm::MatrixCell, Some(_)) => {
                let x = origin.x + rank[relation.to.0 as usize] as f32 * pitch;
                let y = origin.y + rank[relation.from.0 as usize] as f32 * pitch;
                rect_points(Rect::new(
                    Vec2::new(x, y),
                    Size2::new(pitch - 8.0, pitch - 8.0),
                ))
            },
            _ => Vec::new(),
        };
    }
    let constraints: Vec<_> = scene
        .relation_constraints
        .iter()
        .map(|r| r.constraint.clone())
        .collect();
    let (_, conflicts) = placement::forest(&constraints, scene.items.len());
    let outcomes: Vec<_> = constraints
        .iter()
        .enumerate()
        .map(|(index, request)| {
            let endpoints = item_bounds(scene, request.from).zip(item_bounds(scene, request.to));
            if endpoints.is_none() {
                return (false, "a requested occurrence is absent".into());
            }
            match request.form {
                RelationForm::Order => {
                    let honored = rank[request.from.0 as usize] < rank[request.to.0 as usize];
                    (
                        honored,
                        if honored {
                            "placed in the requested arrangement order"
                        } else {
                            "conflicting order requests, groups or pinned slots"
                        }
                        .into(),
                    )
                },
                RelationForm::Containment => {
                    if let Some(reason) = conflicts.get(&index) {
                        return (false, reason.clone());
                    }
                    let (parent, child) = endpoints.unwrap();
                    let honored = contains(parent, child);
                    (
                        honored,
                        if honored {
                            "placed inside the requested parent"
                        } else {
                            "parent extent, groups or pins prevent containment"
                        }
                        .into(),
                    )
                },
                _ => (false, "this placed form has no arrangement reader".into()),
            }
        })
        .collect();
    for (report, (honored, reason)) in scene.relation_constraints.iter_mut().zip(outcomes) {
        report.honored = honored;
        report.reason = reason;
    }
    if scene.frame.is_some()
        || !scene.relation_constraints.is_empty()
        || scene
            .relations
            .iter()
            .any(|r| r.form == RelationForm::MatrixCell)
    {
        let mut bounds = node_bounds;
        if scene
            .relations
            .iter()
            .any(|r| r.form == RelationForm::MatrixCell)
        {
            // Include headings and empty rows/columns, so a sparse matrix has
            // stable dimensions and every occurrence label stays in view.
            bounds = bounds.union(Rect::new(
                Vec2::new(origin.x - pitch, origin.y - pitch),
                Size2::new(
                    (scene.items.len() + 1) as f32 * pitch,
                    (scene.items.len() + 1) as f32 * pitch,
                ),
            ));
        }
        scene.bounds = bounds;
    }
}

/// Row and column heading positions for a scene's relation matrix.
pub fn matrix_headings(scene: &Scene) -> Vec<(InstanceId, Vec2, Vec2)> {
    if !scene
        .relations
        .iter()
        .any(|r| r.form == RelationForm::MatrixCell)
    {
        return Vec::new();
    }
    let bounds = content_bounds(scene);
    let pitch = crate::matrix::MATRIX_CELL_SIZE;
    let origin = Vec2::new(
        bounds.origin.x + bounds.size.w + pitch,
        bounds.origin.y + pitch,
    );
    ranks(scene)
        .into_iter()
        .enumerate()
        .map(|(index, rank)| {
            (
                InstanceId(index as u32),
                Vec2::new(
                    origin.x - pitch * 0.5,
                    origin.y + rank as f32 * pitch + pitch * 0.5,
                ),
                Vec2::new(origin.x + (rank as f32 + 0.5) * pitch, origin.y - 8.0),
            )
        })
        .collect()
}

fn ranks(scene: &Scene) -> Vec<usize> {
    let mut ranks: Vec<_> = (0..scene.items.len()).collect();
    for (rank, instance) in scene.arrangement_order.iter().enumerate() {
        if let Some(value) = ranks.get_mut(instance.0 as usize) {
            *value = rank;
        }
    }
    ranks
}

pub(super) fn item_bounds(scene: &Scene, id: InstanceId) -> Option<Rect> {
    let item = scene.items.get(id.0 as usize)?;
    let local = item.footprint.bounds()?;
    let transform = scene.to_world(item.space)?.then(&item.transform);
    let points = rect_points(local);
    let first = transform.apply(points[0]);
    let mut bounds = Rect::new(first, Size2::new(0.0, 0.0));
    for point in points {
        let point = transform.apply(point);
        if !point.x.is_finite() || !point.y.is_finite() {
            return None;
        }
        bounds = bounds.union(Rect::new(point, Size2::new(0.0, 0.0)));
    }
    Some(bounds)
}

fn content_bounds(scene: &Scene) -> Rect {
    let mut bounds: Option<Rect> = None;
    for index in 0..scene.items.len() {
        if let Some(rect) = item_bounds(scene, InstanceId(index as u32)) {
            bounds = Some(bounds.map_or(rect, |old| old.union(rect)));
        }
    }
    let geometry = scene
        .regions
        .iter()
        .filter_map(|region| {
            Some((
                region.space,
                sceno::Transform2::IDENTITY,
                region.contour.as_ref()?,
            ))
        })
        .chain(
            scene
                .backdrops
                .iter()
                .filter(|backdrop| backdrop.visible)
                .map(|backdrop| (backdrop.space, backdrop.transform, &backdrop.footprint)),
        );
    for (space, local, footprint) in geometry {
        let Some(rect) = footprint.bounds() else {
            continue;
        };
        let Some(world) = scene.to_world(space) else {
            continue;
        };
        let transform = world.then(&local);
        for point in rect_points(rect).into_iter().map(|p| transform.apply(p)) {
            let rect = Rect::new(point, Size2::default());
            bounds = Some(bounds.map_or(rect, |old| old.union(rect)));
        }
    }
    if let Some(frame) = scene
        .frame
        .as_ref()
        .and_then(|frame| frame.world_bounds(scene))
    {
        bounds = Some(bounds.map_or(frame, |old| old.union(frame)));
    }
    if bounds.is_none() {
        if let Some(sceno::SceneFrame {
            spec: sceno::Frame::Unbounded { origin },
            space,
        }) = &scene.frame
        {
            if let Some(world) = scene.to_world(*space) {
                bounds = Some(Rect::new(world.apply(*origin), Size2::default()));
            }
        }
    }
    bounds.unwrap_or_default()
}

fn contains(parent: Rect, child: Rect) -> bool {
    const TOLERANCE: f32 = 1.0e-3;
    child.origin.x >= parent.origin.x - TOLERANCE
        && child.origin.y >= parent.origin.y - TOLERANCE
        && child.origin.x + child.size.w <= parent.origin.x + parent.size.w + TOLERANCE
        && child.origin.y + child.size.h <= parent.origin.y + parent.size.h + TOLERANCE
}

fn rect_points(rect: Rect) -> Vec<Vec2> {
    vec![
        rect.origin,
        Vec2::new(rect.origin.x + rect.size.w, rect.origin.y),
        Vec2::new(rect.origin.x + rect.size.w, rect.origin.y + rect.size.h),
        Vec2::new(rect.origin.x, rect.origin.y + rect.size.h),
    ]
}
