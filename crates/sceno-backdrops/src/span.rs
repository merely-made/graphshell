// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The placed span a disclosed occurrence names. Moved from `scenomise::span_edit`
//! with its `SceneInput` method, which cannot be implemented outside this crate.

use crate::template::Span;
use crate::{SceneFacts, SceneInput};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpanTarget {
    pub lane: u16,
    pub span: Span,
    pub extent: u32,
    pub ticks_per_unit: u16,
}

impl SceneInput {
    /// Only supplied, placed occurrences; expected roles/groups are not edit targets.
    /// This discloses geometry, not permission to mutate its source.
    #[must_use]
    pub fn span_target(&self, key: &str) -> Option<SpanTarget> {
        match &self.facts {
            SceneFacts::MetreGrid(grid) => grid.lines.iter().enumerate().find_map(|(row, line)| {
                (0..line.syllables.len())
                    .find(|i| key == format!("{}/syllable/{i}", line.key))
                    .and_then(|i| grid.placement(row, i))
                    .map(|span| SpanTarget {
                        lane: row as u16,
                        span,
                        extent: grid.extent(),
                        ticks_per_unit: 1,
                    })
            }),
            SceneFacts::TemplatedSequence(seq) => seq
                .occurrences
                .iter()
                .find(|o| o.key == key)
                .and_then(|o| o.placement.as_ref())
                .map(|p| SpanTarget {
                    lane: p.lane,
                    span: p.span,
                    extent: seq.template.extent,
                    ticks_per_unit: seq.template.ticks_per_unit,
                }),
            _ => None,
        }
    }
}
