// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Compile a contextual input with the same projection compiler as the main view.
use std::collections::BTreeMap;

use sceno::SourceRef;
use scenograph::{
    PublicSourceRevision, SourceBinding,
    contexts::{CommandChoice, ContextScene},
    dataset::{ProjectionDataset, ProjectionFieldType, ProjectionOccurrence, ProjectionValue},
};

use crate::projection::{CompileIssue, CompiledProjection, ProjectionCompiler};

pub fn compile(
    compiler: &ProjectionCompiler,
    context: &ContextScene,
    dataset: &ProjectionDataset,
) -> Result<CompiledProjection, Vec<CompileIssue>> {
    let mut bound = (*context.recipe).clone();
    bound.source = dataset.source.clone();
    bound.provenance.source_revision = Some(dataset.revision.clone());
    compiler.compile(&bound, dataset)
}

/// Presentation data for a command the host can offer at the time of reading.
/// The invoking host retains the route and its authorization.
#[derive(Clone, Debug)]
pub struct CommandDisclosure {
    pub id: String,
    pub label: String,
    pub shortcut: Option<String>,
    pub description: String,
    pub disabled_reason: Option<String>,
}

pub fn commands(
    source: SourceBinding,
    revision: PublicSourceRevision,
    available: &[CommandDisclosure],
    choices: Option<&[CommandChoice]>,
) -> ProjectionDataset {
    use ProjectionFieldType::{Boolean, Number, Text};
    let rows = choices.map_or_else(
        || {
            available
                .iter()
                .map(|command| (command, None))
                .collect::<Vec<_>>()
        },
        |choices| {
            choices
                .iter()
                .filter_map(|choice| {
                    available
                        .iter()
                        .find(|command| command.id == choice.id)
                        .map(|c| (c, Some(choice)))
                })
                .collect()
        },
    );
    ProjectionDataset {
        source,
        revision,
        fields: [
            ("occurrence_id", Text),
            ("label", Text),
            ("shortcut", Text),
            ("description", Text),
            ("disabled_reason", Text),
            ("enabled", Boolean),
            ("column", Number),
            ("order", Number),
        ]
        .into_iter()
        .map(|(name, kind)| (name.into(), kind))
        .collect(),
        occurrences: rows
            .into_iter()
            .enumerate()
            .map(|(rank, (c, choice))| {
                let label = choice
                    .and_then(|value| value.label.as_ref())
                    .unwrap_or(&c.label);
                let shortcut = choice
                    .and_then(|value| value.shortcut.as_ref())
                    .or(c.shortcut.as_ref());
                ProjectionOccurrence::new(
                    c.id.clone(),
                    SourceRef::new("context.commands", c.id.clone()),
                    BTreeMap::from([
                        ("occurrence_id".into(), ProjectionValue::Text(c.id.clone())),
                        ("label".into(), ProjectionValue::Text(label.clone())),
                        (
                            "shortcut".into(),
                            ProjectionValue::Text(shortcut.cloned().unwrap_or_default()),
                        ),
                        (
                            "description".into(),
                            ProjectionValue::Text(c.description.clone()),
                        ),
                        (
                            "disabled_reason".into(),
                            ProjectionValue::Text(c.disabled_reason.clone().unwrap_or_default()),
                        ),
                        (
                            "enabled".into(),
                            ProjectionValue::Boolean(c.disabled_reason.is_none()),
                        ),
                        ("column".into(), ProjectionValue::Number(0.0)),
                        ("order".into(), ProjectionValue::Number(rank as f64)),
                    ]),
                )
            })
            .collect(),
    }
}
