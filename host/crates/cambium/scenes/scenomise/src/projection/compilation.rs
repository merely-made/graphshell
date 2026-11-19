// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

use super::*;

pub(super) fn compile_inner(
    compiler: &ProjectionCompiler,
    definition: &ProjectionDefinition,
    dataset: &ProjectionDataset,
    selected_occurrence: Option<&str>,
    previous: Option<&CompiledProjection>,
    partition_focus: Option<&str>,
) -> Result<CompiledProjection, Vec<CompileIssue>> {
    let mut issues = validation_issues(compiler, definition, dataset);
    if let Some(selected) = selected_occurrence
        && !dataset
            .occurrences
            .iter()
            .any(|occurrence| occurrence.occurrence_id == selected)
    {
        issues.push(CompileIssue::new(
            "selected_occurrence",
            "the saved selected occurrence is absent from this dataset revision",
        ));
    }
    if !issues.is_empty() {
        return Err(issues);
    }

    let label_field = definition
        .encoding
        .label
        .as_ref()
        .and_then(|channel| field_name(channel, "encoding.label"))
        .expect("validated label field");
    let x_field = field_name(&definition.encoding.x, "encoding.x").expect("validated x field");
    let y_field = field_name(&definition.encoding.y, "encoding.y");
    let target = resolve_target(compiler, definition).expect("validated arrangement");
    let channels = target.channels();

    let occurrences: Vec<_> = if channels == Some(ChannelUse::Order) {
        reading_order(definition, dataset, x_field)
            .into_iter()
            .map(|index| &dataset.occurrences[index])
            .collect()
    } else {
        by_id(dataset)
            .into_iter()
            .map(|index| &dataset.occurrences[index])
            .collect()
    };
    let node_marks = super::marks::resolve(
        compiler,
        definition,
        &occurrences,
        selected_occurrence,
        target.extent(),
        target.default_mark(),
    )?;
    let measure = Measure {
        largest: compiler.sizes.card,
        count: occurrences.len(),
        spacing: definition.arrangement.spacing as f32,
        coordinates: (channels == Some(ChannelUse::Coordinate))
            .then(|| coordinate_extent(&occurrences, x_field, y_field.expect("validated y field")))
            .flatten(),
        columns: definition.fields.len().max(1),
    };
    let arrangement = match &target {
        Target::Builtin(family) => family
            .arrangement(&definition.arrangement.options, &measure)
            .map_err(option_issues)?,
        Target::Custom(id) => ScenoArrangement::Custom {
            id: id.clone(),
            config: serde_json::Value::Object(
                definition
                    .arrangement
                    .options
                    .iter()
                    .map(|(key, value)| (key.clone(), serde_json::Value::String(value.clone())))
                    .collect(),
            ),
        },
    };
    let grid_ranks = (channels == Some(ChannelUse::Cells)).then(|| {
        (
            dense_ranks(&occurrences, x_field),
            dense_ranks(&occurrences, y_field.expect("validated y field")),
        )
    });
    let mut score = Score::new(arrangement);
    score.generation = stable_generation(dataset);
    score.frame = definition.frame.clone();
    // Marks are resolved before measurement and placement.
    for (ordinal, occurrence) in occurrences.iter().enumerate() {
        let representation = node_marks[&InstanceId(ordinal as u32)]
            .marks
            .node
            .as_ref()
            .expect("resolved node mark")
            .detail
            .clone();
        let mut item = ScoreItem {
            source: occurrence.source.clone(),
            ordinal: ordinal as u32,
            footprint: Footprint::Rect {
                size: compiler.sizes.card,
            },
            representation,
            placement: Placement::Ordinal,
            layer: 0,
            visible: true,
            axis: None,
            embedding: None,
            weight: None,
        };
        disclose(
            &mut item,
            channels,
            occurrence,
            x_field,
            y_field,
            grid_ranks.as_ref(),
        );
        score.items.push(item);
    }
    let partition_focus = partition_focus.filter(|name| {
        let in_group = |item: &ScoreItem| match &item.axis {
            Some(sceno::AxisValue::Categorical(group)) => group.as_str() == *name,
            _ => false,
        };
        definition.interaction.zoom
            && matches!(score.arrangement, ScenoArrangement::Partition(_))
            && score.items.iter().any(in_group)
            && score.items.iter().any(|item| !in_group(item))
    });
    // Subdivision solvers own their footprint. A detail change can update
    // the placed item without resetting settled or authored geometry.
    let subdivision = matches!(
        score.arrangement,
        ScenoArrangement::Mosaic(_)
            | ScenoArrangement::Partition(_)
            | ScenoArrangement::Crystal(_)
            | ScenoArrangement::Table(_)
    );
    // Compare exact solver inputs, not revision hashes: source revisions can
    // change without affecting placement, and unchanged revisions cannot excuse
    // changed data. Ordered occurrence identity also protects repeated sources.
    let reusable = previous.filter(|old| {
        old.score.version == score.version
            && old.score.arrangement == score.arrangement
            && old.score.frame == score.frame
            && old.partition_focus() == partition_focus
            && (old.score.items == score.items
                || subdivision
                    && old.score.items.len() == score.items.len()
                    && old.score.items.iter().zip(&score.items).all(|(old, new)| {
                        old == new
                            || ScoreItem {
                                representation: new.representation.clone(),
                                ..old.clone()
                            } == *new
                    }))
            && old.score.holds == score.holds
            && occurrences.iter().enumerate().all(|(index, occurrence)| {
                old.occurrence_by_instance.get(&InstanceId(index as u32))
                    == Some(&occurrence.occurrence_id)
            })
    });
    let placement_reused = reusable.is_some();
    let scene = if let Some(old) = reusable {
        let constrained = !old.scene.relation_constraints.is_empty()
            || old
                .scene
                .relations
                .iter()
                .any(|r| r.form != sceno::RelationForm::Stroke);
        let mut scene = if constrained {
            old.base_scene.clone()
        } else {
            old.scene.clone()
        };
        if old.scene.frame.as_ref().is_some_and(|frame| {
            matches!(frame.spec, sceno::Frame::Nested { .. })
                && old.base_scene.frame.as_ref() == Some(frame)
        }) {
            scene.spaces.clone_from(&old.scene.spaces);
        }
        if subdivision {
            for (item, input) in scene.items.iter_mut().zip(&score.items) {
                item.representation = input.representation.clone();
            }
        }
        scene.generation = score.generation;
        scene
    } else {
        crate::registry::solve_via_with_focus(&score, &compiler.registry, partition_focus)
            .map_err(|error| vec![CompileIssue::new(
                if matches!(&error, crate::SolveError::Unplaceable { reason, .. } if reason.starts_with("frame:")) { "frame" } else { "arrangement.kind" },
                error.to_string())])?
    };
    let mut labels = HashMap::new();
    let mut occurrence_by_instance = HashMap::new();
    let mut instance_by_occurrence = BTreeMap::new();
    for (index, occurrence) in occurrences.into_iter().enumerate() {
        let instance = InstanceId(index as u32);
        labels.insert(
            instance,
            occurrence
                .values
                .get(label_field)
                .and_then(ProjectionValue::text)
                .expect("validated label value")
                .to_owned(),
        );
        occurrence_by_instance.insert(instance, occurrence.occurrence_id.clone());
        instance_by_occurrence.insert(occurrence.occurrence_id.clone(), instance);
    }
    let selected = selected_occurrence.and_then(|id| instance_by_occurrence.get(id).copied());
    Ok(CompiledProjection {
        base_scene: scene.clone(),
        relation_rules: compiler.presentation.clone(),
        relation_marks: BTreeMap::new(),
        placed_forms: super::relations::placed_forms(compiler, definition),
        relation_registry: compiler.registry.clone(),
        marks: definition.marks.clone(),
        node_marks,
        score,
        scene,
        labels,
        occurrence_by_instance,
        instance_by_occurrence,
        selected,
        partition_focus: partition_focus.map(str::to_owned),
        placement_reused,
    })
}

