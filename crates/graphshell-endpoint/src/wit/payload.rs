// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Payloads: one version byte, then CBOR::Core through cbor-core (D6,
//! engine split D209), at most [`MAX_PAYLOAD`] bytes (D13). Both sides check
//! the cap, and a payload that is not canonical is refused.

use cbor_core::Value;
use serde::Serialize;
use serde::de::DeserializeOwned;

use super::Failure;

/// Version 1 was ciborium's lenient encoding; 2 is canonical CBOR::Core.
pub const VERSION: u8 = 2;
/// The browser carrier's native-message cap, the tightest carrier's.
pub const MAX_PAYLOAD: usize = 1024 * 1024;

/// # Errors
///
/// [`Failure::Malformed`] when the value does not serialize, and
/// [`Failure::TooLarge`] past the cap.
pub fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, Failure> {
    let value = Value::serialized(value).map_err(|e| Failure::Malformed(e.to_string()))?;
    let mut bytes = vec![VERSION];
    value
        .write_to(&mut bytes)
        .map_err(|e| Failure::Malformed(e.to_string()))?;
    check(&bytes)?;
    Ok(bytes)
}

/// # Errors
///
/// [`Failure::TooLarge`] past the cap, and [`Failure::Malformed`] for another
/// version, bytes that are not canonical CBOR, or a value that is not a `T`.
pub fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, Failure> {
    check(bytes)?;
    match bytes.split_first() {
        Some((&VERSION, body)) => Value::decode(body)
            .map_err(|e| Failure::Malformed(e.to_string()))?
            .deserialized()
            .map_err(|e| Failure::Malformed(e.to_string())),
        Some((version, _)) => Err(Failure::Malformed(format!("payload version {version}"))),
        None => Err(Failure::Malformed("empty payload".into())),
    }
}

/// # Errors
///
/// [`Failure::TooLarge`] past the cap.
pub fn check(bytes: &[u8]) -> Result<(), Failure> {
    if bytes.len() > MAX_PAYLOAD {
        return Err(Failure::TooLarge(bytes.len() as u64));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_value_round_trips_canonically() {
        let value = (String::from("session"), 7u64, vec![1.5f32, -2.0]);
        let bytes = encode(&value).unwrap();
        assert_eq!(bytes[0], VERSION);
        assert_eq!(bytes[1..], Value::serialized(&value).unwrap().encode());
        assert_eq!(decode::<(String, u64, Vec<f32>)>(&bytes).unwrap(), value);
    }

    #[test]
    fn a_non_canonical_or_older_payload_is_refused() {
        assert_eq!(decode::<u64>(&[VERSION, 0x07]).unwrap(), 7, "control");
        assert!(matches!(
            decode::<u64>(&[VERSION, 0x18, 0x07]),
            Err(Failure::Malformed(_))
        ));
        assert!(matches!(
            decode::<u64>(&[1, 0x07]),
            Err(Failure::Malformed(_))
        ));
    }

    #[test]
    fn the_cap_is_checked_both_ways() {
        let under = Value::from(vec![0u8; MAX_PAYLOAD - 16]);
        assert!(encode(&under).is_ok(), "control");
        let over = Value::from(vec![0u8; MAX_PAYLOAD]);
        assert!(matches!(encode(&over), Err(Failure::TooLarge(_))));
        assert!(matches!(
            decode::<u64>(&vec![VERSION; MAX_PAYLOAD + 1]),
            Err(Failure::TooLarge(_))
        ));
    }
}
