use super::*;
use graphshell_client::ResolvedContent;

#[test]
fn loopback_resolves_rich_and_compact_profiles() {
    let run = run_loopback_canary().unwrap();
    assert!(matches!(
        run.rich[0].content,
        ResolvedContent::PortableCard(_)
    ));
    assert!(matches!(run.rich[1].content, ResolvedContent::Image { .. }));
    assert!(matches!(
        run.compact[0].content,
        ResolvedContent::NativeGlyph(_)
    ));
    assert_eq!(run.compact[1].content, ResolvedContent::LabeledPlaceholder);
}

#[test]
fn loopback_discloses_the_remote_viewers_backdrop() {
    let mut endpoint = FixtureEndpoint::new();
    let snapshot = endpoint.snapshot(endpoint.request()).unwrap();
    let backdrops = snapshot.scene.active_backdrops_in_order();
    assert_eq!(backdrops.len(), 1);
    assert_eq!(backdrops[0].1.kind, "graphshell:remote-grid");
}

#[test]
fn both_profiles_keep_advertised_actions_accessible() {
    let run = run_loopback_canary().unwrap();
    for tree in [&run.rich_accessibility, &run.compact_accessibility] {
        assert_eq!(tree.children.len(), 2);
        assert_eq!(tree.children[0].actions[0].label, "Open field note");
        assert_eq!(tree.children[1].actions[0].label, "Inspect map tile");
    }
}

#[test]
fn fixture_rechecks_the_advertised_choice_before_accepting_it() {
    let mut endpoint = FixtureEndpoint::new();
    let snapshot = endpoint.snapshot(endpoint.request()).unwrap();
    let action = snapshot
        .presentation
        .offers
        .get(&PresentationKey("fixture:map".into()))
        .expect("map offer")
        .first()
        .expect("map presentation")
        .semantics
        .actions
        .first()
        .expect("map action");
    let payload = action
        .compose_payload(&BTreeMap::from([(
            "inspection_scope".to_string(),
            "coordinates".to_string(),
        )]))
        .expect("advertised choice composes");
    let invoke = |payload| IntentInvocation {
        session: snapshot.session.clone(),
        target: InstanceId(1),
        observed_epoch: snapshot.scene.epoch,
        observed_revision: snapshot.scene.revision,
        intent: action.intent.0.clone(),
        payload,
    };
    assert_eq!(
        endpoint.invoke(invoke(payload)).unwrap(),
        IntentResult::Accepted
    );
    let fresh = endpoint.snapshot(endpoint.request()).unwrap();
    assert!(matches!(
        endpoint
            .invoke(IntentInvocation {
                session: fresh.session.clone(),
                target: InstanceId(1),
                observed_epoch: fresh.scene.epoch,
                observed_revision: fresh.scene.revision,
                intent: action.intent.0.clone(),
                payload: Vec::new(),
            })
            .unwrap(),
        IntentResult::Rejected { .. }
    ));
}

#[test]
fn fixture_refuses_a_live_view_when_the_recipient_cannot_read_its_source() {
    let mut endpoint = FixtureEndpoint::new();
    let before = endpoint
        .snapshot(endpoint.request())
        .expect("fixture snapshot");
    let accepted = endpoint
        .invoke(IntentInvocation {
            session: before.session.clone(),
            target: InstanceId(0),
            observed_epoch: before.scene.epoch,
            observed_revision: before.scene.revision,
            intent: OPEN_LIVE_VIEW_INTENT.to_string(),
            payload: LiveViewReferenceV1::new(FIXTURE_PUBLIC_LIVE_VIEW)
                .encode()
                .expect("public reference serializes"),
        })
        .expect("the endpoint admits its disclosed record");
    assert_eq!(accepted, IntentResult::Accepted);
    let result = endpoint
        .invoke(IntentInvocation {
            session: before.session.clone(),
            target: InstanceId(0),
            observed_epoch: before.scene.epoch,
            observed_revision: before.scene.revision,
            intent: OPEN_LIVE_VIEW_INTENT.to_string(),
            payload: LiveViewReferenceV1::new("eidetic:private-view")
                .encode()
                .expect("reference serializes"),
        })
        .expect("the endpoint answers with a bounded refusal");
    assert_eq!(
        result,
        IntentResult::Rejected {
            reason: "the recipient is not authorized to read this live-view source".to_string()
        }
    );
    let after = endpoint
        .snapshot(endpoint.request())
        .expect("fixture remains live");
    assert_eq!(
        after.scene.tables, before.scene.tables,
        "a refusal never substitutes an empty scene"
    );
}
