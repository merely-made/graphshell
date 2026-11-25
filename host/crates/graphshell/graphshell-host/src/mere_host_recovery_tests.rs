// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Engine binding E3: a missed delta is recovered. The deltas in transit are
//! real: each is the diff between two of `MereHost`'s own builds after a
//! journaled edit. A resume is answered with a fresh keyframe, nothing
//! retained (engine split D206).

use chirograph::{
    Advance, DeltaRetention, PresentationChange, PresentationManifest, ProjectionDiff,
    ProjectionSession, ProjectionSnapshot, ResumeReply, ResumeRequest, keyframe_reply,
};
use scenotime::SceneOp;
use graphshell_client::{ClientState, DiffApplication, MountedScene, ResumeApplication};
use graphshell_endpoint::ProjectionSource;
use kernel::graph::{CapturedDelta, SemanticStatementSpec};
use muniment::MemoryBackend;
use pandect::access::AccessContext;

use crate::mere_host::{
    FIXTURE_DEVICE_TWO_ADDRESS, FIXTURE_PERSONA_ADDRESS, MereHost, SelectedPersonaRef,
    fixture_handlers,
};

const CITES: &str = "https://recovery.test/cites";

/// A `MereHost`, and the deltas between its builds that travel to its
/// clients. `DeltaRetention` here only makes the deltas in transit; a resume
/// is answered with a fresh build.
struct Producer {
    host: MereHost<MemoryBackend>,
    retention: DeltaRetention,
}

impl Producer {
    fn new(nodes: u128) -> Self {
        Self::with_retention(nodes, DeltaRetention::default())
    }

    fn with_retention(nodes: u128, retention: DeltaRetention) -> Self {
        let mut host = MereHost::empty(
            MemoryBackend::new(),
            SelectedPersonaRef {
                persona: FIXTURE_PERSONA_ADDRESS.to_string(),
                profile: "profile:graphshell-e3".to_string(),
            },
            fixture_handlers(),
            AccessContext {
                persona: FIXTURE_PERSONA_ADDRESS.to_string(),
                device: FIXTURE_DEVICE_TWO_ADDRESS.to_string(),
                at_ms: 3_000,
            },
        );
        host.apply_edits((1..=nodes).map(node).collect()).unwrap();
        let mut producer = Self { host, retention };
        assert_eq!(producer.advance(), Advance::Restarted);
        producer
    }

    fn session(&self) -> ProjectionSession {
        self.host.session()
    }

    fn build(&mut self) -> ProjectionSnapshot {
        let request = self.host.local_request();
        self.host.snapshot(request).unwrap()
    }

    fn advance(&mut self) -> Advance {
        let built = self.build();
        self.retention.advance(built)
    }

    /// Add node `n`, citing node `n - 1`: one journaled edit, then a build.
    fn edit(&mut self, n: u128) -> Advance {
        self.host.apply_edits(vec![node(n)]).unwrap();
        let graph = self.host.graph();
        let (from, to) = (key(graph, n), key(graph, n - 1));
        self.host
            .mutate_product_graph(|graph| {
                graph
                    .assert_semantic_statement(
                        from,
                        to,
                        SemanticStatementSpec {
                            predicate: CITES.into(),
                            ..Default::default()
                        },
                    )
                    .expect("the statement is asserted");
            })
            .unwrap();
        self.advance()
    }

    fn keyframe(&self) -> ProjectionSnapshot {
        self.retention.keyframe().unwrap().clone()
    }

    /// The answer to a resume: current, or a fresh keyframe (D206).
    fn resume(&mut self, request: &ResumeRequest) -> ResumeReply {
        let keyframe = self.build();
        keyframe_reply(request, keyframe)
    }

    /// What a client that resumes from `client`'s acknowledgement is sent.
    fn reply_for(&mut self, client: &ClientState) -> ResumeReply {
        self.resume(&client.resume_request(&self.session()).unwrap())
    }

    /// The latest retained delta: what an undisturbed client is sent next.
    fn latest(&self) -> ProjectionDiff {
        self.retention.retained().last().unwrap().clone()
    }
}

fn node(n: u128) -> CapturedDelta {
    CapturedDelta::ReplayAddNodeWithIdIfMissing {
        id: uuid::Uuid::from_u128(n).to_string(),
        url: format!("https://recovery.test/{n}"),
        position: [0.0, 0.0],
    }
}

fn key(graph: &kernel::graph::Graph, n: u128) -> kernel::graph::NodeKey {
    graph
        .get_node_key_by_id(uuid::Uuid::from_u128(n))
        .expect("the node is in the graph")
}

