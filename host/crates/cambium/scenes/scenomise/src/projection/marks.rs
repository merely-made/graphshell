// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

use super::*;
use scenograph::presentation::{Context, InputAvailability, PrimitiveKind, Scalar, ViewState};
use scenograph::{MarkSlots, ResolvedMarks};

pub(super) fn resolve(
    compiler: &ProjectionCompiler,
    definition: &ProjectionDefinition,
    occurrences: &[&ProjectionOccurrence],
    selected: Option<&str>,
    extent: Extent,
    family_mark: Option<NodeMark>,
) -> Result<HashMap<InstanceId, ResolvedMarks>, Vec<CompileIssue>> {
    compiler.presentation.validate().map_err(issues)?;
    let state = compiler.sizes.representation_state();
    let availability = InputAvailability {
        states: [ViewState::Selected].into(),
        screen_size: true,
    };
    let mut resolved = HashMap::new();
    let mut errors = Vec::new();
    // A host's explicit mark outranks the family's own; the class registry
    // answers only when neither speaks.
    let family_mark = family_mark.filter(|_| compiler.class_registry.is_some());
    if occurrences.is_empty() {
        let mark = definition
            .marks
            .node
            .as_ref()
            .or(family_mark.as_ref())
            .unwrap_or(&compiler.mark);
        if !mark.accepts(extent) {
            errors.push(extent_issue(definition, mark, extent, "marks.node"));
        }
    }
    for (index, occurrence) in occurrences.iter().enumerate() {
        let mark = definition
            .marks
            .node
            .clone()
            .or_else(|| family_mark.clone())
            .unwrap_or_else(|| match &compiler.class_registry {
                Some(registry) => registry
                    .resolve_classes(occurrence.classes.iter().map(String::as_str))
                    .mark(state, None),
                None => compiler.mark.clone(),
            });
        let defaults = MarkSlots {
            node: Some(mark),
            ..definition.marks.clone()
        };
        let context = Context {
            id: occurrence.occurrence_id.clone(),
            kind: PrimitiveKind::Node,
            facts: occurrence
                .values
                .iter()
                .filter_map(|(key, value)| {
                    // Rules test scalars only; references, dates and lists
                    // are not facts a rule can read yet.
                    let scalar = match value {
                        ProjectionValue::Text(text) => Scalar::Text(text.clone()),
                        ProjectionValue::Number(number) => Scalar::Number(*number),
                        ProjectionValue::Boolean(value) => Scalar::Bool(*value),
                        ProjectionValue::Reference(_)
                        | ProjectionValue::Date(_)
                        | ProjectionValue::DateTime(_)
                        | ProjectionValue::List(_) => return None,
                    };
                    Some((key.clone(), scalar))
                })
                .collect(),
            states: if selected == Some(occurrence.occurrence_id.as_str()) {
                [ViewState::Selected].into()
            } else {
                Default::default()
            },
            screen_size: Some(f64::from(compiler.sizes.card.w.min(compiler.sizes.card.h))),
            previous_matches: Default::default(),
        };
        match compiler
            .presentation
            .resolve_marks(&defaults, &context, &availability)
        {
            Ok(presentation) => {
                let mark = presentation.marks.node.as_ref().expect("node default");
                if let Err(message) = mark.validate() {
                    errors.push(CompileIssue::new(
                        format!("occurrences[{}].mark", context.id),
                        message,
                    ));
                } else if !mark.accepts(extent) {
                    errors.push(extent_issue(
                        definition,
                        mark,
                        extent,
                        &format!("occurrences[{}].mark", context.id),
                    ));
                }
                resolved.insert(InstanceId(index as u32), presentation);
            },
            Err(problems) => errors.extend(issues(problems)),
        }
    }
    if errors.is_empty() {
        Ok(resolved)
    } else {
        Err(errors)
    }
}

fn extent_issue(
    definition: &ProjectionDefinition,
    mark: &NodeMark,
    extent: Extent,
    field: &str,
) -> CompileIssue {
    let accepted: Vec<_> = mark.accepts.iter().map(|value| value.id()).collect();
    CompileIssue::new(
        field,
        format!(
            "{} imposes the {} extent; the {} mark accepts {}",
            definition.arrangement.kind,
            extent.id(),
            mark.label(),
            accepted.join(", ")
        ),
    )
}

fn issues(problems: Vec<scenograph::presentation::Issue>) -> Vec<CompileIssue> {
    problems
        .into_iter()
        .map(|issue| CompileIssue::new(format!("presentation.{}", issue.field), issue.message))
        .collect()
}
