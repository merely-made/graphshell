use super::*;
use p2panda_core::cbor::{decode_cbor, encode_cbor};
use p2panda_core::{Body, Header, SigningKey};

const GRAPH: [u8; 32] = [3; 32];
const NOW_MS: u64 = 1_700_000_000_000;

fn issuer() -> Ed25519Keypair {
    Ed25519Keypair::from_seed([0x51; 32])
}

fn slot() -> BlindedSlotId {
    BlindedSlotId([0xAB; 32])
}

/// A signed carriage operation. The transport writer is deliberately a
/// different key from the issuer: the grammar allows a relay to carry
/// another device's slot, because the lease signature is what authorizes.
fn operation(ext: CarriageExt) -> Operation<CarriageExt> {
    let body = Body::from_bytes(b"wrapped-epoch-record-stand-in");
    let writer = SigningKey::from_bytes(&[0x77; 32]);
    // p2panda 0.7.1 made the header's CBOR cache, size and digest private
    // and folded signing into the builder: `build` encodes, signs and
    // caches the digest in one step, so the struct-literal + `sign` pair
    // has no equivalent. `body` sets payload_size and payload_hash.
    let header = Header::builder()
        .body(body.as_bytes())
        .seq_num(0)
        .backlink(None)
        .build(&writer, ext);
    Operation {
        hash: header.hash(),
        header,
        body: Some(body),
    }
}

fn signed_ext(issue: u64, expires_at_ms: u64) -> CarriageExt {
    let payload = Body::from_bytes(b"wrapped-epoch-record-stand-in").hash();
    CarriageExt {
        graph: GRAPH,
        slot: slot(),
        issue,
        expires_at_ms,
        prune_flag: PruneFlag::new(true),
        issuer_signature: sign_lease(&issuer(), GRAPH, slot(), issue, expires_at_ms, payload),
    }
}

fn policy(held: Option<HashMap<BlindedSlotId, HeldLease>>) -> CarriageAdmissionPolicy {
    CarriageAdmissionPolicy::new(
        GRAPH,
        NOW_MS,
        vec![issuer().public_key().to_bytes()],
        CarriageCeilings::default(),
        held,
    )
}

fn holding(issue: u64) -> HashMap<BlindedSlotId, HeldLease> {
    HashMap::from([(
        slot(),
        HeldLease {
            issue,
            payload: Hash::digest(b"superseded-record"),
            expires_at_ms: NOW_MS + 1_000,
        },
    )])
}

#[test]
fn a_fresh_lease_admits_as_a_destructive_replace_and_erases_the_predecessor() {
    let op = operation(signed_ext(2, NOW_MS + 60_000));
    let admission = policy(Some(holding(1))).admit(&op).unwrap();
    assert!(matches!(
        admission.history,
        stickleback::HistoryAction::PruneBeforeCurrent
    ));
    assert_eq!(
        admission.erase_payloads,
        vec![Hash::digest(b"superseded-record")]
    );
    assert_eq!(
        admission.target.topic,
        Topic::from(carriage_topic(GRAPH)),
        "the slot lands on the carriage topic, never the graph's"
    );
}

#[test]
fn the_refusal_ladder_names_each_failure() {
    let cases: Vec<(&str, Operation<CarriageExt>)> = vec![
        ("carriage-wrong-graph", {
            let mut ext = signed_ext(2, NOW_MS + 60_000);
            ext.graph = [4; 32];
            operation(ext)
        }),
        ("carriage-history-not-pruned", {
            let mut ext = signed_ext(2, NOW_MS + 60_000);
            ext.prune_flag = PruneFlag::new(false);
            operation(ext)
        }),
        ("carriage-lease-expired", operation(signed_ext(2, NOW_MS))),
        (
            "carriage-exceeds-absolute-ceiling",
            operation(signed_ext(2, NOW_MS + CARRIAGE_ABSOLUTE_CEILING_MS + 1)),
        ),
        ("carriage-untrusted-issuer", {
            // Signed by a key nobody trusts. The lease facts are fine.
            let payload = Body::from_bytes(b"wrapped-epoch-record-stand-in").hash();
            let stranger = Ed25519Keypair::from_seed([0x99; 32]);
            let mut ext = signed_ext(2, NOW_MS + 60_000);
            ext.issuer_signature =
                sign_lease(&stranger, GRAPH, slot(), 2, NOW_MS + 60_000, payload);
            operation(ext)
        }),
        (
            "carriage-stale-issue",
            operation(signed_ext(1, NOW_MS + 60_000)),
        ),
    ];
    for (code, op) in cases {
        let reject = policy(Some(holding(1))).admit(&op).unwrap_err();
        assert_eq!(reject.code, code);
    }
}

