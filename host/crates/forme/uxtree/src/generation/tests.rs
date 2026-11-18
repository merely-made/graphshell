// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

use super::*;
use accesskit::{NodeId, TreeId};

#[test]
fn queued_admission_stays_on_its_published_revision_and_placement() {
    let key = (TreeId::ROOT, NodeId(2));
    let other = (TreeId::ROOT, NodeId(3));
    let mut map = GenerationMap::default();
    let old = (4, 10);
    map.publish(HashMap::from([(key, old), (other, old)]), PartialEq::eq);
    let queued = map.get(&key).unwrap().clone();
    let unrelated = map.get(&other).unwrap().clone();
    map.publish(HashMap::from([(key, old), (other, old)]), PartialEq::eq);
    assert!(map.is_current(&key, &queued));
    for changed in [(5, 10), (5, 100), old] {
        let prior = map.get(&key).unwrap().clone();
        map.publish(HashMap::from([(key, changed), (other, old)]), PartialEq::eq);
        assert_eq!(map.get(&key).unwrap().admission, changed);
        assert!(!map.is_current(&key, &prior));
        assert!(!map.is_current(&key, &queued));
        assert!(map.is_current(&other, &unrelated));
    }
    let prior = map.get(&key).unwrap().clone();
    map.publish(HashMap::new(), PartialEq::eq);
    assert!(!map.is_current(&key, &prior));
    map.publish(HashMap::from([(key, old)]), PartialEq::eq);
    assert!(!map.is_current(&key, &prior));
}
