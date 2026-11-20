// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The remote board's mapping and sync, moved with it from the port.

use super::*;
use crate::live_endpoint::LiveEndpoint;
use graphshell_client::ClientState;

mod folds;

mod mapping {
    use super::*;
    use chirograph::{PresentationChange, ProjectionDiff, ProjectionSession, ProtocolVersion};
    use graphshell_endpoint::ProjectionSource;
    use scenotime::{Revision, SceneDiff};

    fn mount(endpoint: &mut LiveEndpoint) -> (ClientState, chirograph::ProjectionSession) {
        let snapshot = endpoint.snapshot(endpoint.request()).expect("a snapshot");
        let session = snapshot.session.clone();
        let mut client = ClientState::default();
        client.apply_snapshot(snapshot).expect("it applies");
        (client, session)
    }

    fn rename(client: &mut ClientState, session: &ProjectionSession, instance: u32, label: &str) {
        let mounted = client.mounted(session).unwrap();
        let key = mounted
            .presentation
            .bindings
            .iter()
            .find(|binding| binding.instance == InstanceId(instance))
            .unwrap()
            .key
            .clone();
        let mut offers = mounted.presentation.offers[&key].clone();
        offers[0].semantics.label = label.to_owned();
        client
            .apply_diff(&ProjectionDiff {
                version: ProtocolVersion::V1,
                session: session.clone(),
                scene: SceneDiff {
                    epoch: mounted.scene.epoch,
                    base: mounted.scene.revision,
                    revision: Revision(mounted.scene.revision.0 + 1),
                    operations: Vec::new(),
                },
                presentation: vec![PresentationChange::ReplaceOffers { key, offers }],
                status: None,
            })
            .unwrap();
    }

    #[test]
    fn the_live_board_maps_to_cards_at_their_slots() {
        let mut endpoint = LiveEndpoint::new();
        endpoint.append();
        let (client, session) = mount(&mut endpoint);
        let scene = board_scene(client.mounted(&session).unwrap());
        assert_eq!(scene.cards.len(), 2);
        assert!(scene.cards[0].lead && !scene.cards[1].lead);
        assert_eq!(scene.cards[0].slot, (0.0, 0.0));
        assert_eq!(scene.cards[1].slot, (140.0, 0.0));
        assert_eq!(scene.cards[0].site, "live.graphshell");
        assert_eq!(
            scene.cards[0].title, "Card 0",
            "a snapshot lists cards in index order"
        );
        assert_eq!(scene.cards[1].title, "Card 1");
        assert!(matches!(
            scene.cards[0].footprint,
            BoardFootprint::Rect { .. }
        ));
    }

    #[cfg(feature = "physics-engine")]
    #[test]
    fn semantic_geometry_matches_full_rectangles_after_pan_and_resize() {
        let mut endpoint = LiveEndpoint::new();
        endpoint.append();
        let (client, session) = mount(&mut endpoint);
        let mut remote = RemoteBoard::new();
        remote.sync_reading(client.mounted(&session), Some(1), Some("first"));
        let fit = BoardFit::new(24.0, 19.0, 21.0, 17.0);
        let (generation, first) = remote.semantic_geometry(640, 480, fit).unwrap();
        let original = remote.scene().card_rects(remote.board(), 640, 480, fit);
        for (width, height, fit) in [
            (330, 250, fit.camera(1.4, (-127.0, 38.0))),
            (901, 389, fit.framing_bounds().camera(0.7, (50.0, -68.0))),
        ] {
            let (current, transform) = remote.semantic_geometry(width, height, fit).unwrap();
            assert_eq!(current, generation);
            let ratio = transform.scale / first.scale;
            let dx = transform.origin.0 - ratio * first.origin.0;
            let dy = transform.origin.1 - ratio * first.origin.1;
            let full = remote
                .scene()
                .card_rects(remote.board(), width, height, fit);
            for ((id, title, before), (after_id, after_title, after)) in original.iter().zip(full) {
                assert_eq!((id, title), (&after_id, &after_title));
                for (expected, actual) in [
                    (before.x * ratio + dx, after.x),
                    (before.y * ratio + dy, after.y),
                    (before.width * ratio, after.width),
                    (before.height * ratio, after.height),
                ] {
                    assert!((expected - actual).abs() < 0.001, "{expected} vs {actual}");
                }
            }
        }
        assert!(!remote.tick());
        assert_eq!(
            remote.semantic_geometry(640, 480, fit).unwrap().0,
            generation
        );
        remote.board_mut().halt();
        assert_ne!(
            remote.semantic_geometry(640, 480, fit).unwrap().0,
            generation
        );
        assert!(
            remote
                .semantic_geometry(640, 480, fit.fixed_cards(1.0))
                .is_none()
        );
        assert!(
            remote
                .semantic_geometry(640, 480, fit.camera(f32::NAN, (0.0, 0.0)))
                .is_none()
        );
        remote.board_mut().offload(std::sync::Arc::new(|| {}));
        assert!(!remote.board().has_synchronous_positions());
        assert!(remote.semantic_geometry(640, 480, fit).is_none());
    }

