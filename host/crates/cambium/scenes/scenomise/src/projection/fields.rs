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
