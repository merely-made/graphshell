// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

use std::time::SystemTime;

use crate::MereSessions;
use kernel::graph::apply::GraphDelta;
use mere::ContextCache;
use mere::acquire::page_claims::{KEPT_PAGE_CLAIMS_FACET, interpret};
use mere::model::resource::ResourceNode;
use muniment::MemoryBackend;
use oxrdf::Triple;
use pandect::Author;

use super::*;

#[test]
fn report_keeps_its_evidence_and_stale_actions_fail_after_a_durable_edit() {
    pollster::block_on(async {
        let backend = MemoryBackend::new();
        let sessions = MereSessions::new(backend.clone());
        let author = Author::user();
        let manifest = sessions.mint(author.clone(), None).await.unwrap();
        let mut session = sessions.open(manifest.session_id).await.unwrap();
        let source = NamedNode::new_unchecked("urn:fixture:kept-capture");
        let other = NamedNode::new_unchecked("urn:fixture:other-capture");
        let page = "https://example.test/field-notes";
        let seen = r##"{"@id":"#river","https://example.test/seen":"yes"}"##;
        let required = r##"{"@id":"#river","https://example.test/required":"yes"}"##;
        let initial = interpret(&ContextCache::new(), page, [(5, seen), (6, "{")]);
        let replacement = interpret(&ContextCache::new(), page, [(11, required)]);
        let now = SystemTime::now();
        session
            .admit_page_claims(author.clone(), &other, &replacement, now)
            .await
            .unwrap();
        let missing = session.pin(session.journal().live_cursor()).unwrap();
        let shapes = required_property_shapes();
        assert!(matches!(
            PinnedValidation::validate(missing, &source, &shapes),
            Err(PinnedValidationError::MissingSource)
        ));
        let stored = session
            .admit_page_claims(author.clone(), &source, &initial, now)
            .await
            .unwrap();
        let pin = session.pin(session.journal().live_cursor()).unwrap();
        let historic = session.pin(session.journal().live_cursor()).unwrap();
        let displayed = PinnedValidation::validate(pin, &source, &shapes).unwrap();
        let reopened = GraphSession::open(backend.clone(), manifest.session_id)
            .await
            .unwrap();
        require_current_revision(displayed.revision(), &reopened).unwrap();

        let mut wrong_digest = displayed.revision().clone();
        wrong_digest.digest = "different digest at the same cursor".into();
        assert!(matches!(
            require_current_revision(&wrong_digest, &reopened),
            Err(PinnedValidationError::Stale { .. })
        ));

        // Deliberately change this capture's live facet through the journal to
        // exercise the action fence with genuinely different source evidence.
        let resource_id = ResourceNode::for_term(source.as_str()).id();
        let mut record = session.graph().resource_record(resource_id).unwrap();
        let replacement = KeptPageClaims::new(source.clone(), &replacement);
        record
            .facets
            .iter_mut()
            .find(|facet| facet.facet == KEPT_PAGE_CLAIMS_FACET)
            .unwrap()
            .value_json = replacement.facet_value().to_string();
        session
            .apply_deltas(
                author,
                vec![GraphDelta::ReplaySetResourceRecordById {
                    resource_id,
                    record: Some(record),
                }],
            )
            .await
            .unwrap();
        let reopen = GraphSession::open(backend, manifest.session_id)
            .await
            .unwrap();
        let updated_pin = reopen.pin(reopen.journal().live_cursor()).unwrap();
        let updated = PinnedValidation::validate(updated_pin, &source, &shapes).unwrap();
        assert_eq!(updated.kept(), &replacement);
        assert!(updated.report().conforms);
        require_current_revision(updated.revision(), &reopen).unwrap();
        assert!(matches!(
            require_current_revision(displayed.revision(), &reopen),
            Err(PinnedValidationError::Stale { displayed: old, current })
                if old == *displayed.revision() && current == *updated.revision()
        ));

        let historical = PinnedValidation::validate(historic, &source, &shapes).unwrap();
        assert_eq!(historical.revision(), displayed.revision());
        let pinned_evidence = displayed.pin().kept_page_claims(&source).unwrap().unwrap();
        assert_eq!(pinned_evidence, stored);
        let evidence = historical.kept();
        let report = historical.report();
        assert_eq!(evidence, &stored);
        assert_eq!(evidence.claims.blocks[0].order, 5);
        assert_eq!(evidence.claims.diagnostics[0].order, 6);
        assert!(!report.conforms);
        let [finding] = report.findings.as_slice() else {
            panic!("Expected one finding");
        };
        assert_eq!(
            finding.path.as_ref().and_then(|path| path.predicate()),
            Some(&NamedNode::new_unchecked("https://example.test/required"))
        );
    });
}

fn required_property_shapes() -> Graph {
    mere::parse_turtle(
        r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <https://example.test/> .
ex:shape a sh:NodeShape ;
    sh:targetNode <https://example.test/field-notes#river> ;
    sh:property [ sh:path ex:required ; sh:minCount 1 ] .
"#,
        None,
    )
    .unwrap()
    .quads(View::Live)
    .map(|quad| Triple::new(quad.subject, quad.predicate, quad.object))
    .collect()
}