/// Give an item what its family reads from x and y.
pub(super) fn disclose(
    item: &mut ScoreItem,
    channels: Option<ChannelUse>,
    occurrence: &ProjectionOccurrence,
    x_field: &str,
    y_field: Option<&str>,
    grid_ranks: Option<&(BTreeMap<u64, i32>, BTreeMap<u64, i32>)>,
) {
    let x_number = number(occurrence, x_field);
    let x_text = occurrence
        .values
        .get(x_field)
        .and_then(ProjectionValue::text);
    let y_number = y_field.and_then(|field| number(occurrence, field));
    let xy = || {
        let (x, y) = (
            x_number.expect("validated x value"),
            y_number.expect("validated y value"),
        );
        (x, y)
    };
    match channels {
        Some(ChannelUse::Cells) => {
            let (x, y) = xy();
            let (x_ranks, y_ranks) = grid_ranks.expect("grid ranks for a cell family");
            // Dense numeric ranks preserve the two declared encodings without
            // allocating ninety-six empty rows for a 96 bpm card.
            item.placement = Placement::Cell {
                column: x_ranks[&dense_rank_key(x)],
                row: y_ranks[&dense_rank_key(y)],
            };
        },
        Some(ChannelUse::Coordinate) => {
            let (x, y) = xy();
            item.placement = Placement::Coordinate(Vec2::new(x as f32, y as f32));
        },
        Some(ChannelUse::Embedding) => {
            let (x, y) = xy();
            item.embedding = Some(Vec2::new(x as f32, y as f32));
        },
        Some(ChannelUse::NumericAxis | ChannelUse::IntegerAxis) => {
            item.axis = Some(AxisValue::Numeric(x_number.expect("validated x value")));
        },
        Some(ChannelUse::CategoricalAxis) => {
            item.axis = Some(AxisValue::Categorical(
                x_text.expect("validated x value").to_owned(),
            ));
        },
        // The ordinal carries the order.
        Some(ChannelUse::Order) => {},
        // A custom solver reads whatever x and y can disclose; its declared
        // requirements are checked when it solves.
        None => {
            item.axis = x_number
                .map(AxisValue::Numeric)
                .or_else(|| x_text.map(|text| AxisValue::Categorical(text.to_owned())));
            item.embedding = x_number
                .zip(y_number)
                .map(|(x, y)| Vec2::new(x as f32, y as f32));
            item.weight = y_number.map(|y| y as f32);
        },
    }
}