fn mounted_client(keyframe: ProjectionSnapshot) -> ClientState {
    let mut client = ClientState::default();
    client.apply_snapshot(keyframe).unwrap();
    client
}

fn mounted<'a>(client: &'a ClientState, session: &ProjectionSession) -> &'a MountedScene {
    client.mounted(session).unwrap()
}

/// The comparison every recovery is judged by: the whole scene, the engine
/// revision it names, and the presentation beside it.
fn assert_same_scene(held: &MountedScene, undisturbed: &MountedScene, what: &str) {
    assert_eq!(held.scene, undisturbed.scene, "{what}: scene");
    assert_eq!(
        bound(&held.presentation),
        bound(&undisturbed.presentation),
        "{what}: presentation"
    );
}

/// A manifest with its bindings in instance order: a keyframe and a manifest
/// that took binds list them in different orders, which mean nothing.
fn bound(manifest: &PresentationManifest) -> PresentationManifest {
    let mut manifest = manifest.clone();
    manifest.bindings.sort_by_key(|binding| binding.instance.0);
    manifest
}

fn applied(client: &mut ClientState, session: &ProjectionSession, reply: ResumeReply) {
    assert!(
        matches!(
            client.apply_resume(session, reply).unwrap(),
            ResumeApplication::Applied(_)
        ),
        "the reply applies"
    );
}

#[test]
fn a_dropped_delta_is_seen_before_the_next_applies_and_recovered_by_resume() {
    let mut producer = Producer::new(3);
    let session = producer.session();
    let mut undisturbed = mounted_client(producer.keyframe());
    let mut dropped = mounted_client(producer.keyframe());

    for n in 4..=8 {
        assert_eq!(producer.edit(n), Advance::Retained, "edit {n} is a delta");
        let delta = producer.latest();
        assert!(delta.scene.base_engine.is_some() && delta.scene.engine_revision.is_some());
        applied(
            &mut undisturbed,
            &session,
            ResumeReply::Diffs(vec![delta.clone()]),
        );
        if n == 5 {
            continue; // Lost in transit.
        }
        let before = mounted(&dropped, &session).clone();
        let acknowledged = dropped.resume_request(&session).unwrap();
        match dropped
            .apply_resume(&session, ResumeReply::Diffs(vec![delta]))
            .unwrap()
        {
            ResumeApplication::Applied(_) => assert_ne!(n, 6, "applied past the gap"),
            ResumeApplication::Resynchronize(request) => {
                assert_eq!(n, 6, "the gap is seen at the delta after it");
                assert_eq!(mounted(&dropped, &session), &before, "nothing applied");
                assert_eq!(request, acknowledged, "resumes from its acknowledgement");
                assert!(request.engine_revision.is_some(), "naming its identity");
                let reply = producer.resume(&request);
                assert!(
                    matches!(&reply, ResumeReply::Snapshot(_)),
                    "nothing is retained: a fresh keyframe"
                );
                applied(&mut dropped, &session, reply);
            },
            ResumeApplication::Current(_) => panic!("never current after an edit"),
        }
    }
    assert_same_scene(
        mounted(&dropped, &session),
        mounted(&undisturbed, &session),
        "recovered",
    );
    let keyframe = producer.keyframe();
    assert_eq!(mounted(&undisturbed, &session).scene, keyframe.scene);
    assert_eq!(
        mounted(&undisturbed, &session).scene.engine_revision,
        producer.build().scene.engine_revision,
        "the client names the engine revision the host now builds"
    );
}

#[test]
#[should_panic(expected = "unrecovered: scene")]
fn the_scene_comparison_catches_a_missed_delta() {
    // Positive control: the same comparison, against a client that missed a
    // delta and never resumed, fails.
    let mut producer = Producer::new(3);
    let session = producer.session();
    let mut undisturbed = mounted_client(producer.keyframe());
    let missed = mounted_client(producer.keyframe());
    producer.edit(4);
    let reply = producer.reply_for(&undisturbed);
    applied(&mut undisturbed, &session, reply);
    assert_same_scene(
        mounted(&missed, &session),
        mounted(&undisturbed, &session),
        "unrecovered",
    );
}

