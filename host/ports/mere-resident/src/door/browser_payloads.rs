// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The browser door's payload translation (engine split D231). The extension
//! writes identity intent payloads as JSON, and the identity endpoint reads
//! only canonical CBOR, so each known intent is parsed into its Rust type here
//! and re-encoded. An intent this door does not know is refused before it
//! reaches the endpoint.

use chirograph::{CarrierRequest, CarrierRequestBody};
use dramatis::intents::{
    CreateProfileIntentV1, DEVICE_REVOKE_INTENT, GenerateSshKeyIntentV1, ImportSshKeyNativeIntentV1,
    LockVaultIntentV1, PROFILE_CREATE_INTENT, PROFILE_SWITCH_INTENT, RemoveSshKeyIntentV1,
    RevokeDeviceIntentV1, SIGNING_APPROVE_IDLE_INTENT, SIGNING_APPROVE_ONCE_INTENT,
    SIGNING_DENY_INTENT, SSH_GENERATE_INTENT, SSH_IMPORT_NATIVE_INTENT, SSH_REMOVE_INTENT,
    SigningDecisionIntentV1, SwitchProfileIntentV1, VAULT_LOCK_INTENT,
};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::door::identity_endpoint::{TRANSFER_ACCEPT_INTENT, TransferAcceptIntentV1};

fn typed<T: Serialize + DeserializeOwned>(json: &[u8]) -> Result<Vec<u8>, String> {
    let value: T = serde_json::from_slice(json)
        .map_err(|error| format!("the payload is not this action's JSON: {error}"))?;
    chirograph::wire::encode(&value).map_err(|error| error.to_string())
}

/// One identity intent's JSON payload as the canonical CBOR the endpoint
/// reads. An empty payload stays empty.
pub(crate) fn identity_payload(intent: &str, json: &[u8]) -> Result<Vec<u8>, String> {
    if json.is_empty() {
        return Ok(Vec::new());
    }
    match intent {
        SIGNING_APPROVE_ONCE_INTENT | SIGNING_APPROVE_IDLE_INTENT | SIGNING_DENY_INTENT => {
            typed::<SigningDecisionIntentV1>(json)
        },
        SSH_GENERATE_INTENT => typed::<GenerateSshKeyIntentV1>(json),
        SSH_IMPORT_NATIVE_INTENT => typed::<ImportSshKeyNativeIntentV1>(json),
        SSH_REMOVE_INTENT => typed::<RemoveSshKeyIntentV1>(json),
        DEVICE_REVOKE_INTENT => typed::<RevokeDeviceIntentV1>(json),
        PROFILE_SWITCH_INTENT => typed::<SwitchProfileIntentV1>(json),
        PROFILE_CREATE_INTENT => typed::<CreateProfileIntentV1>(json),
        VAULT_LOCK_INTENT => typed::<LockVaultIntentV1>(json),
        TRANSFER_ACCEPT_INTENT => typed::<TransferAcceptIntentV1>(json),
        other => Err(format!("the browser door does not carry {other}")),
    }
}

/// Translate an intent request's payload in place; other requests pass.
pub(crate) fn translate(request: &mut CarrierRequest) -> Result<(), String> {
    if let CarrierRequestBody::Intent(intent) = &mut request.body {
        intent.payload = identity_payload(&intent.intent, &intent.payload)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_json_identity_payload_becomes_the_cbor_of_its_type() {
        let json = br#"{"profile":"family"}"#;
        let cbor = identity_payload(PROFILE_SWITCH_INTENT, json).unwrap();
        let read: SwitchProfileIntentV1 = chirograph::wire::decode(&cbor).unwrap();
        assert_eq!(read, serde_json::from_slice::<SwitchProfileIntentV1>(json).unwrap());
        assert!(identity_payload(PROFILE_SWITCH_INTENT, br#"{"profile":7}"#).is_err());
        assert!(identity_payload("mere.session.apply", json).is_err());
        assert_eq!(identity_payload(VAULT_LOCK_INTENT, b"").unwrap(), Vec::<u8>::new());
    }
}
