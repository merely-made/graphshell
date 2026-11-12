//! The done-condition, stated as tests: *the native state revision changes
//! only for the admitted intent.*
//!
//! Every row here is the same invocation with one thing changed, and each
//! asserts the revision afterwards — because "was it refused" and "did it
//! change anything anyway" are different questions, and only the second
//! one is the property C4a claims.

use super::*;

fn invocation(
    session: &ProjectionSession,
    revision: Revision,
    intent: &str,
) -> IntentInvocation {
    IntentInvocation {
        session: session.clone(),
        target: InstanceId(0),
        observed_epoch: EPOCH,
        observed_revision: revision,
        intent: intent.to_string(),
        payload: Vec::new(),
    }
}

#[test]
fn the_admitted_intent_advances_the_revision_by_one() {
    let mut endpoint = LiveEndpoint::new();
    let session = endpoint.session().clone();
    let before = endpoint.current_revision();

    let result = endpoint
        .invoke(invocation(&session, before, ADMITTED_INTENT))
        .expect("the session matches");

    assert_eq!(result, IntentResult::Accepted);
    assert_eq!(
        endpoint.current_revision(),
        Revision(before.0 + 1),
        "the admitted intent is the one thing that moves the scene"
    );
}

/// The row that makes the done-condition mean something: an action the
/// endpoint *advertises*, invoked correctly, refused, and the revision
/// standing exactly still.
#[test]
fn the_refused_intent_changes_nothing() {
    let mut endpoint = LiveEndpoint::new();
    let session = endpoint.session().clone();
    let before = endpoint.current_revision();

    let result = endpoint
        .invoke(invocation(&session, before, REFUSED_INTENT))
        .expect("the session matches");

    assert!(
        matches!(result, IntentResult::Rejected { .. }),
        "expected a refusal, got {result:?}"
    );
    assert_eq!(
        endpoint.current_revision(),
        before,
        "a refusal that moved the revision would be an acceptance in disguise"
    );
}

/// Both intents are visible to a peer. A refusal only proves something
/// about policy if the action was on offer in the first place.
#[test]
fn both_intents_are_advertised() {
    let intents: Vec<String> = LiveEndpoint::actions()
        .into_iter()
        .map(|action| action.intent.0)
        .collect();
    assert!(intents.iter().any(|intent| intent == ADMITTED_INTENT));
    assert!(
        intents.iter().any(|intent| intent == REFUSED_INTENT),
        "the refused intent must be advertised, or its refusal proves nothing"
    );
}

/// A stale position is answered as a position problem, not an authority
/// one — and still changes nothing.
#[test]
fn a_stale_invocation_is_told_where_the_scene_is() {
    let mut endpoint = LiveEndpoint::new();
    let session = endpoint.session().clone();
    endpoint
        .invoke(invocation(
            &session,
            endpoint.current_revision(),
            ADMITTED_INTENT,
        ))
        .expect("the session matches");
    let current = endpoint.current_revision();

    let result = endpoint
        .invoke(invocation(&session, Revision(1), ADMITTED_INTENT))
        .expect("the session matches");

    assert_eq!(
        result,
        IntentResult::Stale {
            current_epoch: EPOCH,
            current_revision: current,
        }
    );
    assert_eq!(
        endpoint.current_revision(),
        current,
        "a stale intent changes nothing"
    );
}

/// Each accepted intent leaves a diff a resuming peer can be caught up
/// with, rather than forcing a fresh snapshot.
#[test]
fn a_resuming_peer_is_caught_up_by_diff() {
    let mut endpoint = LiveEndpoint::new();
    let session = endpoint.session().clone();
    let start = endpoint.current_revision();
    for _ in 0..3 {
        endpoint
            .invoke(invocation(
                &session,
                endpoint.current_revision(),
                ADMITTED_INTENT,
            ))
            .expect("the session matches");
    }

    let reply = endpoint
        .resume(ResumeRequest {
            session: session.clone(),
            epoch: EPOCH,
            revision: start,
        })
        .expect("the session matches");

    match reply {
        ResumeReply::Diffs(diffs) => {
            assert_eq!(diffs.len(), 3, "one diff per accepted intent");
            assert_eq!(diffs[0].scene.base, start);
            assert_eq!(diffs[2].scene.revision, endpoint.current_revision());
        },
        other => panic!("expected diffs, got {other:?}"),
    }
}

#[test]
fn a_peer_already_current_is_told_so_rather_than_re_sent() {
    let mut endpoint = LiveEndpoint::new();
    let session = endpoint.session().clone();
    let reply = endpoint
        .resume(ResumeRequest {
            session: session.clone(),
            epoch: EPOCH,
            revision: endpoint.current_revision(),
        })
        .expect("the session matches");
    assert!(matches!(reply, ResumeReply::Current(_)), "{reply:?}");
}

/// Accepting an intent rings the bell exactly once, and a refusal rings
/// nothing — the notice lane carries the same claim as the revision.
#[test]
fn only_an_accepted_intent_rings() {
    let mut endpoint = LiveEndpoint::new();
    let session = endpoint.session().clone();
    assert!(endpoint.poll_notice().expect("no error").is_none());

    endpoint
        .invoke(invocation(
            &session,
            endpoint.current_revision(),
            REFUSED_INTENT,
        ))
        .expect("the session matches");
    assert!(
        endpoint.poll_notice().expect("no error").is_none(),
        "a refusal must not ring"
    );

    endpoint
        .invoke(invocation(
            &session,
            endpoint.current_revision(),
            ADMITTED_INTENT,
        ))
        .expect("the session matches");
    let notice = endpoint.poll_notice().expect("no error").expect("one bell");
    assert_eq!(notice.revision, endpoint.current_revision());
    assert!(
        endpoint.poll_notice().expect("no error").is_none(),
        "the bell is taken once, not re-rung on every poll"
    );
}
