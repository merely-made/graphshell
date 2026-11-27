// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! pandect holds this crate's `Graph` through its generic session path
//! (engine split D211, D241, D244). pandect's default graph is mere-kernel's;
//! naming this package's type proves incipit-graph implements
//! `mere::session::SessionGraph` itself: edits record, flush and reopen with
//! one journal, one cursor and the same pin digest.

#![cfg(not(target_arch = "wasm32"))]

use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use euclid::default::Point2D;
use kernel::graph::apply::{GraphDelta, add_node, apply_graph_delta};
use kernel::graph::Graph;
use mere::model::author::Author;
use muniment::DirectoryBackend;
use muniment::journal::DigestChain;
use pandect::{GraphSession, MereSessions, SessionId};
use uuid::Uuid;

type Sessions = MereSessions<DirectoryBackend, Graph>;

async fn reopen(dir: &Path, id: SessionId) -> GraphSession<DirectoryBackend, Graph> {
    Sessions::new_with(DirectoryBackend::open(dir).unwrap())
        .open_with(id)
        .await
        .unwrap()
}

fn live_digest(session: &GraphSession<DirectoryBackend, Graph>) -> String {
    let pin = session.pin(session.journal().live_cursor()).unwrap();
    pin.digest().to_owned()
}

fn title_of(graph: &Graph, id: Uuid) -> String {
    let key = graph.get_node_key_by_id(id).expect("node");
    graph.get_node(key).expect("node").title.clone()
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            out.extend(walk(&path));
        } else {
            out.push(path);
        }
    }
    out
}

fn journals(dir: &Path) -> Vec<PathBuf> {
    walk(dir)
        .into_iter()
        .filter(|path| path.ends_with("journal.jsonl"))
        .collect()
}

fn copy_dir(from: &Path, to: &Path) {
    for entry in std::fs::read_dir(from).unwrap() {
        let path = entry.unwrap().path();
        let target = to.join(path.file_name().unwrap());
        if path.is_dir() {
            std::fs::create_dir_all(&target).unwrap();
            copy_dir(&path, &target);
        } else {
            std::fs::copy(&path, &target).unwrap();
        }
    }
}

/// Copy the session directory and rewrite the first journal line holding
/// `from` in the copy.
fn tamper(dir: &Path, from: &str, to: &str) -> tempfile::TempDir {
    let copy = tempfile::tempdir().unwrap();
    copy_dir(dir, copy.path());
    let journal = journals(copy.path()).pop().expect("a journal file");
    let text = std::fs::read_to_string(&journal).unwrap();
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
    let line = lines
        .iter()
        .position(|line| line.contains(from))
        .expect("a line to tamper");
    lines[line] = lines[line].replacen(from, to, 1);
    std::fs::write(&journal, lines.join("\n") + "\n").unwrap();
    copy
}

#[test]
fn pandect_reopens_an_incipit_graph_session_at_the_same_pin() {
    pollster::block_on(async {
        let dir = tempfile::tempdir().unwrap();
        let sessions = Sessions::new_with(DirectoryBackend::open(dir.path()).unwrap());
        let author = Author::person("incipit-graph-probe");
        let id = sessions
            .mint(author.clone(), None)
            .await
            .unwrap()
            .session_id;
        let mut session = sessions.open_with(id).await.unwrap();
        let (a, b) = (Uuid::from_u128(0xa01), Uuid::from_u128(0xa02));
        session
            .edit_now(author.clone(), |graph| {
                add_node(graph, Some(a), "https://probe.test/a".into(), Point2D::new(0.0, 0.0));
                add_node(graph, Some(b), "https://probe.test/b".into(), Point2D::new(1.0, 0.0));
            })
            .unwrap();
        session
            .edit_now(author, |graph| {
                let key = graph.get_node_key_by_id(a).unwrap();
                apply_graph_delta(
                    graph,
                    GraphDelta::SetNodeTitle {
                        key,
                        title: "probe-title".into(),
                    },
                );
            })
            .unwrap();
        session.flush(UNIX_EPOCH).await.unwrap();

        let cursor = session.journal().live_cursor();
        let entries = session.journal().entries().len();
        assert!(cursor.0 > 0, "the edits recorded entries");
        assert_eq!(cursor.0 as usize, entries, "one cursor at the journal's end");
        let digest = live_digest(&session);
        let chain = DigestChain::of(session.journal().log()).unwrap();
        assert_eq!(
            chain.at(cursor.0 as usize).map(|hash| hash.to_hex()),
            Some(digest.clone()),
            "the pin names every entry of the one journal"
        );
        println!("journal entries {entries}, cursor {}, digest {digest}", cursor.0);
        drop(session);
        drop(sessions);

        let files = journals(dir.path());
        assert_eq!(files.len(), 1, "one journal file: {files:?}");
        let reopened = reopen(dir.path(), id).await;
        assert_eq!(reopened.journal().live_cursor(), cursor);
        assert_eq!(reopened.journal().entries().len(), entries);
        assert_eq!(live_digest(&reopened), digest);
        assert_eq!(reopened.graph().node_count(), 2);
        assert_eq!(title_of(reopened.graph(), a), "probe-title");
        drop(reopened);

        // Positive control: the same reopen over a tampered journal line
        // names a different digest.
        let forged = tamper(dir.path(), "probe-title", "forged-title");
        let forged = reopen(forged.path(), id).await;
        let forged_digest = live_digest(&forged);
        println!("tampered digest {forged_digest}");
        assert_ne!(forged_digest, digest, "positive control: tampered journal");
        assert_eq!(title_of(forged.graph(), a), "forged-title");
    });
}
