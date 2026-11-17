// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

use std::cell::Cell;
use std::collections::HashSet;

use super::*;

type Metadata = Envelope<u64, &'static str, (u64, u128)>;

#[test]
fn constructors_preserve_the_producers_four_declared_classes() {
    let address = Address::new(7, NodeId(12));
    let values: [Metadata; 4] = [
        Envelope::life(address, "life witness"),
        Envelope::scoped(address, 0, "revision witness"),
        Envelope::derived(address, Some(41), "route witness"),
        Envelope::authored(address, "field/save", "key owner"),
    ];
    let expected = [
        Stability::Life,
        Stability::Scoped { revision: 0 },
        Stability::Derived {
            generation: Some(41),
        },
        Stability::Authored,
    ];
    for (value, class) in values.iter().zip(expected) {
        assert_eq!(value.address, address);
        assert_eq!(
            value.stability, class,
            "producer declaration must survive construction"
        );
        assert!(value.action_ids.is_empty());
        assert!(value.domain_ref.is_none());
    }
    assert_eq!(values[0].provenance, "life witness");
    assert_eq!(values[1].provenance, "revision witness");
    assert_eq!(values[2].provenance, "route witness");
    assert_eq!(values[3].provenance, "key owner");
    assert_eq!(values[3].authored_key.as_deref(), Some("field/save"));
    assert!(values[..3].iter().all(|value| value.authored_key.is_none()));
}

#[test]
fn derived_observation_does_not_invent_a_generation() {
    let value: Metadata = Envelope::derived(Address::new(7, NodeId(12)), None, "route witness");
    assert_eq!(value.stability, Stability::Derived { generation: None });
}

#[test]
fn address_keeps_local_id_reuse_in_separate_instances() {
    let addresses = [Address::new(7, NodeId(12)), Address::new(8, NodeId(12))];
    assert_eq!(addresses.into_iter().collect::<HashSet<_>>().len(), 2);
}

#[test]
fn payloads_keep_borrowed_owner_facts_and_domain_identity() {
    let owner = Cell::new(true);
    let reference = (3_u64, 40_u128);
    let mut value = Envelope::life(Address::new(7_u64, NodeId(12)), &owner);
    value.domain_ref = Some(&reference);
    value.action_ids = vec!["pin".into(), "open".into()];
    value.authored_key = Some("drawing/item".into());
    let copied = value.clone();
    assert!(std::ptr::eq(copied.provenance, &owner));
    assert!(std::ptr::eq(copied.domain_ref.unwrap(), &reference));
    assert_eq!(copied.action_ids, ["pin", "open"]);
    assert_eq!(copied.authored_key.as_deref(), Some("drawing/item"));
    owner.set(false);
    assert!(
        !copied.provenance.get(),
        "copying metadata must not renew its owner"
    );
}

fn fields(source: &str) -> Vec<&str> {
    source
        .split_once("pub struct Envelope<")
        .unwrap()
        .1
        .split_once('{')
        .unwrap()
        .1
        .split_once('}')
        .unwrap()
        .0
        .lines()
        .filter_map(|line| line.trim().strip_prefix("pub "))
        .map(|line| line.split_once(':').unwrap().0)
        .collect()
}

fn assert_metadata_only(names: &[&str]) {
    for field in ["role", "name", "state", "bounds", "children", "relations"] {
        assert!(
            !names.contains(&field),
            "AccessKit field duplicated: {field}"
        );
    }
    assert_eq!(
        names,
        [
            "address",
            "stability",
            "provenance",
            "action_ids",
            "domain_ref",
            "authored_key"
        ]
    );
}

#[test]
fn envelope_has_six_metadata_fields_and_no_accesskit_node_fields() {
    assert_metadata_only(&fields(include_str!("../envelope.rs")));
}

#[test]
fn schema_guard_catches_each_planted_accesskit_field() {
    for foreign in ["role", "name", "state", "bounds", "children", "relations"] {
        let mut names = fields(include_str!("../envelope.rs"));
        names.push(foreign);
        let failure = std::panic::catch_unwind(|| assert_metadata_only(&names));
        assert!(failure.is_err(), "guard failed to detect {foreign}");
    }
}
