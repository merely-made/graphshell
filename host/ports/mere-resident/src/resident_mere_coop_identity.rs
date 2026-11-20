// Copyright 2026 Mark Alan Boykin
// SPDX-License-Identifier: MPL-2.0

//! The attached resident owns the co-op authority. The custodian retains its signing keys.

use super::{Arc, IdentityProvider, ResidentCustody, SessionId, coop};
use castellan_client::custody::{CustodyEpoch, KeySource};
use insigne::DerivedKeyAttestation;
use insigne::delegation::{
    CapabilityScope, DelegationCertificate, DelegationRevocation, SignedDelegationCertificate,
    SignedDelegationRevocation, delegation_signing_salt,
};
use personae::{Ed25519Keypair, Ed25519PublicKey, IdentityError};

pub(super) struct OwnerIdentity {
    custody: Arc<dyn ResidentCustody>,
    graph: SessionId,
    master: Ed25519PublicKey,
    epoch: Option<CustodyEpoch>,
}

impl OwnerIdentity {
    pub(super) fn bind(
        custody: Arc<dyn ResidentCustody>,
        graph: SessionId,
    ) -> Result<Self, String> {
        let epoch = custody
            .custody_epoch()
            .map_err(|error| format!("co-op custody: {error:?}"))?;
        let master = custody.master_public_key();
        let identity = Self {
            custody,
            graph,
            master,
            epoch,
        };
        if !identity.current() {
            return Err("Unlock the identity before using co-op".into());
        }
        Ok(identity)
    }

    pub(super) fn current(&self) -> bool {
        !self.custody.is_locked()
            && self.custody.master_public_key() == self.master
            && self
                .custody
                .custody_epoch()
                .is_ok_and(|epoch| epoch == self.epoch)
    }

    fn sign(
        &self,
        scope: &CapabilityScope,
        message: &[u8],
    ) -> Result<(DerivedKeyAttestation, Vec<u8>), String> {
        let mut permitted = coop::scope(self.master.to_bytes(), self.graph);
        if scope.actions.contains("edit") {
            permitted.actions.insert("edit".into());
        }
        if scope != &permitted || !self.current() {
            return Err("co-op signing authority unavailable".into());
        }
        let salt = delegation_signing_salt(scope);
        let attestation = self
            .custody
            .attest(KeySource::Persona, None, &salt)
            .map_err(|error| format!("co-op signing key: {error:?}"))?;
        let (public_key, signature) = self
            .custody
            .sign_delegation(coop::DOMAIN, &salt, message)
            .map_err(|error| format!("co-op signature: {error:?}"))?;
        let checked = attestation
            .check(&salt)
            .map_err(|error| error.to_string())?;
        if !self.current()
            || checked.master() != &self.master.to_bytes()
            || checked.derived() != &public_key
        {
            return Err("co-op signing authority changed".into());
        }
        Ok((attestation, signature))
    }

    fn check_transport(&self, salt: &[u8]) -> Result<(), IdentityError> {
        if salt != coop::TRANSPORT_SALT {
            return Err(IdentityError::DerivationFailed(
                "not the co-op transport salt".into(),
            ));
        }
        if !self.current() {
            return Err(IdentityError::Locked);
        }
        Ok(())
    }
}

impl IdentityProvider for OwnerIdentity {
    fn master_public_key(&self) -> Ed25519PublicKey {
        self.master
    }

    fn derive_keypair(&self, salt: &[u8]) -> Result<Ed25519Keypair, IdentityError> {
        self.check_transport(salt)?;
        let key = self.custody.derive_keypair(salt)?;
        self.check_transport(salt)?;
        Ok(key)
    }

    fn attest_derived_key(&self, salt: &[u8]) -> Result<DerivedKeyAttestation, IdentityError> {
        self.check_transport(salt)?;
        let attestation = self.custody.attest_derived_key(salt)?;
        self.check_transport(salt)?;
        Ok(attestation)
    }
}

impl coop::Signer for OwnerIdentity {
    fn certificate(
        &self,
        value: DelegationCertificate,
    ) -> Result<SignedDelegationCertificate, String> {
        let (signer, signature) = self.sign(&value.scope, &value.signing_bytes())?;
        Ok(SignedDelegationCertificate::from_parts(
            value, signer, signature,
        ))
    }

    fn revocation(
        &self,
        value: DelegationRevocation,
    ) -> Result<SignedDelegationRevocation, String> {
        let (signer, signature) = self.sign(&value.scope, &value.signing_bytes())?;
        Ok(SignedDelegationRevocation::from_parts(
            value, signer, signature,
        ))
    }
}
