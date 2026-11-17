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
) -> Result<HashMap<InstanceId, ResolvedMarks>, Vec<CompileIssue>> {
    compiler.presentation.validate().map_err(issues)?;
    let state = compiler.sizes.representation_state();
    let availability = InputAvailability {
        states: [ViewState::Selected].into(),
        screen_size: true,
    };
    let mut resolved = HashMap::new();
    let mut errors = Vec::new();
    for (index, occurrence) in occurrences.iter().enumerate() {
        let mark =
            definition
                .marks
                .node
                .clone()
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
                .map(|(key, value)| {
                    let scalar = match value {
                        ProjectionValue::Text(text) => Scalar::Text(text.clone()),
                        ProjectionValue::Number(number) => Scalar::Number(*number),
                        ProjectionValue::Boolean(value) => Scalar::Bool(*value),
                    };
                    (key.clone(), scalar)
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
                    errors.push(CompileIssue::new(
                        format!("occurrences[{}].mark", context.id),
                        format!(
                            "{} imposes {} extent; {} mark refuses it",
                            definition.arrangement.kind,
                            extent.id(),
                            mark.label()
                        ),
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

fn issues(problems: Vec<scenograph::presentation::Issue>) -> Vec<CompileIssue> {
    problems
        .into_iter()
        .map(|issue| CompileIssue::new(format!("presentation.{}", issue.field), issue.message))
        .collect()
}
