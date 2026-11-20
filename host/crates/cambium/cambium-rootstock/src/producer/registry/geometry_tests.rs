// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

use super::*;
use layout_dom_api::LayoutDomMut;

#[test]
fn geometry_retention_cannot_cross_registration_or_dom_owner_lifetimes() {
    let mut dom = crate::ScriptedDom::new();
    let first = dom.document();
    let second = dom.create_text("other owner");
    let publication = |key, registration, owner, node, revision| ProducerLabelRevision {
        key,
        registration,
        owner,
        node,
        revision,
        geometry: Some(ProducerSemanticGeometry {
            revision: 7,
            scale: 1.0,
            origin: [0.0; 2],
        }),
    };
    let original = publication(7, 100, 40, first, 1);
    assert!(original.same_registration(&publication(7, 100, 40, first, 2)));
    for rebound in [
        publication(8, 100, 40, first, 1),
        publication(7, 101, 40, first, 1),
        publication(7, 100, 41, first, 1),
        publication(7, 100, 40, second, 1),
    ] {
        assert!(!original.same_registration(&rebound));
        assert!(
            ProducerRegistry::new()
                .producer_label_changes(&original, &rebound)
                .is_none()
        );
    }
}