    #[cfg(feature = "physics-engine")]
    #[test]
    fn semantic_geometry_revisions_are_unique_across_boards_and_remounts() {
        let mut endpoint = LiveEndpoint::new();
        let (client, session) = mount(&mut endpoint);
        let mounted = client.mounted(&session);
        let fit = BoardFit::new(24.0, 24.0, 24.0, 24.0);
        let mut first = RemoteBoard::new();
        first.sync_reading(mounted, Some(1), Some("same"));
        let original = first.semantic_geometry(640, 480, fit).unwrap().0;
        let mut second = RemoteBoard::new();
        second.sync_reading(mounted, Some(1), Some("same"));
        let independent = second.semantic_geometry(640, 480, fit).unwrap().0;
        first = RemoteBoard::new();
        first.sync_reading(mounted, Some(1), Some("same"));
        let replacement = first.semantic_geometry(640, 480, fit).unwrap().0;
        assert_eq!(
            BTreeSet::from([original, independent, replacement]).len(),
            3
        );
    }

    #[cfg(feature = "physics-engine")]
    #[test]
    fn semantic_geometry_invalidates_labels_membership_and_attachment() {
        let mut endpoint = LiveEndpoint::new();
        let (mut client, session) = mount(&mut endpoint);
        let mut remote = RemoteBoard::new();
        let identity = Some(session.0.as_str());
        remote.sync_reading(client.mounted(&session), Some(1), identity);
        let fit = BoardFit::new(24.0, 24.0, 24.0, 24.0);
        let before = remote.semantic_geometry(640, 480, fit).unwrap().0;
        rename(&mut client, &session, 0, "New title");
        assert!(remote.apply_label_changes(
            client.mounted(&session).unwrap(),
            identity,
            client.accepted_label_update(&session).unwrap(),
        ));
        let renamed = remote.semantic_geometry(640, 480, fit).unwrap().0;
        assert_ne!(renamed, before);
        remote.sync_reading(client.mounted(&session), Some(2), Some("other"));
        let reattached = remote.semantic_geometry(640, 480, fit).unwrap().0;
        assert_ne!(reattached, renamed);
        endpoint.append();
        let (next, next_session) = mount(&mut endpoint);
        remote.sync_reading(next.mounted(&next_session), Some(3), Some("other"));
        assert_eq!(remote.scene().cards.len(), 2);
        assert_ne!(
            remote.semantic_geometry(640, 480, fit).unwrap().0,
            reattached
        );
        remote.semantic_generation = u64::MAX;
        assert!(remote.semantic_geometry(640, 480, fit).is_none());
    }

    #[test]
    fn semantic_geometry_refuses_nonrectangular_and_transport_boards() {
        let mut endpoint = LiveEndpoint::new();
        let mut snapshot = endpoint.snapshot(endpoint.request()).unwrap();
        snapshot.scene.tables.items[0].as_mut().unwrap().footprint = sceno::Footprint::Point;
        let session = snapshot.session.clone();
        let mut client = ClientState::default();
        client.apply_snapshot(snapshot).unwrap();
        let fit = BoardFit::new(24.0, 24.0, 24.0, 24.0);
        let mut remote = RemoteBoard::new();
        remote.sync_reading(client.mounted(&session), Some(1), Some("first"));
        assert!(!remote.rectangular_cards);
        assert!(remote.semantic_geometry(640, 480, fit).is_none());
        let mut remote = RemoteBoard::with_transport(Box::new(
            cartography::canvas::UnavailableTransport::default(),
        ));
        remote.sync_reading(client.mounted(&session), Some(1), Some("first"));
        assert!(!remote.board().has_synchronous_positions());
        assert!(remote.semantic_geometry(640, 480, fit).is_none());
    }

