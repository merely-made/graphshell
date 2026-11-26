// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Codicils written by the kernel.

use super::*;

/// A graph codicil and a profiled codicil the kernel saved open through
/// incipit's codicil hook as the kernel reads them; a re-save through
/// incipit names the kernel's manifest; a hook naming other ids refuses.
#[test]
fn graph_codicils_open_through_incipits_hook() {
    use incipit_session::graph_codicil as incipit;
    use pandect::graph_codicil as kernel_codicil;
    pollster::block_on(async {
        let backend = MemoryBackend::new();
        let (session_id, _, _) = kernel_written(&backend).await;
        let source = KernelSessions::new(backend.clone())
            .open(session_id)
            .await
            .unwrap();
        let graph = source.graph().clone();
        let mut store = MemoryBackend::new();
        let plain = kernel_codicil::save_graph_codicil(
            &mut store,
            &graph,
            RedactionPolicy::default(),
            Timestamp(7),
        )
        .await
        .unwrap();
        let profiled = kernel_codicil::save_profiled_graph_codicil_sealed(
            &mut store,
            None,
            ProfiledGraphCodicil {
                codicil: GraphCodicil {
                    snapshot: graph.to_snapshot(),
                    facets: graph.facets().clone(),
                    sessions: Vec::new(),
                },
                placement: Some(PlacementProfile::RecordedStrataV1),
            },
            RedactionPolicy::include_all(),
            Timestamp(8),
        )
        .await
        .unwrap();

        // Each reads as the kernel reads it.
        for id in [plain, profiled] {
            let read = incipit::load_profiled_graph_codicil_sealed(&mut store, None, id)
                .await
                .unwrap()
                .unwrap();
            let kernel_read =
                kernel_codicil::load_profiled_graph_codicil_sealed(&mut store, None, id)
                    .await
                    .unwrap()
                    .unwrap();
            assert_eq!(
                serde_json::to_value(&read).unwrap(),
                serde_json::to_value(&kernel_read).unwrap()
            );
            let opened = incipit::open_codicil_as_session(&mut store, id)
                .await
                .unwrap()
                .unwrap();
            let kernel_opened = kernel_codicil::open_codicil_as_session(&mut store, id)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(incipit_whole(&opened), kernel_whole(&kernel_opened));
        }
        let listed = |manifests: Vec<eidetic::BlobManifest>| {
            let mut ids: Vec<String> = manifests.into_iter().map(|m| m.id.to_string()).collect();
            ids.sort();
            ids
        };
        assert_eq!(
            listed(incipit::list_graph_codicils(&mut store).await.unwrap()),
            listed(
                kernel_codicil::list_graph_codicils(&mut store)
                    .await
                    .unwrap()
            )
        );
        assert_eq!(
            listed(incipit::list_graph_codicils(&mut store).await.unwrap()).len(),
            2
        );

        // A re-save of each thaw names the manifest the kernel's re-save names.
        let opened = incipit::open_codicil_as_session(&mut store, plain)
            .await
            .unwrap()
            .unwrap();
        let kernel_opened = kernel_codicil::open_codicil_as_session(&mut store, plain)
            .await
            .unwrap()
            .unwrap();
        let (mut ours, mut theirs) = (MemoryBackend::new(), MemoryBackend::new());
        let resaved = incipit::save_graph_codicil(
            &mut ours,
            &opened,
            RedactionPolicy::default(),
            Timestamp(9),
        )
        .await
        .unwrap();
        let kernel_resaved = kernel_codicil::save_graph_codicil(
            &mut theirs,
            &kernel_opened,
            RedactionPolicy::default(),
            Timestamp(9),
        )
        .await
        .unwrap();
        assert_eq!(resaved, kernel_resaved);
        assert_eq!(bytes(&ours).await, bytes(&theirs).await);

        // A session opens from the profiled codicil through the sessions' hook.
        let thawed = Sessions::new(store.clone())
            .open_codicil_checked(profiled, author(), |_| Ok(()))
            .await
            .unwrap();
        let thawed = Sessions::new(store.clone())
            .open(thawed.session_id)
            .await
            .unwrap();
        let kernel_thawed = KernelSessions::new(store.clone())
            .open_codicil_checked(profiled, author(), |_| Ok(()))
            .await
            .unwrap();
        let kernel_thawed = KernelSessions::new(store.clone())
            .open(kernel_thawed.session_id)
            .await
            .unwrap();
        assert_eq!(
            incipit_whole(thawed.graph()),
            kernel_whole(kernel_thawed.graph())
        );

        // A compose over incipit names the manifest the kernel's compose of
        // the same codicils names, and writes the same bytes (D268), whether
        // redacted or checked.
        let mut extra = KernelGraph::new();
        for (n, url) in [(1, "https://cross.test/1"), (9, "https://cross.test/extra")] {
            mere_kernel::graph::apply::add_node(
                &mut extra,
                Some(id(n)),
                url.into(),
                Default::default(),
            );
        }
        let other = kernel_codicil::save_graph_codicil(
            &mut store,
            &extra,
            RedactionPolicy::default(),
            Timestamp(10),
        )
        .await
        .unwrap();
        let (mut ours, mut theirs) = (copy_of(&store).await, copy_of(&store).await);
        let composed = incipit::compose_graph_codicils(
            &mut ours,
            &[plain, other],
            RedactionPolicy::default(),
            Timestamp(11),
        )
        .await
        .unwrap()
        .unwrap();
        let kernel_composed = kernel_codicil::compose_graph_codicils(
            &mut theirs,
            &[plain, other],
            RedactionPolicy::default(),
            Timestamp(11),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(composed, kernel_composed);
        let checked = incipit::compose_graph_codicils_checked(
            &mut ours,
            &[profiled, other],
            Timestamp(12),
            |_| Ok(()),
        )
        .await
        .unwrap()
        .unwrap();
        let kernel_checked = kernel_codicil::compose_graph_codicils_checked(
            &mut theirs,
            &[profiled, other],
            Timestamp(12),
            |_| Ok(()),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(checked, kernel_checked);
        assert_eq!(bytes(&ours).await, bytes(&theirs).await);
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
