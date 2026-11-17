// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

use super::*;
use uxtree::envelope::Stability;

#[test]
fn published_tree_has_finalized_envelopes_for_every_node_and_real_command() {
    let mut f = Fixture::new();
    f.producers
        .register(
            7,
            Structural {
                life: Rc::new(RefCell::new(ProducerChildLifetime::new())),
                shown: Rc::new(Cell::new(true)),
                action: true,
            },
            &[],
        )
        .unwrap();
    let projection = f.projection();
    let source = projection
        .publication_envelopes(
            &f.owner(),
            &WindowDom::document(&f.dom),
            &f.leaves,
            &f.producers,
        )
        .unwrap();
    let source_by_identity: Vec<_> = source.envelopes.values().cloned().collect();
    // Advance the process allocator so this also detects accidentally publishing
    // semantic source IDs when the two small numeric namespaces happen to align.
    let _ = f.prepare(&NativeIds::default());
    let mut host = A11yHost::new(|| {});
    let tree = f.publish(&mut host);
    assert_eq!(tree.nodes.len(), host.published_envelopes().len());
    let mut commands = 0;
    let mut structural_refs = 0;
    for (id, node) in &tree.nodes {
        let envelope = &host.published_envelopes()[id];
        assert_eq!(envelope.address.local, *id);
        assert_eq!(envelope.address.instance, f.owner());
        assert_eq!(envelope.stability, Stability::Life);
        let original = source_by_identity
            .iter()
            .find(|e| e.provenance == envelope.provenance)
            .unwrap();
        assert_ne!(envelope.address.local, original.address.local);
        if node.label() == Some("Pin") {
            assert_eq!(envelope.action_ids, ["pin"]);
            assert_eq!(
                envelope.domain_ref,
                Some(cambium_rootstock::SourceRef::new("fixture", "member"))
            );
            assert!(envelope.authored_key.is_none());
            commands += 1;
        } else {
            assert!(
                envelope.action_ids.is_empty(),
                "action kinds do not create command IDs"
            );
        }
        if node.label() == Some("Structural") {
            assert!(envelope.domain_ref.is_some());
            assert!(!host.requests.contains_key(id));
            structural_refs += 1;
        }
    }
    assert_eq!((commands, structural_refs), (1, 1));
    assert_eq!(
        host.envelopes[&f.slot_id(&host)].authored_key.as_deref(),
        Some("native-slot")
    );
    println!(
        "P2: {} native nodes, every envelope finalized; producer pin and domain preserved",
        tree.nodes.len()
    );
    f.marker.set(false);
    host.sync(
        &f.owner(),
        &WindowDom::document(&f.dom),
        &f.layout,
        &f.leaves,
        &f.producers,
        None,
        1.0,
    );
    assert!(
        host.published_envelopes().is_empty(),
        "ended owner clears metadata with routes"
    );
}
