// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Metadata beside a producer's AccessKit nodes (uxtree spine D1–D4).
//!
//! [`Envelope`] adds identity and source facts without another node model.
//! AccessKit continues to own roles, names, state, geometry, children and
//! relations. Producers explicitly declare [`Stability`] through a named
//! constructor; readers do not infer it from the tree or numeric ID.
//!
//! The instance `I`, provenance `P` and domain reference `D` belong to callers.
//! A host can use its existing session key, weak publication witness and domain
//! reference without translating them into source text or importing host types
//! here. All producers within one reader use that reader's common types, as the
//! mixed-source E1a bench does. Genet keeps its own neutral projection and gains
//! no dependency on uxtree; its host supplies the envelope.
//!
//! # Field sources
//!
//! This is the spine brief's per-field inventory, not an assertion that every
//! publishing adapter is migrated. Those adapters are P2–P6.
//!
//! | Field | Cambium | Genet, through host | Pelt | Canvas | Turnstone | graphshell-client, through host |
//! |---|---|---|---|---|---|---|
//! | address | source ID + owner | local ID + session | tile + local ID | described key | path hash + part | scene instance |
//! | stability | Life | Scoped, semantic revision | Life | Life | Derived, route generation | Scoped, scene epoch |
//! | provenance | publication witness | host session; projection support stays in Genet | session and placement | host slot | part and route owner | scene source and owner |
//! | action_ids | texture command IDs; DOM kinds alone give none | kinds alone give none | kinds alone give none | advertised intent | app actions still need command mapping | advertised intent |
//! | domain_ref | DOM has none | none | tile reference, partial | member UUID needs forwarding | frozen member; chrome has none | SourceRef |
//! | authored_key | authored DOM key needs forwarding | host needs to read/forward key | tile DOM key needs forwarding | member UUID | chrome has none | instance or source key |
//!
//! Empty optional fields preserve absent source information. Command IDs come
//! only from advertised commands; names and action kinds never manufacture one.
//!
//! # Admission remains at the owner
//!
//! This module allocates no identities, revisions or generations and performs
//! no action admission. A request must retain its observed scope and provenance
//! and be checked by its current owner. Re-reading a reused address cannot grant
//! authority to an old request. Derived nodes without a generation are readable
//! observations only. See [`crate::generation::GenerationMap`] for publication. Authored keys are resolved
//! to a current Life or Scoped target at use, and that resolved target is not
//! retained as a durable identity. There is no serialization or wire contract.
//!
//! ```
//! use accesskit::NodeId;
//! use uxtree::envelope::{Address, Envelope, Stability};
//!
//! let address = Address::new(7_u64, NodeId(12)); // instance issued by the owner
//! let envelope: Envelope<_, _, (u64, u64)> =
//!     Envelope::scoped(address, 3, ("document", "owner witness"));
//! assert_eq!(envelope.stability, Stability::Scoped { revision: 3 });
//! assert!(envelope.action_ids.is_empty());
//! ```

use accesskit::{NodeId, TreeId};

/// An address in an installed AccessKit tree, with the tree's local node ID.
pub type TreeAddress = (TreeId, NodeId);

/// The identity space used by a reader that combines producers and grafts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AddressSpace<I> {
    Producer(I),
    Grafted(TreeId),
}

/// An address that distinguishes producer instances from installed trees.
pub type FederatedAddress<I> = Address<AddressSpace<I>>;

/// An instance and its local AccessKit ID. Pure producers can supply their
/// existing instance identity. Readers that also use installed trees use
/// [`FederatedAddress`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Address<I> {
    pub instance: I,
    pub local: NodeId,
}

impl<I> Address<I> {
    pub fn new(instance: I, local: NodeId) -> Self {
        Self { instance, local }
    }
}

impl<I> FederatedAddress<I> {
    /// Keep a local node in an un-grafted producer's identity space.
    pub fn producer(instance: I, local: NodeId) -> Self {
        Self::new(AddressSpace::Producer(instance), local)
    }

    /// Address a node by the installed tree and its unchanged local ID.
    pub fn grafted((tree, local): TreeAddress) -> Self {
        Self::new(AddressSpace::Grafted(tree), local)
    }

    pub fn tree_address(&self) -> Option<TreeAddress> {
        match &self.instance {
            AddressSpace::Producer(_) => None,
            AddressSpace::Grafted(tree) => Some((*tree, self.local)),
        }
    }
}

/// A producer's identity claim; it is never permission to dispatch an action.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Stability {
    /// Owner-issued for one object's life; never reused for another object.
    Life,
    /// The owner must check the observed semantic revision or scene epoch.
    Scoped { revision: u64 },
    /// A path-derived ID, with an owner-issued route generation when available.
    /// `None` supports observation only. Neither value is a frame counter.
    Derived { generation: Option<u64> },
    /// Resolve the authored key at use to a current Life or Scoped target.
    Authored,
}

/// The six source fields beside an AccessKit node, with caller-owned payloads.
///
/// `P` preserves the owner's provenance, including opaque weak witnesses;
/// `D` preserves the product's actual domain reference. These are process-local
/// observations, without `Send`, `'static` or serialization requirements.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Envelope<I, P, D> {
    pub address: Address<I>,
    pub stability: Stability,
    pub provenance: P,
    pub action_ids: Vec<String>,
    pub domain_ref: Option<D>,
    pub authored_key: Option<String>,
}

impl<I, P, D> Envelope<I, P, D> {
    fn declared(address: Address<I>, stability: Stability, provenance: P) -> Self {
        Self {
            address,
            stability,
            provenance,
            action_ids: Vec::new(),
            domain_ref: None,
            authored_key: None,
        }
    }

    /// Declare an owner-issued object life. This does not mint or renew it.
    pub fn life(address: Address<I>, provenance: P) -> Self {
        Self::declared(address, Stability::Life, provenance)
    }

    /// Capture the source's semantic revision or scene epoch unchanged.
    pub fn scoped(address: Address<I>, revision: u64, provenance: P) -> Self {
        Self::declared(address, Stability::Scoped { revision }, provenance)
    }

    /// Capture a path address and its optional owner-issued route generation.
    pub fn derived(address: Address<I>, generation: Option<u64>, provenance: P) -> Self {
        Self::declared(address, Stability::Derived { generation }, provenance)
    }

    /// Carry an authored lookup key. The local address is only an observation;
    /// callers resolve this key again before acting and retain no resolved ID.
    pub fn authored(address: Address<I>, key: impl Into<String>, provenance: P) -> Self {
        let mut envelope = Self::declared(address, Stability::Authored, provenance);
        envelope.authored_key = Some(key.into());
        envelope
    }
}

#[cfg(test)]
mod tests;
