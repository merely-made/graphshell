// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The projection wire's envelope (engine split D181, D207 to D210).
//!
//! A keyframe or a diff crosses a carrier as one CBOR::Core map: a version,
//! the payload's kind, its required identity (the engine revision or a
//! declared no-engine marker, the projection epoch and revision), the payload
//! as embedded canonical CBOR,
//! and a domain-separated BLAKE3 over all of it. `wire/projection.cddl` is
//! the schema and `wire/vectors/` pins the bytes. Signing stays with each
//! carrier.

use std::fmt;

use cbor_core::{DecodeOptions, Value};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::{EngineRevision, ProjectionDiff, ProjectionSnapshot, Revision, SceneEpoch};

/// The envelope version this build writes and the only one it reads.
pub const ENVELOPE_VERSION: u64 = 1;

/// The BLAKE3 key-derivation context the digest is computed under.
pub const ENVELOPE_DOMAIN: &str = "graphshell projection envelope v1";

const VERSION: &str = "version";
const KIND: &str = "kind";
const ENGINE: &str = "engine";
const EPOCH: &str = "epoch";
const REVISION: &str = "revision";
const PAYLOAD: &str = "payload";
const DIGEST: &str = "digest";
const FIELDS: [&str; 7] = [VERSION, KIND, ENGINE, EPOCH, REVISION, PAYLOAD, DIGEST];

/// What an envelope carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnvelopeKind {
    Keyframe,
    Diff,
}

impl EnvelopeKind {
    fn name(self) -> &'static str {
        match self {
            Self::Keyframe => "keyframe",
            Self::Diff => "diff",
        }
    }

    fn named(name: &str) -> Option<Self> {
        [Self::Keyframe, Self::Diff]
            .into_iter()
            .find(|kind| kind.name() == name)
    }
}

/// The identity an envelope binds its payload to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnvelopeIdentity {
    /// `None` is the declared no-engine marker, written as `null`: a
    /// projection no engine backs. The field itself is never omitted (D217).
    pub engine: Option<EngineRevision>,
    pub epoch: SceneEpoch,
    pub revision: Revision,
}

/// A payload that travels in the envelope and states its own identity.
pub trait Enveloped: Serialize + DeserializeOwned {
    const KIND: EnvelopeKind;

    /// # Errors
    ///
    /// [`EnvelopeError::MissingIdentity`] when the payload's identity is
    /// incomplete.
    fn identity(&self) -> Result<EnvelopeIdentity, EnvelopeError>;
}

impl Enveloped for ProjectionSnapshot {
    const KIND: EnvelopeKind = EnvelopeKind::Keyframe;

    fn identity(&self) -> Result<EnvelopeIdentity, EnvelopeError> {
        Ok(EnvelopeIdentity {
            engine: self.scene.engine_revision.clone(),
            epoch: self.scene.epoch,
            revision: self.scene.revision,
        })
    }
}

/// A diff names both its base and target engine revisions or neither; the
/// header carries the target and the digest binds the base inside the payload.
impl Enveloped for ProjectionDiff {
    const KIND: EnvelopeKind = EnvelopeKind::Diff;

    fn identity(&self) -> Result<EnvelopeIdentity, EnvelopeError> {
        match (&self.scene.base_engine, &self.scene.engine_revision) {
            (Some(_), None) => return Err(EnvelopeError::MissingIdentity("engine_revision")),
            (None, Some(_)) => return Err(EnvelopeError::MissingIdentity("base_engine")),
            _ => {},
        }
        Ok(EnvelopeIdentity {
            engine: self.scene.engine_revision.clone(),
            epoch: self.scene.epoch,
            revision: self.scene.revision,
        })
    }
}

/// Why an envelope was not written or not accepted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EnvelopeError {
    /// Bytes that decode but are not CBOR::Core's canonical form.
    NonCanonical(String),
    /// Bytes that are not one well-formed CBOR item of the envelope's shape.
    Malformed(String),
    UnknownVersion(u64),
    /// A kind this build does not know, or not the one asked for.
    Kind(String),
    /// The engine revision, epoch or revision is absent, from the header or
    /// from the payload.
    MissingIdentity(&'static str),
    /// A field the schema does not name.
    UnknownField(String),
    DigestMismatch,
    /// The header's identity is not the payload's own.
    IdentityMismatch,
    /// The payload did not encode or decode as its type.
    Payload(String),
}

