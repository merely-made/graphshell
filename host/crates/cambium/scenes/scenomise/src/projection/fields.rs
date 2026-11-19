// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

use super::*;

/// A recipe's fields against what the dataset discloses and the viewers the
/// host declares (table projection plan, T1). Each issue names the field.
pub(super) fn field_issues(
    compiler: &ProjectionCompiler,
    definition: &ProjectionDefinition,
    dataset: &ProjectionDataset,
) -> Vec<CompileIssue> {
    let mut issues = Vec::new();
    for (index, field) in definition.fields.iter().enumerate() {
        if !dataset.fields.contains_key(&field.path) {
            issues.push(CompileIssue::new(
                format!("fields[{index}].path"),
                format!(
                    "the field {:?} reads {:?}, which this dataset does not disclose",
                    field.name, field.path
                ),
            ));
        }
        if !compiler.viewers.contains(field.viewer_id()) {
            issues.push(CompileIssue::new(
                format!("fields[{index}].viewer"),
                format!(
                    "the field {:?} asks for the viewer {:?}, which this host does not declare",
                    field.name,
                    field.viewer_id()
                ),
            ));
        }
    }
    issues
}

/// A recipe's sort, filter and grouping (table projection plan, T4, T5):
/// only an arrangement that places by order reads them; each sort or group
/// key names a disclosed field, once; each condition fits its field's kind.
pub(super) fn sort_issues(
    compiler: &ProjectionCompiler,
    definition: &ProjectionDefinition,
    dataset: &ProjectionDataset,
) -> Vec<CompileIssue> {
    let reading = &definition.reading;
    let mut issues = Vec::new();
    let ordered = resolve_target(compiler, definition)
        .is_some_and(|target| target.channels() == Some(ChannelUse::Order));
    for (slot, used, what) in [
        ("reading.sort", !reading.sort.is_empty(), "a sort"),
        ("reading.filter", !reading.filter.is_empty(), "a filter"),
        ("reading.group", !reading.group.is_empty(), "a grouping"),
    ] {
        if used && !ordered {
            issues.push(CompileIssue::new(
                slot,
                format!("this arrangement does not place by order, so it cannot read {what}"),
            ));
        }
    }
    key_issues(
        &mut issues,
        dataset,
        &reading.sort,
        "reading.sort",
        "the sort",
    );
    key_issues(
        &mut issues,
        dataset,
        &reading.group,
        "reading.group",
        "the grouping",
    );
    if let Err(refusals) = scenograph::filter::Matcher::new(&reading.filter, &dataset.fields) {
        issues.extend(
            refusals
                .into_iter()
                .map(|(index, why)| CompileIssue::new(format!("reading.filter[{index}]"), why)),
        );
    }
    issues
}

fn key_issues(
    issues: &mut Vec<CompileIssue>,
    dataset: &ProjectionDataset,
    keys: &[scenograph::SortKey],
    slot: &str,
    what: &str,
) {
    for (index, key) in keys.iter().enumerate() {
        if !dataset.fields.contains_key(&key.field) {
            issues.push(CompileIssue::new(
                format!("{slot}[{index}].field"),
                format!(
                    "{what} reads {:?}, which this dataset does not disclose",
                    key.field
                ),
            ));
        }
        if keys[..index]
            .iter()
            .any(|earlier| earlier.field == key.field)
        {
            issues.push(CompileIssue::new(
                format!("{slot}[{index}].field"),
                format!("{what} names {:?} twice", key.field),
            ));
        }
    }
}
