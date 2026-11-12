// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! A view is rebuilt from the dataset revision it cites (engine split P4,
//! D133): the journal cursor a view was set at names the graph and the view
//! state to rebuild from, after later edits and a reopen, with no format change.
//!
//! The test compares state: the graph at N and the decoded `ViewIntent` at N.
//! It does not compare the projected scene, because today's projection does
//! not read `ViewIntent`; the scene-level check is P8's probe (engine split
//! D4, D133).

use super::*;
use crate::mere_host::{
    FIXTURE_PERSONA_ADDRESS, MereHost, SelectedPersonaRef, SessionViewIntent, fixture_handlers,
};
use kernel::graph::CapturedDelta;
use muniment::MemoryBackend;
use pandect::access::AccessContext;
use crate::mere_host::FIXTURE_DEVICE_TWO_ADDRESS;
use pandect::ViewKey;

fn persona() -> SelectedPersonaRef {
    SelectedPersonaRef {
        persona: FIXTURE_PERSONA_ADDRESS.to_string(),
        profile: "profile:graphshell-a3".to_string(),
    }
}

fn node(n: u128) -> CapturedDelta {
    CapturedDelta::ReplayAddNodeWithIdIfMissing {
        id: uuid::Uuid::from_u128(n).to_string(),
        url: format!("https://cursor.test/{n}"),
        position: [0.0, 0.0],
    }
}

fn intent(strategy: &str, hidden: &[&str]) -> SessionViewIntent {
    SessionViewIntent {
        strategy: Some(strategy.to_string()),
        hidden_assertions: hidden.iter().map(|id| id.to_string()).collect(),
        ..SessionViewIntent::default()
    }
}

fn snapshot_json(graph: &Graph) -> Value {
    serde_json::to_value(graph.to_snapshot_at(0)).expect("a graph snapshot serializes")
}

#[test]
fn a_view_is_rebuilt_from_its_cursor_after_later_edits_and_a_reopen() {
    pollster::block_on(async {
        let backend = MemoryBackend::new();
        let context = AccessContext {
            persona: FIXTURE_PERSONA_ADDRESS.to_string(),
            device: FIXTURE_DEVICE_TWO_ADDRESS.to_string(),
            at_ms: 3_000,
        };
        let mut host = MereHost::empty(backend.clone(), persona(), fixture_handlers(), context.clone());
        let key = ViewKey::new("turnstone", "main").unwrap();

        // The view as set at cursor N, with the graph it was set over.
        host.through("turnstone".into(), |host| host.apply_edits(vec![node(1), node(2)]))
            .unwrap();
        let at_n = intent("phyllotaxis.default", &["assertion-a"]);
        host.set_view("turnstone", "main", at_n.clone()).await.unwrap();
        let cursor = host.graph_session().journal().live_cursor();
        let graph_at_n = snapshot_json(host.graph());
        host.persist(1_800_000_000).await.unwrap();

        // Edit past N: more graph, then a different view of it.
        host.through("turnstone".into(), |host| host.apply_edits(vec![node(3), node(4)]))
            .unwrap();
        let later = intent("spectral", &["assertion-a", "assertion-b"]);
        host.set_view("turnstone", "main", later.clone()).await.unwrap();
        host.persist(1_800_000_001).await.unwrap();
        let id = host.graph_session().id();
        assert!(host.graph_session().journal().live_cursor() > cursor);
        drop(host);

        // Reopen, then rebuild from the cursor.
        let reopened =
            MereHost::open_session(backend, id, persona(), fixture_handlers(), context)
                .await
                .unwrap();
        assert_eq!(reopened.view("turnstone", "main"), Some(later.clone()));
        assert_ne!(graph_at_n, snapshot_json(reopened.graph()), "the graph moved on");

        let session = reopened.graph_session();
        let rebuilt_graph = session.graph_at(cursor).expect("the cursor is within the journal");
        let rebuilt_view: SessionViewIntent = session
            .view_at(&key, cursor)
            .expect("the view had state at the cursor")
            .decode(VIEW_INTENT_SCHEMA)
            .expect("the view decodes as a view intent");
        assert_eq!(snapshot_json(&rebuilt_graph), graph_at_n);
        assert_eq!(rebuilt_view, at_n);
        assert_ne!(rebuilt_view, later);
    });
}
