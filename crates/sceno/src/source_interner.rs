// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Interning many sources into one scene through an index.

use std::collections::HashMap;

use crate::scene::{Scene, SourceIx, SourceRef};

/// An index over one scene's sources.
///
/// [`Scene::intern_source`] searches the source table linearly, so a builder
/// interning one source per item is quadratic in the item count. This hands
/// out the same indices, the first entry for an equal source, in constant
/// time. Every source added to the scene after [`SourceInterner::for_scene`]
/// must come through [`SourceInterner::intern`], or the index goes stale.
#[derive(Debug, Clone, Default)]
pub struct SourceInterner {
    index: HashMap<SourceRef, SourceIx>,
}

impl SourceInterner {
    /// An index over `scene`'s current sources. A source the table already
    /// holds twice keeps its first index, as `intern_source` would find it.
    pub fn for_scene(scene: &Scene) -> Self {
        let mut index = HashMap::with_capacity(scene.sources.len());
        for (i, source) in scene.sources.iter().enumerate() {
            index.entry(source.clone()).or_insert(SourceIx(i as u32));
        }
        Self { index }
    }

    /// Intern `source` into `scene`, reusing an equal entry.
    pub fn intern(&mut self, scene: &mut Scene, source: SourceRef) -> SourceIx {
        *self.index.entry(source).or_insert_with_key(|source| {
            scene.sources.push(source.clone());
            SourceIx((scene.sources.len() - 1) as u32)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn refs(ids: &[&str]) -> Vec<SourceRef> {
        ids.iter().map(|id| SourceRef::new("test", *id)).collect()
    }

    /// The interner and the linear search agree index for index, on a table
    /// that already holds a duplicate.
    #[test]
    fn interner_matches_linear_intern_source() {
        let mut seeded = Scene::new();
        seeded.sources = refs(&["a", "b", "a"]);
        let sequence = refs(&["c", "a", "d", "b", "c", "e", "a"]);

        let mut linear = seeded.clone();
        let by_search: Vec<SourceIx> = sequence
            .iter()
            .map(|s| linear.intern_source(s.clone()))
            .collect();
        let mut indexed = seeded.clone();
        let mut interner = SourceInterner::for_scene(&indexed);
        let by_index: Vec<SourceIx> = sequence
            .iter()
            .map(|s| interner.intern(&mut indexed, s.clone()))
            .collect();

        assert_eq!(by_index, by_search);
        assert_eq!(indexed.sources, linear.sources);
        assert_eq!(by_index[1], SourceIx(0), "the first `a` wins");

        // Control: an interner that skipped the seeded table would append
        // `a` and `b` again, and the comparison catches it.
        let mut fresh = seeded.clone();
        let mut blind = SourceInterner::default();
        let blind_ix: Vec<SourceIx> = sequence
            .iter()
            .map(|s| blind.intern(&mut fresh, s.clone()))
            .collect();
        assert_ne!(blind_ix, by_search);
        assert_ne!(fresh.sources, linear.sources);
    }
}
