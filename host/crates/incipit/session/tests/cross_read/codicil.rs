// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Codicils written by the kernel: a plain graph codicil and a profiled one
//! of the store case's graph, and a plain one of a two-node graph. A compose
//! takes its inputs' dates, so it writes the kernel's bytes exactly.

use super::*;
use eidetic::ManifestId;

fn manifest(value: &Value) -> ManifestId {
    serde_json::from_value(value.clone()).unwrap()
}

/// The codicils the kernel saved open through incipit's codicil hook as the
/// kernel read them; a re-save and a compose through incipit write the
/// kernel's manifests and bytes; a hook naming other ids refuses.
#[test]
fn graph_codicils_open_through_incipits_hook() {
    use incipit_session::graph_codicil as incipit;
    pollster::block_on(async {
        let mut store = store("codicil/input.bin").await;
        let kernel = expected("codicil");
        let (plain, profiled, other) = (
            manifest(&kernel["plain"]),
            manifest(&kernel["profiled"]),
            manifest(&kernel["other"]),
        );

        // Each reads as the kernel read it.
        let reads = kernel["reads"].as_array().unwrap();
        assert_eq!(reads.len(), 3);
        for read in reads {
            let id = manifest(&read["id"]);
            let loaded = incipit::load_profiled_graph_codicil_sealed(&mut store, None, id)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(serde_json::to_value(&loaded).unwrap(), read["read"]);
            let opened = incipit::open_codicil_as_session(&mut store, id)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(incipit_whole(&opened), read["opened"]);
        }
        let mut listed: Vec<String> = incipit::list_graph_codicils(&mut store)
            .await
            .unwrap()
            .into_iter()
            .map(|m| m.id.to_string())
            .collect();
        listed.sort();
        assert_eq!(json!(listed), kernel["listed"]);
        assert_eq!(listed.len(), 3);

        // A re-save of the plain thaw. Its snapshot is dated by the wall clock
        // when it is saved, so it matches the kernel's re-save up to that
        // second: the same codicil once dated alike, and the same manifest but
        // for the content address over it.
        let opened = incipit::open_codicil_as_session(&mut store, plain)
            .await
            .unwrap()
            .unwrap();
        let mut ours = MemoryBackend::new();
        let resaved = incipit::save_graph_codicil(
            &mut ours,
            &opened,
            RedactionPolicy::default(),
            Timestamp(9),
        )
        .await
        .unwrap();
        let mut theirs = kernel_written::store("codicil/resaved.bin").await;
        let read = incipit::load_profiled_graph_codicil_sealed(&mut ours, None, resaved)
            .await
            .unwrap()
            .unwrap();
        let kernel_read =
            incipit::load_profiled_graph_codicil_sealed(&mut theirs, None, manifest(&kernel["resaved"]))
                .await
                .unwrap()
                .unwrap();
        let kernel_read = serde_json::to_value(&kernel_read).unwrap();
        assert_eq!(
            dated_as_kernel(
                serde_json::to_value(&read).unwrap(),
                &kernel_read,
                "/snapshot/timestamp_secs"
            ),
            kernel_read
        );
        let unaddressed = |manifests: Vec<eidetic::BlobManifest>| -> Vec<Value> {
            manifests
                .into_iter()
                .map(|manifest| {
                    let mut manifest = serde_json::to_value(manifest).unwrap();
                    for field in ["id", "content_hash", "sources"] {
                        manifest.as_object_mut().unwrap().remove(field);
                    }
                    manifest
                })
                .collect()
        };
        assert_eq!(
            unaddressed(incipit::list_graph_codicils(&mut ours).await.unwrap()),
            unaddressed(incipit::list_graph_codicils(&mut theirs).await.unwrap())
        );
        assert_eq!(bytes(&ours).await.len(), held("codicil/resaved.bin").len());

        // A session opens from the profiled codicil through the sessions' hook.
        let thaw_store = copy_of(&store).await;
        let thawed = Sessions::new(thaw_store.clone())
            .open_codicil_checked(profiled, author(), |_| Ok(()))
            .await
            .unwrap();
        let thawed = Sessions::new(thaw_store)
            .open(thawed.session_id)
            .await
            .unwrap();
        assert_eq!(incipit_whole(thawed.graph()), kernel["thawed"]);

        // A compose over incipit names the manifest the kernel's compose of
        // the same codicils named, and writes the same bytes (D268), whether
        // redacted or checked.
        let mut ours = copy_of(&store).await;
        let composed = incipit::compose_graph_codicils(
            &mut ours,
            &[plain, other],
            RedactionPolicy::default(),
            Timestamp(11),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(json!(composed), kernel["composed"]);
        let checked = incipit::compose_graph_codicils_checked(
            &mut ours,
            &[profiled, other],
            Timestamp(12),
            |_| Ok(()),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(json!(checked), kernel["checked"]);
        assert_eq!(bytes(&ours).await, held("codicil/composed.bin"));
        // Control: the same codicils in the other order compose to another
        // manifest, so the comparison sees the inputs.
        let reversed = incipit::compose_graph_codicils(
            &mut ours,
            &[other, plain],
            RedactionPolicy::default(),
            Timestamp(11),
        )
        .await
        .unwrap()
        .unwrap();
        assert_ne!(reversed, composed);

        // Control: a hook naming other schema ids reads neither codicil.
        static ELSEWHERE: pandect::graph_codicil::CodicilHook<Graph> =
            pandect::graph_codicil::CodicilHook {
                graph_schema: b"mere.graph-snapshot/v5",
                profiled_schema: b"mere.graph-snapshot-profiled/v5",
                v3_profiled_schema: b"mere.graph-snapshot/v3x",
                v2_schema: b"mere.graph-snapshot/v2x",
                v1_schema: b"mere.graph-snapshot/v1x",
                redact: incipit_session::graph_codicil::redact,
            };
        for id in [plain, profiled] {
            let refused = pandect::graph_codicil::load_profiled_graph_codicil_sealed_with(
                &mut store, None, &ELSEWHERE, id,
            )
            .await;
            assert!(
                refused
                    .as_ref()
                    .is_err_and(|error| error.to_string().contains("is not a graph codicil")),
                "{refused:?}"
            );
        }
    });
}
