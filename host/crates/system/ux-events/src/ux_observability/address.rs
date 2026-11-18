// Copyright 2026 Mark Alan Boykin
// SPDX-License-Identifier: MPL-2.0

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use uxtree::envelope::{AddressSpace, Envelope};

/// Address metadata copied from the observed accessibility envelope.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "space", rename_all = "snake_case")]
pub enum EventAddress {
    Producer { instance: Uuid, local: u64 },
    Grafted { tree: Uuid, local: u64 },
}

impl EventAddress {
    /// The host names its producer instance with an opaque ID. An installed
    /// graft keeps its tree ID. The envelope supplies the local ID in either case.
    pub fn from_envelope<I, P, D>(
        envelope: &Envelope<AddressSpace<I>, P, D>,
        producer_id: impl FnOnce(&I) -> Uuid,
    ) -> Self {
        let local = envelope.address.local.0;
        match &envelope.address.instance {
            AddressSpace::Producer(instance) => Self::Producer {
                instance: producer_id(instance),
                local,
            },
            AddressSpace::Grafted(tree) => Self::Grafted {
                tree: tree.0,
                local,
            },
        }
    }
}
