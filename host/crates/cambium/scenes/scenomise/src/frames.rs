// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Frame capability checks and the geometry supplied to existing arrangements.

use crate::{
    SolveError, SolverRegistry,
    catalog::Family,
    projection::{CompileIssue, ItemSizes, ProjectionCompiler},
};
use sceno::{Arrangement, Frame, FrameKind, Rect, Score, Size2, Vec2};
use scenograph::ProjectionDefinition;
use std::borrow::Cow;

fn family(arrangement: &Arrangement) -> Option<Family> {
    Some(match arrangement {
        Arrangement::Spiral(_) => Family::Spiral,
        Arrangement::Grid(_) => Family::Grid,
        Arrangement::Geographic(_) => Family::Geographic,
        Arrangement::Hulls(_) => Family::Hulls,
        Arrangement::Stack(_) => Family::Stack,
        Arrangement::Penrose(_) => Family::Penrose,
        Arrangement::LSystem(_) => Family::LSystem,
        Arrangement::Timeline(_) => Family::Timeline,
        Arrangement::Kanban(_) => Family::Kanban,
        Arrangement::Embedded(_) => Family::Embedded,
        Arrangement::Radial(_) => Family::Radial,
        Arrangement::Partition(_) => Family::Partition,
        Arrangement::Setscape(_) => Family::Sets,
        Arrangement::Mosaic(_) => Family::Mosaic,
        Arrangement::Crystal(_) => Family::Crystal,
        Arrangement::Custom { .. } => return None,
    })
}

fn refuse(score: &Score, reason: impl Into<String>) -> SolveError {
    let id = family(&score.arrangement)
        .map(|f| f.id().to_owned())
        .unwrap_or_else(|| match &score.arrangement {
            Arrangement::Custom { id, .. } => id.clone(),
            _ => unreachable!(),
        });
    SolveError::Unplaceable {
        id,
        reason: format!("frame: {}", reason.into()),
    }
}

fn check_kind(frame: &Frame, accepted: &[FrameKind]) -> Result<(), String> {
    frame.validate().map_err(str::to_owned)?;
    if !accepted.contains(&frame.kind()) {
        return Err(format!(
            "the arrangement does not declare the {} frame",
            frame.kind().id()
        ));
    }
    Ok(())
}

pub(crate) fn definition_issues(
    compiler: &ProjectionCompiler,
    definition: &ProjectionDefinition,
) -> Vec<CompileIssue> {
    let Some(frame) = &definition.frame else {
        return Vec::new();
    };
    let accepted =
        crate::arrangement::arrangement_capabilities(&definition.arrangement, compiler.registry())
            .map(|entry| entry.frames)
            .unwrap_or_default();
    let mut issues = Vec::new();
    if let Err(message) = check_kind(frame, &accepted) {
        issues.push(CompileIssue::new(
            "frame",
            format!("{}: {message}", definition.arrangement.kind),
        ));
    }
    if matches!(frame, Frame::Bounded { .. }) {
        for key in ["width", "height"] {
            if definition.arrangement.options.contains_key(key) {
                issues.push(CompileIssue::new(
                    format!("arrangement.options.{key}"),
                    "an explicit frame supplies the rectangle; remove this legacy size option",
                ));
            }
        }
    }
    issues
}

pub(crate) fn validate_score(score: &Score, registry: &SolverRegistry) -> Result<(), SolveError> {
    let Some(frame) = &score.frame else {
        return Ok(());
    };
    let accepted = match family(&score.arrangement) {
        Some(family) => family.frames().to_vec(),
        None => {
            let Arrangement::Custom { id, .. } = &score.arrangement else {
                unreachable!()
            };
            registry
                .resolve(id)
                .ok_or_else(|| SolveError::Unregistered(id.clone()))?
                .capability()
                .frames
        },
    };
    check_kind(frame, &accepted).map_err(|message| refuse(score, message))
}

/// Keep the family structs' old fields as compatibility inputs for old Scores.
/// New frames supply the effective rectangle/origin on a working copy, so there
/// is one authority during placement and the authored Score remains untouched.
pub(crate) fn prepare(score: &Score) -> Result<Cow<'_, Score>, SolveError> {
    let Some(frame) = &score.frame else {
        return Ok(Cow::Borrowed(score));
    };
    if score.version < 6 {
        return Err(refuse(score, "explicit frames require score version 6"));
    }
    frame.validate().map_err(|message| refuse(score, message))?;
    if let Some(family) = family(&score.arrangement) {
        check_kind(frame, family.frames()).map_err(|message| refuse(score, message))?;
    }
    let mut effective = score.clone();
    match (&mut effective.arrangement, frame) {
        (Arrangement::Partition(config), Frame::Bounded { bounds }) => config.bounds = *bounds,
        (Arrangement::Setscape(config), Frame::Bounded { bounds }) => config.bounds = *bounds,
        (Arrangement::Hulls(config), Frame::Bounded { bounds }) => config.bounds = *bounds,
        (arrangement, Frame::Unbounded { origin }) => match arrangement {
            Arrangement::Spiral(c) => c.center = *origin,
            Arrangement::Grid(c) => c.origin = *origin,
            Arrangement::Geographic(c) => c.origin = *origin,
            Arrangement::Stack(c) => c.center = *origin,
            Arrangement::Penrose(c) => c.center = *origin,
            Arrangement::LSystem(c) => c.origin = *origin,
            Arrangement::Timeline(c) => c.origin = *origin,
            Arrangement::Kanban(c) => c.origin = *origin,
            Arrangement::Embedded(c) => c.origin = *origin,
            Arrangement::Radial(c) => c.center = *origin,
            Arrangement::Mosaic(c) => c.origin = *origin,
            Arrangement::Crystal(c) => c.origin = *origin,
            Arrangement::Custom { .. } => {},
            _ => unreachable!("frame capability was checked"),
        },
        (Arrangement::Grid(grid), Frame::Flow { origin, width }) => {
            configure_grid(score, grid, *origin, *width, None)?;
        },
        (Arrangement::Grid(grid), Frame::Nested { bounds, .. }) => {
            configure_grid(
                score,
                grid,
                bounds.origin,
                bounds.size.w,
                Some(bounds.size.h),
            )?;
        },
        (Arrangement::Custom { .. }, _) => {},
        _ => unreachable!("frame capability was checked"),
    }
    Ok(Cow::Owned(effective))
}

