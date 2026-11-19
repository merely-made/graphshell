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

/// A recipe's sort: each key names a disclosed field, once, and only an
/// arrangement that places by order reads it (table projection plan, T4).
pub(super) fn sort_issues(
    compiler: &ProjectionCompiler,
    definition: &ProjectionDefinition,
    dataset: &ProjectionDataset,
) -> Vec<CompileIssue> {
    let sort = &definition.reading.sort;
    let mut issues = Vec::new();
    if sort.is_empty() {
        return issues;
    }
    let ordered = resolve_target(compiler, definition)
        .is_some_and(|target| target.channels() == Some(ChannelUse::Order));
    if !ordered {
        issues.push(CompileIssue::new(
            "reading.sort",
            "this arrangement does not place by order, so it cannot read a sort",
        ));
    }
    for (index, key) in sort.iter().enumerate() {
        if !dataset.fields.contains_key(&key.field) {
            issues.push(CompileIssue::new(
                format!("reading.sort[{index}].field"),
                format!(
                    "the sort reads {:?}, which this dataset does not disclose",
                    key.field
                ),
            ));
        }
        if sort[..index]
            .iter()
            .any(|earlier| earlier.field == key.field)
        {
            issues.push(CompileIssue::new(
                format!("reading.sort[{index}].field"),
                format!("the sort names {:?} twice", key.field),
            ));
        }
    }
    issues
}