impl fmt::Display for EnvelopeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonCanonical(e) => write!(f, "envelope is not canonical CBOR: {e}"),
            Self::Malformed(e) => write!(f, "malformed envelope: {e}"),
            Self::UnknownVersion(v) => write!(f, "unknown envelope version {v}"),
            Self::Kind(k) => write!(f, "unexpected envelope kind {k}"),
            Self::MissingIdentity(field) => write!(f, "envelope identity lacks {field}"),
            Self::UnknownField(field) => write!(f, "envelope carries unknown field {field}"),
            Self::DigestMismatch => write!(f, "envelope digest does not match its contents"),
            Self::IdentityMismatch => write!(f, "envelope header disagrees with its payload"),
            Self::Payload(e) => write!(f, "envelope payload: {e}"),
        }
    }
}

impl std::error::Error for EnvelopeError {}

fn decode_strict(bytes: &[u8]) -> Result<Value<'_>, EnvelopeError> {
    DecodeOptions::new()
        .decode(bytes)
        .map_err(|error| match error {
            cbor_core::Error::NonDeterministic => EnvelopeError::NonCanonical(error.to_string()),
            other => EnvelopeError::Malformed(other.to_string()),
        })
}

/// Encode a payload as its canonical CBOR, the bytes the envelope embeds.
///
/// # Errors
///
/// [`EnvelopeError::Payload`] when the value does not serialize.
pub fn payload_bytes<P: Serialize>(payload: &P) -> Result<Vec<u8>, EnvelopeError> {
    Value::serialized(payload)
        .map(|value| value.encode())
        .map_err(|error| EnvelopeError::Payload(error.to_string()))
}

fn digest_of(unsigned: &Value<'_>) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_derive_key(ENVELOPE_DOMAIN);
    unsigned
        .write_to(&mut hasher)
        .expect("hashing never fails to write");
    *hasher.finalize().as_bytes()
}

/// Seal a payload in an envelope.
///
/// # Errors
///
/// [`EnvelopeError::MissingIdentity`] for a diff that names only one of its
/// engine revisions, or [`EnvelopeError::Payload`] when it does not serialize.
pub fn seal<P: Enveloped>(payload: &P) -> Result<Vec<u8>, EnvelopeError> {
    let identity = payload.identity()?;
    let engine = Value::serialized(&identity.engine)
        .map_err(|error| EnvelopeError::Payload(error.to_string()))?;
    let mut envelope = Value::map([
        (VERSION, Value::from(ENVELOPE_VERSION)),
        (KIND, Value::from(P::KIND.name())),
        (ENGINE, engine),
        (EPOCH, Value::from(identity.epoch.0)),
        (REVISION, Value::from(identity.revision.0)),
        (PAYLOAD, Value::from(payload_bytes(payload)?)),
    ]);
    let digest = digest_of(&envelope);
    envelope.insert(DIGEST, Value::from(digest.to_vec()));
    Ok(envelope.encode())
}

/// Open an envelope holding a `P`.
///
/// # Errors
///
/// Every refusal [`Envelope::read`] makes, a kind other than `P`'s, a
/// payload that does not decode, or a header that is not the payload's own.
pub fn open<P: Enveloped>(bytes: &[u8]) -> Result<P, EnvelopeError> {
    Envelope::read(bytes)?.payload()
}

/// An envelope whose digest verified, with its payload still encoded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Envelope<'a> {
    pub kind: EnvelopeKind,
    pub identity: EnvelopeIdentity,
    payload: &'a [u8],
}