    /// A board that runs in a Worker asks for its transport once, when it first
    /// holds a scene: a page that never joins a session starts no Worker.
    #[test]
    fn a_board_started_on_demand_asks_for_its_transport_at_the_first_scene() {
        use std::cell::Cell;
        use std::rc::Rc;
        let asked = Rc::new(Cell::new(0));
        let mut endpoint = LiveEndpoint::new();
        let (client, session) = mount(&mut endpoint);
        let counter = Rc::clone(&asked);
        let mut remote = RemoteBoard::when_needed(move || {
            counter.set(counter.get() + 1);
            Some(Box::new(
                cartography::canvas::UnavailableTransport::default(),
            ))
        });
        let stage = cartography::canvas::PhysicsChoice::default().into_spec();
        remote.sync(None, None, Some(&stage), Speed::REAL_TIME);
        assert_eq!(asked.get(), 0, "no scene, no Worker");
        remote.sync(
            client.mounted(&session),
            Some(1),
            Some(&stage),
            Speed::REAL_TIME,
        );
        assert_eq!(asked.get(), 1);
        assert_eq!(remote.board().len(), 1);
        remote.sync(
            client.mounted(&session),
            Some(2),
            Some(&stage),
            Speed::REAL_TIME,
        );
        assert_eq!(asked.get(), 1, "asked once");
    }

    #[test]
    fn the_board_resyncs_only_when_the_revision_moves() {
        let mut endpoint = LiveEndpoint::new();
        let (client, session) = mount(&mut endpoint);
        let mut remote = RemoteBoard::new();
        remote.sync(
            client.mounted(&session),
            Some(1),
            Some(&cartography::canvas::PhysicsChoice::default().into_spec()),
            Speed::REAL_TIME,
        );
        assert_eq!(remote.board().len(), 1);

        endpoint.append();
        let (client, session) = mount(&mut endpoint);
        // Same revision claimed: nothing is reconciled.
        remote.sync(
            client.mounted(&session),
            Some(1),
            Some(&cartography::canvas::PhysicsChoice::default().into_spec()),
            Speed::REAL_TIME,
        );
        assert_eq!(remote.board().len(), 1);
        remote.sync(
            client.mounted(&session),
            Some(2),
            Some(&cartography::canvas::PhysicsChoice::default().into_spec()),
            Speed::REAL_TIME,
        );
        assert_eq!(remote.board().len(), 2);
        assert_eq!(remote.scene().cards.len(), 2);
    }

    #[test]
    fn a_contiguous_label_update_keeps_the_scene_and_physics_in_place() {
        let mut endpoint = LiveEndpoint::new();
        endpoint.append();
        let (mut client, session) = mount(&mut endpoint);
        let identity = Some(session.0.as_str());
        let mut remote = RemoteBoard::new();
        remote.sync_scoped(
            client.mounted(&session),
            Some(endpoint.current_revision().0),
            identity,
            None,
            Speed::REAL_TIME,
        );
        remote.set_focus(Some("1"));
        let cards = remote.scene().cards.as_ptr();
        let positions = [remote.board().position("0"), remote.board().position("1")];

        rename(&mut client, &session, 0, "New title");
        let mounted = client.mounted(&session).unwrap();
        let update = client.accepted_label_update(&session).unwrap();
        let fit = BoardFit::new(24.0, 24.0, 24.0, 24.0);
        assert!(remote.title_change_visible(update, 640, 480, fit));
        assert!(!remote.title_change_visible(update, 640, 480, fit.camera(1.0, (0.0, -640.0))));
        assert!(!remote.apply_label_changes(mounted, Some("another-attachment"), update));
        assert_eq!(remote.scene().cards[0].title, "Card 0");
        assert!(remote.apply_label_changes(mounted, identity, update));
        assert!(!remote.apply_label_changes(mounted, identity, update));
        assert!(!remote.title_change_visible(update, 640, 480, fit));
        remote.sync_scoped(
            Some(mounted),
            Some(update.revision.0),
            identity,
            None,
            Speed::REAL_TIME,
        );
        assert_eq!(remote.scene().cards.as_ptr(), cards);
        assert_eq!(remote.scene().cards[0].title, "New title");
        assert_eq!(remote.scene().cards[1].title, "Card 1");
        assert!(remote.scene().cards[1].lead);
        assert_eq!(
            [remote.board().position("0"), remote.board().position("1")],
            positions,
        );
    }

