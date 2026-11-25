// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Serde adapters that seal a keyframe or diff wherever a carrier message
//! carries one (D210, D219): a snapshot reply is a keyframe envelope, and a
//! diffs reply a sequence of diff envelopes, each bound alone.
//!
//! A binary format carries the sealed bytes, both ways. A human-readable one
//! carries the plain value: that is the native-messaging hop, where the host
//! has already opened the envelope and the browser reads JSON (D220).
//! [`crate::CarrierOutput`] is read field by field so that a keyframe inside
//! it still knows which it is.

use std::fmt;

use serde::de::{self, Deserializer, SeqAccess, Visitor};
use serde::ser::{self, SerializeSeq, Serializer};
use serde::{Deserialize, Serialize};

use super::{Enveloped, open, seal};
use crate::{ProjectionDiff, ProjectionSnapshot};

fn sealed<P: Enveloped, S: Serializer>(value: &P, serializer: S) -> Result<S::Ok, S::Error> {
    if serializer.is_human_readable() {
        return value.serialize(serializer);
    }
    let bytes = seal(value).map_err(ser::Error::custom)?;
    serializer.serialize_bytes(&bytes)
}

struct Sealed<P>(std::marker::PhantomData<P>);

impl<P: Enveloped> Visitor<'_> for Sealed<P> {
    type Value = P;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a sealed projection envelope")
    }

    fn visit_bytes<E: de::Error>(self, bytes: &[u8]) -> Result<P, E> {
        open(bytes).map_err(E::custom)
    }
}

fn opened<'de, P: Enveloped, D: Deserializer<'de>>(deserializer: D) -> Result<P, D::Error> {
    if deserializer.is_human_readable() {
        return P::deserialize(deserializer);
    }
    deserializer.deserialize_bytes(Sealed(std::marker::PhantomData))
}

/// `#[serde(with)]` for a boxed keyframe.
pub mod keyframe {
    use super::*;

    /// # Errors
    ///
    /// When the keyframe does not seal.
    pub fn serialize<S: Serializer>(
        value: &ProjectionSnapshot,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        sealed(value, serializer)
    }

    /// # Errors
    ///
    /// Every refusal [`super::super::open`] makes.
    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Box<ProjectionSnapshot>, D::Error> {
        opened(deserializer).map(Box::new)
    }
}

/// `#[serde(with)]` for a run of diffs: an array of diff envelopes.
pub mod diffs {
    use super::*;

    struct One<'a>(&'a ProjectionDiff);

    impl Serialize for One<'_> {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            sealed(self.0, serializer)
        }
    }

    struct Each;

    impl<'de> Visitor<'de> for Each {
        type Value = Vec<ProjectionDiff>;

        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("an array of sealed diff envelopes")
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut diffs = Vec::with_capacity(seq.size_hint().unwrap_or(0).min(1024));
            while let Some(OpenedDiff(diff)) = seq.next_element()? {
                diffs.push(diff);
            }
            Ok(diffs)
        }
    }

    struct OpenedDiff(ProjectionDiff);

    impl<'de> Deserialize<'de> for OpenedDiff {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            opened(deserializer).map(OpenedDiff)
        }
    }

    /// # Errors
    ///
    /// When a diff does not seal.
    pub fn serialize<S: Serializer>(
        value: &[ProjectionDiff],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(value.len()))?;
        for diff in value {
            seq.serialize_element(&One(diff))?;
        }
        seq.end()
    }

    /// # Errors
    ///
    /// Every refusal [`super::super::open`] makes, for any diff.
    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Vec<ProjectionDiff>, D::Error> {
        deserializer.deserialize_seq(Each)
    }
}

/// A keyframe that serializes as its envelope, for a carrier that sends one
/// on its own (the WIT door's snapshot call).
#[derive(Clone, Debug, PartialEq)]
pub struct Keyframe(pub ProjectionSnapshot);

impl Serialize for Keyframe {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        sealed(&self.0, serializer)
    }
}

impl<'de> Deserialize<'de> for Keyframe {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        opened(deserializer).map(Keyframe)
    }
}
