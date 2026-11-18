// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Publication generations for Derived addresses. Each publisher owns its map
//! and supplies the equivalence rule for admission records.

use std::{collections::HashMap, hash::Hash};

/// The admission observed when a publisher captured a request.
#[derive(Clone, Debug, PartialEq)]
pub struct Published<A> {
    pub generation: u64,
    pub admission: A,
}

/// One publisher's complete current set. Generations belong to this map and
/// are checked against the key and map that captured the request.
pub struct GenerationMap<K, A> {
    current: HashMap<K, Published<A>>,
    next_generation: u64,
}

impl<K, A> Default for GenerationMap<K, A> {
    fn default() -> Self {
        Self {
            current: HashMap::new(),
            next_generation: 0,
        }
    }
}

impl<K: Eq + Hash, A> GenerationMap<K, A> {
    /// Replace the whole published set. A retained, equivalent admission keeps
    /// its generation. Changed or returning admissions get fresh generations.
    pub fn publish(&mut self, admissions: HashMap<K, A>, mut same: impl FnMut(&A, &A) -> bool) {
        self.current = admissions
            .into_iter()
            .map(|(key, admission)| {
                let generation = self
                    .current
                    .get(&key)
                    .filter(|old| same(&old.admission, &admission))
                    .map_or_else(
                        || {
                            self.next_generation = self
                                .next_generation
                                .checked_add(1)
                                .expect("accessibility admission generations exhausted");
                            self.next_generation
                        },
                        |old| old.generation,
                    );
                (
                    key,
                    Published {
                        generation,
                        admission,
                    },
                )
            })
            .collect();
    }

    pub fn get(&self, key: &K) -> Option<&Published<A>> {
        self.current.get(key)
    }

    pub fn is_current(&self, key: &K, captured: &Published<A>) -> bool {
        self.get(key)
            .is_some_and(|current| current.generation == captured.generation)
    }

    /// The current snapshot, for publisher diagnostics and tests.
    pub fn entries(&self) -> &HashMap<K, Published<A>> {
        &self.current
    }
}

#[cfg(test)]
mod tests;
