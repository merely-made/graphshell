// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The backdrop studies shipped with the canvas: label and scene-document JSON,
//! for hosts that offer them as samples (engine split D119).

/// The seven sample backdrops, in the order a host lists them.
pub const SAMPLES: [(&str, &str); 7] = [
    ("Metre grid", include_str!("../../../backdrops/metre.json")),
    ("Progression in C", include_str!("../../../backdrops/templates/progression.json")),
    ("Progression in G", include_str!("../../../backdrops/templates/progression-g.json")),
    ("Regular arrangement", include_str!("../../../backdrops/templates/arrangement-regular.json")),
    ("Varied arrangement", include_str!("../../../backdrops/templates/arrangement-varied.json")),
    ("Fretboard", include_str!("../../../backdrops/fretboard.json")),
    ("Terrain and coverage", include_str!("../../../backdrops/terrain.json")),
];
