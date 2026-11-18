// Copyright 2026 Mark Alan Boykin
// SPDX-License-Identifier: MPL-2.0

use super::{Finding, FindingKind, Sink, SnapshotSize};
use registry::diagnostics::channels::{
    CHANNEL_UX_CONTRACT_WARNING, CHANNEL_UX_PRESENTATION_BOUNDS_MISSING,
    CHANNEL_UX_STRUCTURAL_VIOLATION, CHANNEL_UX_TREE_BUILD, CHANNEL_UX_TREE_SNAPSHOT_BUILT,
};
use registry::diagnostics::{DiagnosticEvent, StructuredPayloadField, emit_event};

/// Sends address-only findings and counts through the existing UX channels.
pub struct RegistrySink;

impl Sink for RegistrySink {
    fn begin(&mut self, size: SnapshotSize) {
        emit(
            CHANNEL_UX_TREE_BUILD,
            vec![field("trees", size.trees), field("nodes", size.nodes)],
        );
    }

    fn finding(&mut self, finding: &Finding) {
        let channel = match finding.kind {
            FindingKind::Structure(_) => CHANNEL_UX_STRUCTURAL_VIOLATION,
            FindingKind::MissingBounds => CHANNEL_UX_PRESENTATION_BOUNDS_MISSING,
            _ => CHANNEL_UX_CONTRACT_WARNING,
        };
        let mut fields = vec![
            field("tree_id", finding.tree.0),
            field("finding", format!("{:?}", finding.kind)),
        ];
        if let Some(node) = finding.node {
            fields.push(field("node_id", node.0));
        }
        emit(channel, fields);
    }

    fn end(&mut self, size: SnapshotSize, findings: usize) {
        emit(
            CHANNEL_UX_TREE_SNAPSHOT_BUILT,
            vec![
                field("trees", size.trees),
                field("nodes", size.nodes),
                field("findings", findings),
            ],
        );
    }
}

fn field(name: &'static str, value: impl ToString) -> StructuredPayloadField {
    StructuredPayloadField {
        name,
        value: value.to_string(),
    }
}

fn emit(channel_id: &'static str, fields: Vec<StructuredPayloadField>) {
    let byte_len = fields.iter().map(|f| f.name.len() + f.value.len()).sum();
    emit_event(DiagnosticEvent::MessageSentStructured {
        channel_id,
        byte_len,
        fields,
    });
}