#[test]
fn an_unknown_held_view_refuses_everything() {
    let op = operation(signed_ext(2, NOW_MS + 60_000));
    let reject = policy(None).admit(&op).unwrap_err();
    assert_eq!(reject.code, "carriage-order-unknown");
}

#[test]
fn the_knowable_ceilings_bind_where_known() {
    let op = operation(signed_ext(2, NOW_MS + 60_000));
    let mut with_ttl = policy(Some(holding(1)));
    with_ttl.ceilings = CarriageCeilings {
        device_max_ttl_ms: Some(30_000),
        grant_expires_at_ms: None,
    };
    assert_eq!(
        with_ttl.admit(&op).unwrap_err().code,
        "carriage-exceeds-device-ttl"
    );
    let mut with_grant = policy(Some(holding(1)));
    with_grant.ceilings = CarriageCeilings {
        device_max_ttl_ms: None,
        grant_expires_at_ms: Some(NOW_MS + 30_000),
    };
    assert_eq!(
        with_grant.admit(&op).unwrap_err().code,
        "carriage-outlives-grant"
    );
}

#[test]
fn the_lease_signature_binds_the_exact_record() {
    // Same lease facts, different body: a kept lease cannot vouch for a
    // swapped record.
    let ext = signed_ext(2, NOW_MS + 60_000);
    assert!(verify_lease(
        &ext,
        Body::from_bytes(b"wrapped-epoch-record-stand-in").hash(),
        &[issuer().public_key().to_bytes()],
    ));
    assert!(!verify_lease(
        &ext,
        Body::from_bytes(b"a-different-record").hash(),
        &[issuer().public_key().to_bytes()],
    ));
}

#[test]
fn the_two_grammars_cannot_be_crossed_by_accident() {
    // A graph-lane extension does not decode as a carriage extension, so
    // no PersonalGraphEvent operation is admissible on the carriage topic
    // even before any policy check runs.
    #[derive(Serialize)]
    struct GraphExtStandIn {
        graph: [u8; 32],
    }
    let bytes = encode_cbor(&GraphExtStandIn { graph: GRAPH }).unwrap();
    let decoded: Result<CarriageExt, _> = decode_cbor(bytes.as_slice());
    assert!(decoded.is_err(), "a graph extension decoded as carriage");
}

#[test]
fn the_purge_proposal_is_pure_reviewable_and_blocked_without_a_view() {
    let mut held = holding(1);
    held.insert(
        BlindedSlotId([0xCD; 32]),
        HeldLease {
            issue: 3,
            payload: Hash::digest(b"expired-record"),
            expires_at_ms: NOW_MS,
        },
    );
    let proposal = propose_carriage_purge(NOW_MS, Some(&held));
    assert!(proposal.is_executable());
    assert_eq!(proposal.expired, vec![BlindedSlotId([0xCD; 32])]);
    assert_eq!(proposal.retain.len(), 1);
    assert_eq!(proposal.retain[0].reason, CarriageRetentionReason::Live);

    let blocked = propose_carriage_purge(NOW_MS, None);
    assert!(!blocked.is_executable());
    assert!(
        blocked.expired.is_empty(),
        "a blocked purge proposes nothing"
    );
    assert_eq!(
        blocked.blockers,
        vec![CarriagePurgeBlocker::HeldViewUnavailable]
    );
}
