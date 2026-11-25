// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

use super::*;
use crate::{CarrierOutput, CarrierRequest, CarrierRequestBody, CarrierResponse};

fn request(id: u64) -> Vec<u8> {
    encode(&CarrierRequest {
        id,
        body: CarrierRequestBody::Close,
    })
    .unwrap()
}

#[test]
fn an_item_split_across_two_pushes_arrives_once() {
    let item = request(1);
    let mut items = ItemAssembler::default();
    items.push(&item[..3]);
    assert_eq!(
        items.next_item().unwrap(),
        None,
        "part of an item is not one"
    );
    items.push(&item[3..]);
    assert_eq!(items.next_item().unwrap(), Some(item));
    assert_eq!(items.next_item().unwrap(), None);
    assert!(items.is_empty());
}

#[test]
fn two_items_in_one_push_come_out_as_two() {
    let (a, b) = (request(1), request(2));
    let mut items = ItemAssembler::default();
    items.push(&[a.clone(), b.clone()].concat());
    assert_eq!(items.next_item().unwrap(), Some(a));
    assert_eq!(items.next_item().unwrap(), Some(b));
    assert_eq!(items.next_item().unwrap(), None);
}

#[test]
fn an_item_arriving_a_byte_at_a_time_is_walked_once() {
    let mut nested = Value::from(vec![0u8; 300]);
    for _ in 0..50 {
        nested = Value::map([("inner", nested), ("text", Value::from("ä €"))]);
    }
    let item = nested.encode();
    let mut items = ItemAssembler::default();
    for byte in &item[..item.len() - 1] {
        items.push(&[*byte]);
        assert_eq!(items.next_item().unwrap(), None);
    }
    items.push(&item[item.len() - 1..]);
    assert_eq!(items.next_item().unwrap(), Some(item));
}

#[test]
fn an_indefinite_length_or_stray_break_is_refused() {
    let mut items = ItemAssembler::default();
    items.push(&[0x82, 0x01, 0x02]);
    assert!(items.next_item().unwrap().is_some(), "control");
    for bad in [[0x9f, 0x01, 0xff], [0xff, 0x00, 0x00], [0x1c, 0x00, 0x00]] {
        let mut items = ItemAssembler::default();
        items.push(&bad);
        assert!(
            matches!(items.next_item(), Err(WireError::Malformed(_))),
            "{bad:02x?}"
        );
    }
}

#[test]
fn the_unfinished_tail_is_named_at_end_of_stream() {
    let item = request(1);
    let mut items = ItemAssembler::default();
    items.push(&item);
    assert!(items.next_item().unwrap().is_some());
    assert_eq!(items.finish(), Ok(()), "control");
    items.push(&item[..2]);
    assert_eq!(items.next_item().unwrap(), None);
    assert_eq!(items.finish(), Err(WireError::Truncated(2)));
}

#[test]
fn a_value_round_trips_through_one_item() {
    let output = CarrierOutput::Response(CarrierResponse {
        id: 3,
        body: Ok(crate::CarrierResponseBody::Closed),
    });
    let item = encode(&output).unwrap();
    assert_eq!(decode::<CarrierOutput>(&item).unwrap(), output);
}

#[test]
fn non_canonical_trailing_or_foreign_bytes_are_refused() {
    let item = request(23);
    assert!(decode::<CarrierRequest>(&item).is_ok(), "control");
    // id 23 is written inline as 0x17; a one-byte argument is not shortest.
    let at = item.iter().position(|byte| *byte == 0x17).unwrap();
    let mut longer = item[..at].to_vec();
    longer.extend([0x18, 0x17]);
    longer.extend(&item[at + 1..]);
    assert!(matches!(
        decode::<CarrierRequest>(&longer),
        Err(WireError::Malformed(_))
    ));
    let mut trailing = item.clone();
    trailing.push(0x00);
    assert!(matches!(
        decode::<CarrierRequest>(&trailing),
        Err(WireError::Malformed(_))
    ));
    assert!(matches!(
        decode::<CarrierRequest>(&[0x01]),
        Err(WireError::Value(_))
    ));
}

#[test]
fn a_reader_yields_items_and_names_a_truncated_end() {
    let stream = [request(1), request(2)].concat();
    let mut reader = ItemReader::new(&stream[..]);
    assert_eq!(
        reader
            .next::<CarrierRequest>()
            .unwrap()
            .unwrap()
            .unwrap()
            .id,
        1
    );
    assert_eq!(
        reader
            .next::<CarrierRequest>()
            .unwrap()
            .unwrap()
            .unwrap()
            .id,
        2
    );
    assert!(reader.next::<CarrierRequest>().unwrap().is_none());
    let cut = &stream[..stream.len() - 1];
    let mut reader = ItemReader::new(cut);
    assert!(reader.next_item().unwrap().is_some());
    assert_eq!(
        reader.next_item().unwrap_err().kind(),
        std::io::ErrorKind::InvalidData
    );
}

fn snapshot_output() -> CarrierOutput {
    let scene = sceno::Scene::new();
    CarrierOutput::Response(CarrierResponse {
        id: 5,
        body: Ok(crate::CarrierResponseBody::Snapshot(Box::new(
            crate::ProjectionSnapshot {
                version: crate::ProtocolVersion::V1,
                session: crate::ProjectionSession("fixture".into()),
                scene: crate::SceneSnapshot::from_dense(
                    crate::SceneEpoch(1),
                    crate::Revision(1),
                    scene,
                )
                .unwrap(),
                presentation: Default::default(),
                cache_policy: Default::default(),
            },
        ))),
    })
}

#[test]
fn a_snapshot_on_the_wire_is_its_envelope_and_nothing_else() {
    let output = snapshot_output();
    let item = encode(&output).unwrap();
    assert_eq!(decode::<CarrierOutput>(&item).unwrap(), output, "control");
    let body = &Value::decode(&item).unwrap()["body"]["Ok"]["Snapshot"];
    let envelope = crate::Envelope::read(body.as_bytes().unwrap()).unwrap();
    assert_eq!(envelope.kind, crate::EnvelopeKind::Keyframe);
    // The same response with the snapshot written plainly is refused.
    let CarrierOutput::Response(CarrierResponse {
        body: Ok(crate::CarrierResponseBody::Snapshot(snapshot)),
        ..
    }) = &output
    else {
        unreachable!()
    };
    let mut plain = Value::decode_owned(&item).unwrap();
    plain["body"]["Ok"].insert("Snapshot", Value::serialized(snapshot).unwrap());
    assert!(matches!(
        decode::<CarrierOutput>(&plain.encode()),
        Err(WireError::Value(_))
    ));
}

#[test]
fn the_json_edge_writes_and_reads_the_snapshot_plainly() {
    let output = snapshot_output();
    let json = serde_json::to_value(&output).unwrap();
    assert!(json["body"]["Ok"]["Snapshot"]["presentation"]["bindings"].is_array());
    assert_eq!(
        serde_json::from_value::<CarrierOutput>(json).unwrap(),
        output
    );
}