    #[test]
    fn renaming_a_hidden_member_keeps_its_stand_in_title() {
        let mut endpoint = LiveEndpoint::new();
        endpoint.append();
        let mut snapshot = endpoint.snapshot(endpoint.request()).unwrap();
        snapshot.scene.tables.folds.push(Some(sceno::Fold {
            members: vec![InstanceId(0), InstanceId(1)],
            stand_in: StandIn::Member(InstanceId(0)),
            rule: None,
            boundary: None,
            label: None,
        }));
        let session = snapshot.session.clone();
        let identity = Some(session.0.as_str());
        let mut client = ClientState::default();
        client.apply_snapshot(snapshot).unwrap();
        let mut remote = RemoteBoard::new();
        remote.sync_scoped(
            client.mounted(&session),
            Some(endpoint.current_revision().0),
            identity,
            None,
            Speed::REAL_TIME,
        );

        rename(&mut client, &session, 1, "Hidden title");
        let mounted = client.mounted(&session).unwrap();
        let update = client.accepted_label_update(&session).unwrap();
        let fit = BoardFit::new(24.0, 24.0, 24.0, 24.0);
        assert!(!remote.title_change_visible(update, 640, 480, fit));
        assert!(remote.apply_label_changes(mounted, identity, update));
        assert_eq!(remote.scene().cards[0].labelled(), "Card 0 +1");

        rename(&mut client, &session, 0, "Stand-in title");
        let mounted = client.mounted(&session).unwrap();
        let update = client.accepted_label_update(&session).unwrap();
        assert!(remote.title_change_visible(update, 640, 480, fit));
        assert!(remote.apply_label_changes(mounted, identity, update));
        assert_eq!(remote.scene().cards[0].labelled(), "Stand-in title +1");
    }

    /// The board steps at the speed the viewer's dial chose, and a change of
    /// dial reaches it on the next sync. (Ruled 2026-10-04, "The viewer's own
    /// dial".)
    #[test]
    fn the_board_steps_at_the_viewers_chosen_speed() {
        let mut endpoint = LiveEndpoint::new();
        endpoint.append();
        let (client, session) = mount(&mut endpoint);
        let mounted = client.mounted(&session);
        let mut remote = RemoteBoard::new();
        let steps = |remote: &mut RemoteBoard, frames: u64| {
            let before = remote.board().pace().ticks;
            for _ in 0..frames {
                remote.tick();
            }
            remote.board().pace().ticks - before
        };
        remote.sync(
            mounted,
            Some(1),
            Some(&cartography::canvas::PhysicsChoice::default().into_spec()),
            Speed::REAL_TIME,
        );
        assert_eq!(steps(&mut remote, 4), 4, "1x: a tick a frame");
        remote.sync(
            mounted,
            Some(1),
            Some(&cartography::canvas::PhysicsChoice::default().into_spec()),
            Speed::from_factor(5.0),
        );
        assert_eq!(remote.speed(), Speed::from_factor(5.0));
        assert_eq!(steps(&mut remote, 4), 20, "the viewer's 5x");
        // Control: the same board back at the viewer's 1x runs a tick a frame.
        remote.sync(
            mounted,
            Some(1),
            Some(&cartography::canvas::PhysicsChoice::default().into_spec()),
            Speed::REAL_TIME,
        );
        assert_eq!(steps(&mut remote, 4), 4);
    }
}