impl<'a> Envelope<'a> {
    /// Read and verify an envelope's header without decoding its payload.
    ///
    /// # Errors
    ///
    /// Non-canonical or malformed bytes, an unknown version or kind, a field
    /// the schema does not name, a missing identity, or a digest that does
    /// not match.
    pub fn read(bytes: &'a [u8]) -> Result<Self, EnvelopeError> {
        let mut envelope = decode_strict(bytes)?;
        let map = envelope
            .as_map()
            .map_err(|_| EnvelopeError::Malformed("an envelope is a map".into()))?;
        for key in map.keys() {
            let name = key.as_str().unwrap_or_default();
            if !FIELDS.contains(&name) {
                return Err(EnvelopeError::UnknownField(format!("{key:?}")));
            }
        }
        let version = field(&envelope, VERSION)?
            .to_u64()
            .map_err(|_| EnvelopeError::Malformed("version is not an unsigned integer".into()))?;
        if version != ENVELOPE_VERSION {
            return Err(EnvelopeError::UnknownVersion(version));
        }
        let kind_name = text(field(&envelope, KIND)?, KIND)?;
        let kind = EnvelopeKind::named(kind_name)
            .ok_or_else(|| EnvelopeError::Kind(kind_name.to_string()))?;
        let identity = EnvelopeIdentity {
            engine: engine(identity_field(&envelope, ENGINE)?)?,
            epoch: SceneEpoch(unsigned(identity_field(&envelope, EPOCH)?, EPOCH)?),
            revision: Revision(unsigned(identity_field(&envelope, REVISION)?, REVISION)?),
        };
        let digest = envelope
            .remove(DIGEST)
            .ok_or(EnvelopeError::MissingIdentity(DIGEST))?;
        let Ok(digest) = digest.as_bytes() else {
            return Err(EnvelopeError::Malformed(
                "digest is not a byte string".into(),
            ));
        };
        if digest != digest_of(&envelope) {
            return Err(EnvelopeError::DigestMismatch);
        }
        let payload = match envelope.remove(PAYLOAD) {
            Some(Value::ByteString(std::borrow::Cow::Borrowed(payload))) => payload,
            Some(_) => {
                return Err(EnvelopeError::Malformed(
                    "payload is not a byte string".into(),
                ));
            },
            None => return Err(EnvelopeError::Malformed("payload is absent".into())),
        };
        Ok(Self {
            kind,
            identity,
            payload,
        })
    }

    /// The payload's canonical CBOR, as embedded.
    pub fn payload_bytes(&self) -> &'a [u8] {
        self.payload
    }

    /// Decode the payload as a `P` and check that it is what the header says.
    ///
    /// # Errors
    ///
    /// A kind other than `P`'s, payload bytes that are not canonical or not
    /// a `P`, or a payload whose own identity differs from the header's.
    pub fn payload<P: Enveloped>(&self) -> Result<P, EnvelopeError> {
        if self.kind != P::KIND {
            return Err(EnvelopeError::Kind(self.kind.name().to_string()));
        }
        let value = decode_strict(self.payload)?;
        let payload: P = value
            .deserialized()
            .map_err(|error| EnvelopeError::Payload(error.to_string()))?;
        if payload.identity()? != self.identity {
            return Err(EnvelopeError::IdentityMismatch);
        }
        Ok(payload)
    }
}

fn field<'v>(envelope: &'v Value<'_>, name: &'static str) -> Result<&'v Value<'v>, EnvelopeError> {
    envelope
        .get(name)
        .ok_or_else(|| EnvelopeError::Malformed(format!("{name} is absent")))
}

fn identity_field<'v>(
    envelope: &'v Value<'_>,
    name: &'static str,
) -> Result<&'v Value<'v>, EnvelopeError> {
    envelope
        .get(name)
        .ok_or(EnvelopeError::MissingIdentity(name))
}

fn text<'v>(value: &'v Value<'_>, name: &'static str) -> Result<&'v str, EnvelopeError> {
    value
        .as_str()
        .map_err(|_| EnvelopeError::Malformed(format!("{name} is not text")))
}

fn unsigned(value: &Value<'_>, name: &'static str) -> Result<u64, EnvelopeError> {
    value
        .to_u64()
        .map_err(|_| EnvelopeError::Malformed(format!("{name} is not an unsigned integer")))
}

fn engine(value: &Value<'_>) -> Result<Option<EngineRevision>, EnvelopeError> {
    if value.data_type().is_null() {
        return Ok(None);
    }
    let map = value
        .as_map()
        .map_err(|_| EnvelopeError::Malformed("engine is neither a map nor null".into()))?;
    for key in map.keys() {
        if !matches!(key.as_str(), Ok("session" | "cursor" | "digest")) {
            return Err(EnvelopeError::UnknownField(format!("engine {key:?}")));
        }
    }
    let part = |name: &'static str| value.get(name).ok_or(EnvelopeError::MissingIdentity(name));
    Ok(Some(EngineRevision {
        session: text(part("session")?, "session")?.to_string(),
        cursor: unsigned(part("cursor")?, "cursor")?,
        digest: text(part("digest")?, "digest")?.to_string(),
    }))
}

#[path = "envelope_serde.rs"]
mod carried;
pub use carried::{Keyframe, diffs, keyframe};

#[cfg(test)]
#[path = "envelope_tests.rs"]
mod tests;