#[test]
fn a_reordered_a_duplicate_and_a_divergent_delta_are_each_refused() {
    let mut producer = Producer::new(3);
    let session = producer.session();
    let mut client = mounted_client(producer.keyframe());
    producer.edit(4);
    let first = producer.latest();
    producer.edit(5);
    let second = producer.latest();
    let unchanged = |client: &ClientState, before: &MountedScene, what: &str| {
        assert_eq!(mounted(client, &session), before, "{what} applied nothing");
    };

    // Reordered: the later delta first.
    let before = mounted(&client, &session).clone();
    let reply = ResumeReply::Diffs(vec![second.clone(), first.clone()]);
    assert!(matches!(
        client.apply_resume(&session, reply).unwrap(),
        ResumeApplication::Resynchronize(_)
    ));
    unchanged(&client, &before, "a reordered suffix");
    assert!(matches!(
        client.apply_diff(&second).unwrap(),
        DiffApplication::Resynchronize(_)
    ));
    unchanged(&client, &before, "a delta ahead of its base");

    // Duplicate: the same delta twice applies once.
    assert!(matches!(
        client.apply_diff(&first).unwrap(),
        DiffApplication::Applied(_)
    ));
    let once = mounted(&client, &session).clone();
    assert!(matches!(
        client.apply_diff(&first).unwrap(),
        DiffApplication::AlreadyApplied(_)
    ));
    unchanged(&client, &once, "a duplicate");
    // A duplicate of the held revision under another engine revision diverges.
    let mut impostor = first.clone();
    impostor.scene.engine_revision = second.scene.engine_revision.clone();
    assert!(matches!(
        client.apply_diff(&impostor).unwrap(),
        DiffApplication::Resynchronize(_)
    ));
    unchanged(&client, &once, "a divergent duplicate");

    // Divergent base: the right counter under another engine revision.
    let mut divergent = second.clone();
    divergent.scene.base_engine = first.scene.base_engine.clone();
    assert!(matches!(
        client.apply_diff(&divergent).unwrap(),
        DiffApplication::Resynchronize(_)
    ));
    unchanged(&client, &once, "a divergent base");
    // And the producer refuses a resume that names a divergent base.
    let mut request = client.resume_request(&session).unwrap();
    request.engine_revision = second.scene.engine_revision.clone();
    assert!(matches!(
        producer.resume(&request),
        ResumeReply::Snapshot(_)
    ));

    // The real next delta still applies.
    assert!(matches!(
        client.apply_diff(&second).unwrap(),
        DiffApplication::Applied(_)
    ));
    assert_eq!(mounted(&client, &session).scene, producer.keyframe().scene);
}

#[test]
fn a_node_keeps_its_instance_slot_across_edits() {
    // D6: the newest node is laid out first but appends a slot, so no older
    // node changes slot, source or binding, and its intents still route.
    let mut producer = Producer::new(20);
    let url = |n: u128| format!("https://recovery.test/{n}");
    let instance = |producer: &Producer, n: u128| producer.host.instance_for_address(&url(n));
    let before: Vec<_> = (1..=20).map(|n| instance(&producer, n)).collect();
    assert!(before.iter().all(Option::is_some));
    for n in 21..=24 {
        assert_eq!(producer.edit(n), Advance::Retained);
        let delta = producer.latest();
        let count = |what: fn(&SceneOp) -> bool| delta.scene.operations.iter().filter(|op| what(op)).count();
        assert_eq!(count(|op| matches!(op, SceneOp::UpdateSource { .. })), 0);
        assert_eq!(count(|op| matches!(op, SceneOp::AddItem { .. })), 1);
        let bound: Vec<_> = delta
            .presentation
            .iter()
            .filter_map(|change| match change {
                PresentationChange::Bind(binding) => Some(binding.instance),
                _ => None,
            })
            .collect();
        assert_eq!(bound, [instance(&producer, n).unwrap()], "only the new node binds");
    }
    assert_eq!((1..=20).map(|n| instance(&producer, n)).collect::<Vec<_>>(), before);

    // A node that leaves tombstones its slot; the others keep theirs.
    let gone = uuid::Uuid::from_u128(5).to_string();
    producer
        .host
        .apply_edits(vec![CapturedDelta::ReplayRemoveNodeById { node_id: gone }])
        .unwrap();
    assert_eq!(producer.advance(), Advance::Retained);
    let delta = producer.latest();
    assert!(delta.scene.operations.contains(&SceneOp::TombstoneItem {
        index: before[4].unwrap(),
    }));
    assert_eq!(instance(&producer, 5), None);
    let kept: Vec<_> = (1..=20).filter(|&n| n != 5).map(|n| instance(&producer, n)).collect();
    let was: Vec<_> = (1..=20).filter(|&n| n != 5).map(|n| before[n as usize - 1]).collect();
    assert_eq!(kept, was);
    let keyframe = producer.keyframe();
    assert!(keyframe.scene.tables.items[before[4].unwrap().0 as usize].is_none());
    assert_eq!(keyframe.scene, producer.build().scene, "the build and the retained scene agree");
}
