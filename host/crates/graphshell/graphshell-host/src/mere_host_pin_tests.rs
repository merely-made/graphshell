// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Engine split P8, probe 2 and the stale-intent check (Graphshell's engine
//! binding plan, E1 and E2): a scene built from a pinned view names that pin's
//! engine revision and keeps its graph after later edits, and an intent made
//! in a scene the engine has moved past comes back stale.

use super::*;
use crate::handlers::{OpenAddressV1, intent_id};
use crate::mere_host::{FIXTURE_DEVICE_TWO_ADDRESS, FIXTURE_PERSONA_ADDRESS, fixture_handlers};
use chirograph::IntentResult;
use muniment::MemoryBackend;
use pandect::access::AccessContext;

const HANDLER: &str = "system.default";

fn host() -> MereHost<MemoryBackend> {
    MereHost::empty(
        MemoryBackend::new(),
        SelectedPersonaRef {
            persona: FIXTURE_PERSONA_ADDRESS.to_string(),
            profile: "profile:graphshell-p8".to_string(),
        },
        fixture_handlers(),
        AccessContext {
            persona: FIXTURE_PERSONA_ADDRESS.to_string(),
            device: FIXTURE_DEVICE_TWO_ADDRESS.to_string(),
            at_ms: 3_000,
        },
    )
}

fn url(n: u128) -> String {
    format!("https://pin.test/{n}")
}

fn node(n: u128) -> CapturedDelta {
    CapturedDelta::ReplayAddNodeWithIdIfMissing {
        id: uuid::Uuid::from_u128(n).to_string(),
        url: url(n),
        position: [0.0, 0.0],
    }
}

/// An edit that does not come through this host: its counter does not move.
fn edit_elsewhere(host: &mut MereHost<MemoryBackend>, n: u128) {
    host.graph_session_mut()
        .apply_now(
            Author::person("another-writer").via("elsewhere"),
            vec![node(n)],
        )
        .expect("the other path's edit applies");
}

fn scene(host: &mut MereHost<MemoryBackend>) -> SceneSnapshot {
    let request = host.local_request();
    host.snapshot(request)
        .expect("the host builds a scene")
        .scene
}

fn named(revision: &pandect::GraphRevision) -> scenotime::EngineRevision {
    scenotime::EngineRevision {
        session: revision.session.as_uuid().to_string(),
        cursor: revision.cursor.0,
        digest: revision.digest.clone(),
    }
}

fn open(host: &MereHost<MemoryBackend>, scene: &SceneSnapshot, n: u128) -> IntentInvocation {
    IntentInvocation {
        session: host.session(),
        target: host
            .instance_for_address(&url(n))
            .expect("the node is in the disclosed scene"),
        observed_epoch: scene.epoch,
        observed_revision: scene.revision,
        intent: intent_id(HANDLER),
        payload: chirograph::wire::encode(&OpenAddressV1 {
            address: url(n),
            handler: HANDLER.to_string(),
        })
        .unwrap(),
    }
}

#[test]
fn a_pinned_scene_names_its_pin_and_keeps_its_graph_after_a_later_edit() {
    let mut host = host();
    host.apply_edits(vec![node(1)]).unwrap();
    let earlier = host.graph_session().journal().live_cursor();
    host.apply_edits(vec![node(2)]).unwrap();
    let cursor = host.graph_session().journal().live_cursor();
    let pin = host.graph_session().pin(cursor).unwrap();
    let identity = pin.revision_identity();
    assert!(host.show_pinned(pin).unwrap().is_none());

    let held = scene(&mut host);
    assert_eq!(held.engine_revision, Some(named(&identity)));
    assert_eq!(held.tables.items.len(), 2);

    // A later engine edit leaves the held view's scene as it was.
    edit_elsewhere(&mut host, 3);
    assert!(host.graph_session().journal().live_cursor() > cursor);
    let after = scene(&mut host);
    assert_eq!(after, held, "the pinned scene is unchanged by a later edit");

    // A scene built from a different cursor names that cursor and differs.
    let other = host.graph_session().pin(earlier).unwrap();
    let other_identity = other.revision_identity();
    host.show_pinned(other).unwrap();
    let from_earlier = scene(&mut host);
    assert_ne!(from_earlier.engine_revision, held.engine_revision);
    assert_eq!(from_earlier.engine_revision, Some(named(&other_identity)));
    assert_ne!(from_earlier.tables.items, held.tables.items);

    // A new live view names the live revision and shows the later edit.
    let dropped = host.follow_live().expect("a pin was shown");
    assert_eq!(dropped.cursor(), earlier);
    let live = scene(&mut host);
    let live_revision = host.graph_session().live_revision().unwrap();
    assert_eq!(live.engine_revision, Some(named(&live_revision)));
    assert_ne!(live.engine_revision, held.engine_revision);
    // The earlier pin had no node 2, so its slot was tombstoned there and
    // the live view gives it a new one within the epoch (D6).
    assert_eq!(live.active_item_count(), 3);
    assert_eq!(live.tables.items.len(), 4);
}

#[test]
fn a_pin_of_another_session_is_refused() {
    let mut host = host();
    host.apply_edits(vec![node(1)]).unwrap();
    let other = {
        let mut other = self::host();
        other.apply_edits(vec![node(1)]).unwrap();
        let cursor = other.graph_session().journal().live_cursor();
        other.graph_session().pin(cursor).unwrap()
    };
    assert!(matches!(
        host.show_pinned(other),
        Err(MereHostError::WrongSession)
    ));
    assert!(host.pinned().is_none());
}

#[test]
fn an_intent_from_a_view_the_engine_moved_past_is_stale_and_applies_from_a_current_view() {
    let mut host = host();
    host.apply_edits(vec![node(1), node(2)]).unwrap();

    // A pinned view at the live revision is current: its action applies.
    let cursor = host.graph_session().journal().live_cursor();
    host.show_pinned(host.graph_session().pin(cursor).unwrap())
        .unwrap();
    let first = scene(&mut host);
    assert_eq!(
        host.invoke(open(&host, &first, 1)).unwrap(),
        IntentResult::Accepted
    );

    // Pin the revision that action made, then edit the live graph elsewhere.
    let cursor = host.graph_session().journal().live_cursor();
    host.show_pinned(host.graph_session().pin(cursor).unwrap())
        .unwrap();
    let old = scene(&mut host);
    let old_action = open(&host, &old, 1);
    edit_elsewhere(&mut host, 3);
    let current = host.live_engine_revision().unwrap();
    assert_ne!(Some(&current), old.engine_revision.as_ref());
    assert!(
        current.cursor > cursor.0,
        "the other path moved the journal"
    );

    let refused = host.invoke(old_action.clone()).unwrap();
    assert_eq!(
        refused,
        IntentResult::stale_at_engine(old.epoch, old.revision, Some(current.clone()))
    );
    assert_eq!(host.access_history_for(&url(1)).unwrap().records.len(), 1);

    // The same action from a current view applies.
    host.follow_live();
    let fresh = scene(&mut host);
    assert_eq!(fresh.engine_revision, Some(current.clone()));
    assert!(
        fresh.revision > old.revision,
        "one counter value, one engine revision"
    );
    // The counter moved too; the reply still names the engine revision.
    assert_eq!(
        host.invoke(old_action).unwrap(),
        IntentResult::stale_at_engine(fresh.epoch, fresh.revision, Some(current.clone()))
    );
    assert_eq!(
        host.invoke(open(&host, &fresh, 1)).unwrap(),
        IntentResult::Accepted
    );
    assert_eq!(host.access_history_for(&url(1)).unwrap().records.len(), 2);
}
