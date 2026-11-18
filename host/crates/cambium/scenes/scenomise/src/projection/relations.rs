// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

use super::*;
use sceno::{RelationConstraint, RelationForm};
use scenograph::presentation::{Context, InputAvailability, PrimitiveKind, Scalar, ViewState};

pub(super) fn placed_forms(
    compiler: &ProjectionCompiler,
    definition: &ProjectionDefinition,
) -> Vec<RelationForm> {
    crate::arrangement::arrangement_capabilities(&definition.arrangement, compiler.registry())
        .map(|entry| entry.placed_forms)
        .unwrap_or_default()
}

pub(super) fn form_issues(
    form: RelationForm,
    allowed: &[RelationForm],
    field: &str,
    arrangement: &str,
) -> Vec<CompileIssue> {
    if form == RelationForm::Adjacency {
        return vec![CompileIssue::new(
            field,
            "relation-driven adjacency belongs to Crystal",
        )];
    }
    if form.is_placed() && !allowed.contains(&form) {
        return vec![CompileIssue::new(
            field,
            format!(
                "{arrangement} does not declare the {} relation form",
                form.id()
            ),
        )];
    }
    Vec::new()
}

pub(crate) fn route(
    projection: &mut CompiledProjection,
    dataset: &ProjectionDataset,
    relationships: &[DisclosedRelationship],
    selected: Option<&str>,
) -> Result<Vec<CompiledRelationship>, Vec<CompileIssue>> {
    let mut issues = relationship_disclosure_issues(dataset, relationships);
    if projection.score.generation != stable_generation(dataset) {
        issues.push(CompileIssue::new(
            "projection",
            "the projection was compiled from another dataset or revision",
        ));
    }
    if !issues.is_empty() {
        return Err(issues);
    }
    let mut ordered: Vec<_> = relationships.iter().collect();
    ordered.sort_by(|a, b| a.id.cmp(&b.id));
    let marks = resolve_forms(
        &projection.relation_rules,
        &projection.marks,
        &projection.placed_forms,
        relationships,
        selected,
    )?;
    let mut compiled = Vec::new();
    let mut constraints = Vec::new();
    let mut routed = Vec::new();
    for relationship in ordered {
        let endpoint = |id: &str| projection.instance_by_occurrence.get(id).copied();
        let (Some(from), Some(to)) = (
            endpoint(&relationship.from_occurrence),
            endpoint(&relationship.to_occurrence),
        ) else {
            issues.push(CompileIssue::new(
                format!("relationships.{}.endpoints", relationship.id),
                "the projection does not place this occurrence",
            ));
            continue;
        };
        let resolution = &marks[&relationship.id];
        let form = resolution.marks.relation.unwrap_or(RelationForm::Stroke);
        if form.is_placed() {
            constraints.push(RelationConstraint {
                id: relationship.id.clone(),
                from,
                to,
                form,
            });
        }
        routed.push(sceno::RoutedRelation {
            form,
            from,
            to,
            space: sceno::Scene::WORLD,
            points: Vec::new(),
            kind: Some(relationship.kind.clone()),
            weight: None,
            link_key: None,
            marks: sceno::LinkMarks::default(),
            strength: None,
        });
        compiled.push(CompiledRelationship {
            disclosure: relationship.clone(),
            from,
            to,
        });
    }
    if !issues.is_empty() {
        return Err(issues);
    }
    let had_forms = !projection.scene.relation_constraints.is_empty()
        || projection
            .scene
            .relations
            .iter()
            .any(|r| r.form != RelationForm::Stroke);
    let mut scene = if had_forms {
        projection.base_scene.clone()
    } else {
        projection.scene.clone()
    };
    if projection.scene.frame.as_ref().is_some_and(|frame| {
        matches!(frame.spec, sceno::Frame::Nested { .. })
            && projection.base_scene.frame.as_ref() == Some(frame)
    }) {
        scene.spaces.clone_from(&projection.scene.spaces);
    }
    let base_scene = scene.clone();
    scene.relations = routed;
    let assigned = matches!(
        projection.score.arrangement,
        ScenoArrangement::Partition(_) | ScenoArrangement::Mosaic(_)
    );
    let gap = match &projection.score.arrangement {
        ScenoArrangement::Partition(config) => config.gap,
        _ => 0.0,
    };
    crate::relation_forms::arrange(
        &projection.score,
        &projection.relation_registry,
        &mut scene,
        &constraints,
        assigned,
        gap,
    )?;
    projection.base_scene = base_scene;
    projection.scene = scene;
    projection.relation_marks = marks;
    Ok(compiled)
}

pub(crate) fn preflight(
    compiler: &ProjectionCompiler,
    definition: &ProjectionDefinition,
    relationships: &[DisclosedRelationship],
    selected: Option<&str>,
) -> Result<(), Vec<CompileIssue>> {
    if let Some(form) = definition.marks.relation {
        let issues = form_issues(
            form,
            &placed_forms(compiler, definition),
            "marks.relation",
            &definition.arrangement.kind,
        );
        if !issues.is_empty() {
            return Err(issues);
        }
    }
    resolve_forms(
        &compiler.presentation,
        &definition.marks,
        &placed_forms(compiler, definition),
        relationships,
        selected,
    )
    .map(|_| ())
}

fn resolve_forms(
    rules: &scenograph::presentation::RuleSet,
    defaults: &scenograph::MarkSlots,
    allowed: &[RelationForm],
    relationships: &[DisclosedRelationship],
    selected: Option<&str>,
) -> Result<BTreeMap<String, scenograph::ResolvedMarks>, Vec<CompileIssue>> {
    let availability = InputAvailability {
        states: [ViewState::Selected].into(),
        screen_size: false,
    };
    let mut marks = BTreeMap::new();
    let mut issues = Vec::new();
    for relationship in relationships {
        let context = Context {
            id: relationship.id.clone(),
            kind: PrimitiveKind::Relation,
            facts: [
                ("kind", &relationship.kind),
                ("label", &relationship.label),
                ("from_occurrence", &relationship.from_occurrence),
                ("to_occurrence", &relationship.to_occurrence),
                ("explanation", &relationship.explanation),
            ]
            .into_iter()
            .map(|(key, value)| (key.into(), Scalar::Text(value.clone())))
            .collect(),
            states: if selected == Some(relationship.id.as_str()) {
                [ViewState::Selected].into()
            } else {
                Default::default()
            },
            screen_size: None,
            previous_matches: Default::default(),
        };
        let resolution = match rules.resolve_marks(defaults, &context, &availability) {
            Ok(resolution) => resolution,
            Err(problems) => {
                issues.extend(problems.into_iter().map(|issue| {
                    CompileIssue::new(
                        format!(
                            "relationships.{}.presentation.{}",
                            relationship.id, issue.field
                        ),
                        issue.message,
                    )
                }));
                continue;
            },
        };
        let form = resolution.marks.relation.unwrap_or(RelationForm::Stroke);
        issues.extend(form_issues(
            form,
            allowed,
            &format!("relationships.{}.form", relationship.id),
            "the selected arrangement",
        ));

        marks.insert(relationship.id.clone(), resolution);
    }
    if issues.is_empty() {
        Ok(marks)
    } else {
        Err(issues)
    }
}
