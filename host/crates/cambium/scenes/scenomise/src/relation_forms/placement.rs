// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

use super::*;
use std::collections::{BTreeSet, VecDeque};

pub(super) fn order(
    score: &sceno::Score,
    registry: &crate::SolverRegistry,
    scene: &mut Scene,
    requests: &[RelationConstraint],
) -> Result<(), Vec<crate::projection::CompileIssue>> {
    let n = scene.items.len();
    let mut base: Vec<_> = (0..n).collect();
    if matches!(score.arrangement, sceno::Arrangement::Grid(_)) {
        base.sort_by(|&a, &b| {
            let a_at = scene.items[a].transform.translate;
            let b_at = scene.items[b].transform.translate;
            a_at.y
                .total_cmp(&b_at.y)
                .then(a_at.x.total_cmp(&b_at.x))
                .then(a.cmp(&b))
        });
    }
    if matches!(score.arrangement, sceno::Arrangement::Mosaic(_)) {
        base = scene
            .regions
            .iter()
            .filter(|region| region.label.is_some())
            .flat_map(|region| region.members.iter().map(|id| id.0 as usize))
            .collect();
    }
    let original_rank: BTreeMap<_, _> = base
        .iter()
        .enumerate()
        .map(|(rank, &id)| (id, rank))
        .collect();
    let mut successors = vec![BTreeSet::new(); n];
    let mut incoming = vec![0; n];
    let mut found = false;
    for request in requests.iter().filter(|r| r.form == RelationForm::Order) {
        found = true;
        let (from, to) = (request.from.0 as usize, request.to.0 as usize);
        if from >= n || to >= n || reachable(&successors, to, from) {
            continue;
        }
        if successors[from].insert(to) {
            incoming[to] += 1;
        }
    }
    if !found {
        return Ok(());
    }
    scene.arrangement_order = base.iter().map(|&id| InstanceId(id as u32)).collect();
    let fixed: BTreeSet<_> = scene
        .honored_holds
        .iter()
        .map(|hold| hold.instance.0 as usize)
        .collect();
    let mut ready: BTreeSet<_> = incoming
        .iter()
        .enumerate()
        .filter_map(|(i, degree)| (*degree == 0).then_some((original_rank[&i], i)))
        .collect();
    let mut ordered = Vec::with_capacity(n);
    for slot in 0..n {
        let original = base[slot];
        let next = if fixed.contains(&original) {
            ready.get(&(slot, original)).copied()
        } else {
            ready.iter().copied().find(|(_, id)| !fixed.contains(id))
        };
        let Some((rank, next)) = next else {
            return Ok(());
        };
        ready.remove(&(rank, next));
        ordered.push(next);
        for &to in &successors[next] {
            incoming[to] -= 1;
            if incoming[to] == 0 {
                ready.insert((original_rank[&to], to));
            }
        }
    }
    if matches!(score.arrangement, sceno::Arrangement::Grid(_)) {
        let slots = scene.items.clone();
        for (slot, &instance) in ordered.iter().enumerate() {
            scene.items[instance].transform.translate = slots[base[slot]].transform.translate;
        }
    } else {
        let mut reordered = score.clone();
        let mut score_indices: Vec<_> = (0..score.items.len()).collect();
        score_indices.sort_by_key(|&index| (score.items[index].ordinal, index));
        for (rank, &instance) in ordered.iter().enumerate() {
            reordered.items[score_indices[instance]].ordinal = rank as u32;
        }
        // Test the requested plan before allowing authored pins to replace its positions.
        reordered
            .holds
            .retain(|hold| hold.hold != sceno::Hold::Pinned);
        let plan = crate::solve_via(&reordered, registry).map_err(|error| {
            vec![crate::projection::CompileIssue {
                field: "marks.relation".into(),
                message: error.to_string(),
            }]
        })?;
        if plan.items.len() != n {
            return Ok(());
        }
        for (rank, &instance) in ordered.iter().enumerate() {
            if fixed.contains(&instance) {
                let before = scene.items[instance].transform.translate;
                let after = plan.items[rank].transform.translate;
                if (before.x - after.x).abs() > 1.0e-3 || (before.y - after.y).abs() > 1.0e-3 {
                    return Ok(());
                }
            }
        }
        for (rank, &instance) in ordered.iter().enumerate() {
            scene.items[instance].transform = plan.items[rank].transform;
            scene.items[instance].footprint = plan.items[rank].footprint.clone();
        }
        scene.regions = plan
            .regions
            .into_iter()
            .map(|mut region| {
                region.members = region
                    .members
                    .into_iter()
                    .map(|id| InstanceId(ordered[id.0 as usize] as u32))
                    .collect();
                region
            })
            .collect();
        if matches!(score.arrangement, sceno::Arrangement::Mosaic(_)) {
            // The solver's group regions already include configured group
            // order and the score's member order. Report that realized order.
            ordered = scene
                .regions
                .iter()
                .filter(|region| region.label.is_some())
                .flat_map(|region| region.members.iter().map(|id| id.0 as usize))
                .collect();
        }
    }
    scene.arrangement_order = ordered.into_iter().map(|i| InstanceId(i as u32)).collect();
    Ok(())
}