fn configure_grid(
    score: &Score,
    grid: &mut sceno::Grid,
    origin: Vec2,
    width: f32,
    height: Option<f32>,
) -> Result<(), SolveError> {
    let mut ordered: Vec<_> = score.items.iter().enumerate().collect();
    ordered.sort_by_key(|(index, item)| (item.ordinal, *index));
    let unheld: Vec<_> = ordered
        .iter()
        .enumerate()
        .filter(|(_, (_, item))| score.hold_for(&item.source).is_none())
        .collect();
    if unheld.is_empty() {
        grid.origin = origin;
        return Ok(());
    }
    let envelope = unheld
        .iter()
        .filter_map(|(_, (_, item))| item.footprint.bounds())
        .reduce(Rect::union)
        .unwrap_or_default();
    let step = Vec2::new(grid.cell.x + grid.gap, grid.cell.y + grid.gap);
    if !step.x.is_finite() || !step.y.is_finite() || step.x <= 0.0 || step.y <= 0.0 {
        return Err(refuse(
            score,
            "a grid needs finite positive row and column pitch",
        ));
    }
    let capacity = ((width - envelope.size.w) / step.x).floor() + 1.0;
    if !capacity.is_finite() || capacity < 1.0 {
        return Err(refuse(score, "the page width cannot hold a measured item"));
    }
    grid.columns = grid.columns.max(1).min(capacity as u32);
    grid.origin = Vec2::new(origin.x - envelope.origin.x, origin.y - envelope.origin.y);
    for (rank, (_, item)) in unheld {
        let (column, row) = match item.placement {
            sceno::Placement::Cell { column, row } => (column as f32, row as f32),
            _ => (
                (rank % grid.columns as usize) as f32,
                (rank / grid.columns as usize) as f32,
            ),
        };
        if column < 0.0
            || row < 0.0
            || column * step.x + envelope.size.w > width + 0.001
            || height.is_some_and(|h| row * step.y + envelope.size.h > h + 0.001)
        {
            return Err(refuse(
                score,
                "authored grid cells or measured content do not fit this frame",
            ));
        }
    }
    Ok(())
}

/// Generated ordered cells use the same width limit as the arrangement reader.
pub(crate) fn column_limit(definition: &ProjectionDefinition, sizes: ItemSizes) -> Option<u32> {
    let width = match definition.frame.as_ref()? {
        Frame::Flow { width, .. } => *width,
        Frame::Nested { bounds, .. } => bounds.size.w,
        _ => return None,
    };
    let cell = definition
        .arrangement
        .options
        .get("cell_width")
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(sizes.card.w);
    let step = cell + definition.arrangement.spacing as f32;
    Some((((width - sizes.card.w) / step).floor() + 1.0).max(1.0) as u32)
}

pub(crate) fn finish(mut scene: sceno::Scene, frame: Option<&Frame>) -> sceno::Scene {
    let mut space = sceno::Scene::WORLD;
    let has_geometry =
        !scene.items.is_empty() || !scene.regions.is_empty() || !scene.backdrops.is_empty();
    match frame {
        Some(Frame::Bounded { bounds }) => {
            scene.bounds = if has_geometry {
                scene.bounds.union(*bounds)
            } else {
                *bounds
            }
        },
        Some(Frame::Flow { origin, width }) => {
            let page = Rect::new(*origin, Size2::new(*width, 0.0));
            scene.bounds = if has_geometry {
                scene.bounds.union(page)
            } else {
                page
            };
        },
        Some(Frame::Nested { bounds, transform }) => {
            let local = if has_geometry {
                scene.bounds.union(*bounds)
            } else {
                *bounds
            };
            space = sceno::SpaceId(scene.spaces.len() as u32);
            scene.spaces.push(sceno::Space {
                parent: Some(sceno::Scene::WORLD),
                transform: *transform,
                name: Some("recipe frame".into()),
            });
            for item in &mut scene.items {
                item.space = space;
            }
            for region in &mut scene.regions {
                region.space = space;
            }
            let corners = [
                (0.0, 0.0),
                (local.size.w, 0.0),
                (0.0, local.size.h),
                (local.size.w, local.size.h),
            ];
            scene.bounds = corners
                .into_iter()
                .map(|(x, y)| {
                    Rect::new(
                        transform.apply(Vec2::new(local.origin.x + x, local.origin.y + y)),
                        Size2::default(),
                    )
                })
                .reduce(Rect::union)
                .unwrap();
        },
        Some(Frame::Unbounded { origin }) if scene.items.is_empty() && scene.regions.is_empty() => {
            scene.bounds = Rect::new(*origin, Size2::default());
        },
        _ => {},
    }
    scene.frame = frame.cloned().map(|spec| sceno::SceneFrame { spec, space });
    scene
}
