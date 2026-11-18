// Copyright 2026 Mark Alan Boykin
// SPDX-License-Identifier: MPL-2.0

use crate::diagnostics::channels::{
    CHANNEL_UX_CONTRACT_WARNING, CHANNEL_UX_PRESENTATION_BOUNDS_MISSING,
    CHANNEL_UX_STRUCTURAL_VIOLATION, CHANNEL_UX_TREE_BUILD, CHANNEL_UX_TREE_SNAPSHOT_BUILT,
};
use crate::diagnostics::{DiagnosticEvent, StructuredPayloadField, emit_event};
use uxtree::lint::{Finding, FindingKind, Sink, SnapshotSize};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::{IngressLimits, diagnostic_channel, scoped_thread_sender};
    use accesskit::{Node, NodeId, Role, TreeId, TreeInfo, TreeUpdate};

    #[test]
    fn real_registry_sink_receives_all_five_channels_without_node_text() {
        let (sender, receiver) = diagnostic_channel(IngressLimits::default());
        let _guard = scoped_thread_sender(sender);
        let id = NodeId(1);
        let mut node = Node::new(Role::Window);
        node.set_label("private fixture");
        let tree = TreeUpdate {
            tree_id: TreeId::ROOT,
            tree: Some(TreeInfo::new(id)),
            nodes: vec![(id, node.clone()), (id, node)],
            focus: id,
        };
        let report = uxtree::lint::inspect(TreeId::ROOT, &[tree], |_| false, |_, _| false);
        assert!(receiver.try_drain(10).unwrap().packets.is_empty());
        report.send_to(&mut RegistrySink);
        let batch = receiver.try_drain(10).unwrap();
        let channels: Vec<_> = batch
            .packets
            .iter()
            .map(|packet| {
                let DiagnosticEvent::MessageSentStructured {
                    channel_id, fields, ..
                } = &packet.event
                else {
                    panic!("expected registry channel record");
                };
                assert!(
                    fields
                        .iter()
                        .all(|field| !field.value.contains("private fixture"))
                );
                *channel_id
            })
            .collect();
        assert_eq!(
            channels,
            [
                CHANNEL_UX_TREE_BUILD,
                CHANNEL_UX_STRUCTURAL_VIOLATION,
                CHANNEL_UX_CONTRACT_WARNING,
                CHANNEL_UX_PRESENTATION_BOUNDS_MISSING,
                CHANNEL_UX_TREE_SNAPSHOT_BUILT,
            ]
        );
    }
}
