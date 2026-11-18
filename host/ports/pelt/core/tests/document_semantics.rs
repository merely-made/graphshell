// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

#![cfg(feature = "genet")]

use inker::{
    DocumentA11yAction, DocumentA11yActionData, DocumentA11yNodeId, SessionSpawnRequest,
    SurfaceEngineRegistry,
};
use pelt_core::{PeltController, PeltControllerConfig};
use taproot::{SemanticLookup, SemanticTarget};
use uxtree::envelope::Stability;

struct Clock;
impl pelt_core::PeltClock for Clock {
    fn now_ms(&self) -> f64 {
        0.
    }
}

fn controller() -> PeltController<netrender::Scene> {
    let mut controller = PeltController::new(
        pelt_core::native::sessions(),
        SurfaceEngineRegistry::new(),
        PeltControllerConfig::from_request(
            inker::routing::ENGINE_GENET_LIVERY,
            SessionSpawnRequest::new("https://held.example/")
                .with_body("<!doctype html><input aria-label='Note' value='cedar'>")
                .with_content_type("text/html")
                .with_viewport(240, 120),
        ),
        Clock,
    )
    .unwrap();
    controller.try_frame(240, 120).unwrap();
    controller
}

#[test]
fn genet_refuses_a_stale_revision_from_the_host_lookup() {
    let mut host = controller();
    let observed = host.document_semantic_lookup().unwrap();
    let nodes = observed.nodes();
    let note = nodes
        .iter()
        .find(|node| node.role == Some("textbox") && node.name == Some("Note"))
        .unwrap();
    let SemanticTarget::Contributed(local) = note.target else {
        panic!("engine-local semantic target")
    };
    let target = DocumentA11yNodeId::new(local);
    let envelope = observed.envelope(target).unwrap();
    let revision = host.accessibility_projection().unwrap().revision();
    assert_eq!(envelope.address.local.0, local);
    assert_eq!(envelope.address.instance, host.session_identity());
    assert_eq!(envelope.provenance, host.session_identity());
    assert_eq!(envelope.stability, Stability::Scoped { revision });
    assert!(note.authored_key.is_none());
    let old_address = note.opaque_address.expect("host supplied an address");

    let write = |host: &mut PeltController<netrender::Scene>, lookup: &_, text: &str| {
        host.dispatch_document_semantic_action(
            lookup,
            target,
            DocumentA11yAction::SetValue,
            Some(DocumentA11yActionData::Value(text.into())),
        )
    };
    let value = |host: &PeltController<netrender::Scene>| {
        host.accessibility_projection()
            .unwrap()
            .node(target)
            .unwrap()
            .value
            .clone()
            .unwrap()
    };
    let mut other = controller();
    assert!(!write(&mut other, &observed, "wrong session"));
    assert!(write(&mut host, &observed, "maple"));
    host.try_frame(240, 120).unwrap();
    assert_eq!(value(&host), "maple");

    let current = host.document_semantic_lookup().unwrap();
    assert_ne!(
        current.envelope(target).unwrap().stability,
        envelope.stability
    );
    assert_ne!(
        current
            .nodes()
            .into_iter()
            .find(|node| node.target == note.target)
            .unwrap()
            .opaque_address,
        Some(old_address)
    );
    assert!(!write(&mut host, &observed, "stale write"));
    assert_eq!(value(&host), "maple");
    assert!(write(&mut host, &current, "birch"));
    host.try_frame(240, 120).unwrap();
    assert_eq!(value(&host), "birch");
}