/// Dataset indices in occurrence id order.
fn by_id(dataset: &ProjectionDataset) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..dataset.occurrences.len()).collect();
    indices.sort_by(|&l, &r| {
        dataset.occurrences[l]
            .occurrence_id
            .cmp(&dataset.occurrences[r].occurrence_id)
    });
    indices
}

/// Dataset indices in the order an order-placing family places them: by the
/// reading's sort when it has one, by x otherwise, occurrence id breaking
/// ties. The table realization re-sorts through this same function.
pub(super) fn reading_order(
    definition: &ProjectionDefinition,
    dataset: &ProjectionDataset,
    x_field: &str,
) -> Vec<usize> {
    let ids = by_id(dataset);
    let by_x = [scenograph::SortKey::ascending(x_field)];
    let keys = if definition.reading.sort.is_empty() {
        &by_x[..]
    } else {
        &definition.reading.sort[..]
    };
    scenograph::sort::order(keys, ids.iter().map(|&index| &dataset.occurrences[index].values))
        .into_iter()
        .map(|rank| ids[rank])
        .collect()
}

/// The smallest and largest disclosed coordinate, in the source's units.
pub(super) fn coordinate_extent(
    occurrences: &[&ProjectionOccurrence],
    x_field: &str,
    y_field: &str,
) -> Option<(Vec2, Vec2)> {
    occurrences
        .iter()
        .filter_map(|occurrence| {
            Some(Vec2::new(
                number(occurrence, x_field)? as f32,
                number(occurrence, y_field)? as f32,
            ))
        })
        .fold(None, |extent, at| {
            Some(match extent {
                None => (at, at),
                Some((min, max)) => (
                    Vec2::new(min.x.min(at.x), min.y.min(at.y)),
                    Vec2::new(max.x.max(at.x), max.y.max(at.y)),
                ),
            })
        })
}

pub(super) fn option_issues(issues: Vec<OptionIssue>) -> Vec<CompileIssue> {
    issues
        .into_iter()
        .map(|issue| CompileIssue::new(issue.field, issue.message))
        .collect()
}

/// Builds the same dense ranks as the former per-occurrence closure, once for
/// each grid channel. Finite values have one `==` equivalence class per bit
/// pattern except signed zero, which deliberately shares a rank.
pub(super) fn dense_ranks(
    occurrences: &[&ProjectionOccurrence],
    field: &str,
) -> BTreeMap<u64, i32> {
    let mut values: Vec<_> = occurrences
        .iter()
        .map(|occurrence| number(occurrence, field).expect("validated numeric value"))
        .collect();
    values.sort_by(f64::total_cmp);
    values.dedup();
    values
        .into_iter()
        .enumerate()
        .map(|(rank, value)| (dense_rank_key(value), rank as i32))
        .collect()
}

pub(super) fn dense_rank_key(value: f64) -> u64 {
    if value == 0.0 { 0 } else { value.to_bits() }
}