fn reachable(edges: &[BTreeSet<usize>], from: usize, to: usize) -> bool {
    let mut queue = vec![from];
    let mut seen = BTreeSet::new();
    while let Some(current) = queue.pop() {
        if current == to {
            return true;
        }
        if seen.insert(current) {
            queue.extend(edges[current].iter().copied());
        }
    }
    false
}

pub(super) fn forest(
    requests: &[RelationConstraint],
    n: usize,
) -> (Vec<Option<usize>>, BTreeMap<usize, String>) {
    let mut parents = vec![None; n];
    let mut conflicts = BTreeMap::new();
    for (index, request) in requests
        .iter()
        .enumerate()
        .filter(|(_, r)| r.form == RelationForm::Containment)
    {
        let (parent, child) = (request.from.0 as usize, request.to.0 as usize);
        if parent >= n || child >= n {
            conflicts.insert(index, "a requested occurrence is absent".into());
            continue;
        }
        let mut ancestor = Some(parent);
        let mut cycle = false;
        while let Some(node) = ancestor {
            if node == child {
                cycle = true;
                break;
            }
            ancestor = parents[node];
        }
        if cycle {
            conflicts.insert(index, "containment requests form a cycle".into());
        } else if parents[child].is_some_and(|previous| previous != parent) {
            conflicts.insert(
                index,
                "competing parents cannot share a partition cell".into(),
            );
        } else {
            parents[child] = Some(parent);
        }
    }
    (parents, conflicts)
}

pub(super) fn containment(
    score: &sceno::Score,
    scene: &mut Scene,
    requests: &[RelationConstraint],
    assigned: bool,
    gap: f32,
) {
    let n = scene.items.len();
    let (parents, _) = forest(requests, n);
    let mut children = vec![Vec::new(); n];
    for (child, parent) in parents.iter().enumerate() {
        if let Some(parent) = parent {
            children[*parent].push(child);
        }
    }
    let fixed: BTreeSet<_> = scene
        .honored_holds
        .iter()
        .map(|hold| hold.instance.0 as usize)
        .collect();
    let mut queue: VecDeque<_> = parents
        .iter()
        .enumerate()
        .filter_map(|(i, p)| p.is_none().then_some(i))
        .collect();
    while let Some(parent) = queue.pop_front() {
        let members = &children[parent];
        queue.extend(members.iter().copied());
        if members.is_empty() || members.iter().any(|child| fixed.contains(child)) {
            continue;
        }
        let Some(bounds) = item_bounds(scene, InstanceId(parent as u32)) else {
            continue;
        };
        if scene.items[parent].space != Scene::WORLD || scene.items[parent].layer == i16::MAX {
            continue;
        }
        let area = Rect::new(
            Vec2::new(bounds.origin.x + gap, bounds.origin.y + gap),
            Size2::new(bounds.size.w - 2.0 * gap, bounds.size.h - 2.0 * gap),
        );
        if area.size.w <= 0.0 || area.size.h <= 0.0 {
            continue;
        }
        let mut ordered_items: Vec<_> = score.items.iter().enumerate().collect();
        ordered_items.sort_by_key(|(index, item)| (item.ordinal, *index));
        let weights: Vec<_> = members
            .iter()
            .map(|&child| {
                ordered_items[child]
                    .1
                    .weight
                    .filter(|w| w.is_finite() && *w > 0.0)
                    .unwrap_or(1.0)
            })
            .collect();
        let tiles = crate::families::partition::squarify(&weights, area);
        let placements: Option<Vec<_>> = members
            .iter()
            .zip(tiles)
            .map(|(&child, tile)| {
                let item = &scene.items[child];
                let old = item.footprint.bounds()?;
                let size = if assigned { tile.size } else { old.size };
                if item.space != Scene::WORLD
                    || item.transform.scale != 1.0
                    || item.transform.rotate != 0.0
                    || size.w > tile.size.w
                    || size.h > tile.size.h
                {
                    return None;
                }
                let at = Vec2::new(
                    tile.origin.x + tile.size.w * 0.5,
                    tile.origin.y + tile.size.h * 0.5,
                );
                let placed = Rect::new(Vec2::new(at.x - size.w * 0.5, at.y - size.h * 0.5), size);
                let inside_groups = scene
                    .regions
                    .iter()
                    .filter(|region| region.members.contains(&InstanceId(child as u32)))
                    .filter_map(|region| region.contour.as_ref().and_then(Footprint::bounds))
                    .all(|region| contains(region, placed));
                inside_groups.then_some((child, at, size))
            })
            .collect();
        let Some(placements) = placements else {
            continue;
        };
        for (child, at, size) in placements {
            scene.items[child].transform.translate = at;
            if assigned {
                scene.items[child].footprint = Footprint::Rect { size };
            }
            scene.items[child].layer = scene.items[parent].layer + 1;
        }
    }
}