mod scoped_session {
    use super::*;
    use graphshell_endpoint::{ProjectionCatalog, ProjectionSource};
    #[test]
    fn resident_reused_cards_take_new_owner_slots_and_stay_fitted_without_local_dynamics() {
        let mut endpoint = LiveEndpoint::new();
        for _ in 0..7 {
            endpoint.append();
        }
        let snapshot = endpoint
            .snapshot(endpoint.describe().projections[0].request.clone())
            .unwrap();
        let mut client = ClientState::default();
        client.apply_snapshot(snapshot.clone()).unwrap();
        let mut thaw = client.mounted(&snapshot.session).unwrap().clone();
        for item in thaw.scene.tables.items.iter_mut().flatten() {
            item.transform.translate.x += 6000.0;
            item.transform.translate.y += 3000.0;
        }
        let mut remote = RemoteBoard::new();
        remote.sync_scoped(
            Some(&thaw),
            Some(1),
            Some("thaw:spectral.default"),
            Some(&cartography::canvas::PhysicsChoice::default().into_spec()),
            Speed::REAL_TIME,
        );
        let mut original = thaw.clone();
        for (index, item) in original.scene.tables.items.iter_mut().flatten().enumerate() {
            item.transform.translate.x = (index % 4) as f32 * 180.0;
            item.transform.translate.y = (index / 4) as f32 * 120.0;
        }
        remote.sync_reading(Some(&original), Some(1), Some("original:grid.default"));
        for _ in 0..120 {
            remote.tick();
        }
        for card in &remote.scene().cards {
            assert_eq!(remote.board().position(&card.id), Some(card.slot));
        }
        assert!(!remote.is_settling());
        for (width, height) in [(1100, 762), (1400, 900), (420, 600)] {
            let fit = cartography::canvas::BoardFit::new(24.0, 24.0, 24.0, 24.0);
            let rects = remote
                .scene()
                .card_rects(remote.board(), width, height, fit);
            assert_eq!(rects.len(), 8);
            for (_, _, rect) in rects {
                assert!(rect.x >= 23.99 && rect.y >= 23.99);
                assert!(rect.x + rect.width <= width as f32 - 23.99);
                assert!(rect.y + rect.height <= height as f32 - 23.99);
            }
        }
        // An external same-layout edit can change slots at a new revision.
        original.scene.tables.items[0]
            .as_mut()
            .unwrap()
            .transform
            .translate
            .y = 240.0;
        remote.sync_reading(Some(&original), Some(2), Some("original:grid.default"));
        assert_eq!(remote.board().position("0"), Some((0.0, 240.0)));
    }
    #[test]
    fn epoch_and_attached_session_are_independent_of_revision_and_close_clears_the_board() {
        let mut endpoint = LiveEndpoint::new();
        let request = endpoint.describe().projections[0].request.clone();
        let snapshot = endpoint.snapshot(request).unwrap();
        let mut client = ClientState::default();
        client.apply_snapshot(snapshot.clone()).unwrap();
        let mut mounted = client.mounted(&snapshot.session).unwrap().clone();
        let mut board = RemoteBoard::new();
        board.sync_scoped(
            Some(&mounted),
            Some(1),
            Some("first"),
            None,
            Speed::REAL_TIME,
        );
        assert_eq!(board.board().len(), 1);
        endpoint.append();
        let snapshot = endpoint
            .snapshot(endpoint.describe().projections[0].request.clone())
            .unwrap();
        client.apply_snapshot(snapshot.clone()).unwrap();
        mounted = client.mounted(&snapshot.session).unwrap().clone();
        board.sync_scoped(
            Some(&mounted),
            Some(1),
            Some("second"),
            None,
            Speed::REAL_TIME,
        );
        assert_eq!(
            board.board().len(),
            2,
            "different attachment at the same epoch and revision replaces the canvas"
        );
        mounted.scene.epoch.0 += 1;
        mounted.scene.tables.items[1] = None;
        mounted.scene.tables.item_order[1] = None;
        board.sync_scoped(
            Some(&mounted),
            Some(1),
            Some("second"),
            None,
            Speed::REAL_TIME,
        );
        assert_eq!(
            board.board().len(),
            1,
            "epoch changes cannot reuse another canvas"
        );
        board.sync_scoped(None, None, None, None, Speed::REAL_TIME);
        assert_eq!(
            board.board().len(),
            0,
            "closed authority retains no drawn records"
        );
    }
}
