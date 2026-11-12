// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! A `mere.live-view/v1` record written before C7 (engine split D133, F5) still
//! loads: its view decodes as the arrangement and its camera, focus and mirror
//! keys are ignored. The fixture was written by the code of that day, a record
//! whose view held all three.

use super::*;
use muniment::{Backend, MemoryBackend};

#[test]
fn a_record_written_before_c7_loads_with_its_attention_ignored() {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/pre_c7_live_view.json"
    ))
    .unwrap();
    let dump: serde_json::Value = serde_json::from_str(&text).unwrap();
    pollster::block_on(async {
        let mut store = MemoryBackend::new();
        let mut payload = None;
        for (key, value) in dump["entries"].as_object().unwrap() {
            let hex = value.as_str().unwrap();
            let bytes: Vec<u8> = (0..hex.len())
                .step_by(2)
                .map(|at| u8::from_str_radix(&hex[at..at + 2], 16).unwrap())
                .collect();
            if key.starts_with("blob:") {
                payload = Some(bytes.clone());
            }
            store.put(key, &bytes).await.unwrap();
        }
        let payload = payload.expect("the record's payload blob");
        // Positive control: the stored record does carry attention.
        for word in ["camera", "focus", "mirror_tiles"] {
            assert!(
                payload.windows(word.len()).any(|w| w == word.as_bytes()),
                "the fixture holds {word}"
            );
        }

        let id = ManifestId::of_blob(&payload);
        let record = load_live_view_record(&mut store, id)
            .await
            .expect("a pre-C7 record loads")
            .expect("it is present");
        assert_eq!(record.view.strategy.as_deref(), Some("timeline.updated"));
        assert_eq!(record.view.hidden_relations.len(), 1);
        assert_eq!(record.view.folds.len(), 1);
        let again = serde_json::to_string(&record.view).unwrap();
        for word in ["camera", "focus", "mirror_tiles"] {
            assert!(!again.contains(word), "{word} is not arrangement");
        }
    });
}
