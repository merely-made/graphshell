// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

use super::*;
use sceno::{Rect, Size2, Transform2, Vec2};

fn frames() -> Vec<Frame> {
    let bounds = Rect::new(Vec2::new(30.0, 50.0), Size2::new(300.0, 200.0));
    vec![
        Frame::Bounded { bounds },
        Frame::Unbounded {
            origin: Vec2::new(40.0, -20.0),
        },
        Frame::Nested {
            bounds,
            transform: Transform2::translation(100.0, 200.0),
        },
        Frame::Flow {
            origin: Vec2::new(0.0, 40.0),
            width: 320.0,
        },
    ]
}

#[test]
fn frames_survive_drafts_bindings_json_and_toml() {
    for frame in frames() {
        let mut draft = crate::tests::valid_draft();
        draft.frame = Some(frame.clone());
        let definition = draft.to_definition().unwrap();
        assert_eq!(definition.frame, Some(frame.clone()));
        let mut authored = crate::tests::authored();
        authored.frame = Some(frame.clone());
        let name = authored.sources.keys().next().unwrap().clone();
        let bound = authored.bind(&name, None).unwrap();
        assert_eq!(bound.frame, Some(frame.clone()));
        assert_eq!(
            serde_json::from_slice::<AuthoredProjectionDefinition>(
                &serde_json::to_vec(&authored).unwrap()
            )
            .unwrap(),
            authored
        );
        assert_eq!(
            serde_json::from_slice::<ProjectionDefinition>(&definition.to_json_bytes().unwrap())
                .unwrap(),
            definition
        );
        #[cfg(feature = "toml")]
        {
            assert_eq!(
                AuthoredProjectionDefinition::from_toml(&authored.to_toml().unwrap()).unwrap(),
                authored
            );
            assert_eq!(
                ProjectionDefinition::from_toml(&bound.to_toml().unwrap()).unwrap(),
                bound
            );
        }
    }
}

#[test]
fn omitted_frames_keep_legacy_recipe_wire_and_invalid_frames_are_refused() {
    let draft = crate::tests::valid_draft();
    let definition = draft.to_definition().unwrap();
    let json = definition.to_json_bytes().unwrap();
    assert!(
        serde_json::from_slice::<serde_json::Value>(&json)
            .unwrap()
            .get("frame")
            .is_none()
    );
    assert!(
        serde_json::from_slice::<ProjectionDefinition>(&json)
            .unwrap()
            .frame
            .is_none()
    );
    for frame in [
        Frame::Flow {
            origin: Vec2::ZERO,
            width: 0.0,
        },
        Frame::Bounded {
            bounds: Rect::new(Vec2::ZERO, Size2::new(200.0, -10.0)),
        },
        Frame::Unbounded {
            origin: Vec2::new(f32::NAN, 0.0),
        },
        Frame::Nested {
            bounds: Rect::new(Vec2::ZERO, Size2::new(20.0, 20.0)),
            transform: Transform2 {
                scale: 0.0,
                ..Transform2::IDENTITY
            },
        },
    ] {
        let mut draft = draft.clone();
        draft.frame = Some(frame);
        assert!(
            draft
                .validate()
                .unwrap_err()
                .iter()
                .any(|issue| issue.field == "frame")
        );
    }
}
