use super::*;
use mere_surface_api::settings::{
    SettingMovement, SettingMutability, SettingScope, SettingSecurity,
};

fn spec_with(label: &str, value: &str) -> SettingSpec {
    SettingSpec {
        id: "unsettled.setting".into(),
        label: label.into(),
        scope: SettingScope::Application,
        movement: SettingMovement::LocalOnly,
        mutability: SettingMutability::Live,
        security: SettingSecurity::Ordinary,
        control: SettingControl::Text,
        value: SettingValue::Text(value.into()),
    }
}

fn spec(value: &str) -> SettingSpec {
    spec_with("Unsettled setting", value)
}

fn snapshot() -> SettingRowSnapshot {
    SettingRowSession::new(spec("a")).snapshot()
}

fn text(value: &str) -> SettingValue {
    SettingValue::Text(value.into())
}

fn without_revision(snapshot: SettingRowSnapshot) -> SettingRowSnapshot {
    SettingRowSnapshot {
        revision: 0,
        ..snapshot
    }
}

/// Run `call`; the revision must advance by exactly one and the rest of the
/// snapshot must differ.
fn advances<R>(session: &SettingRowSession, call: impl FnOnce() -> R) -> R {
    let before = session.snapshot();
    let result = call();
    let after = session.snapshot();
    assert_eq!(
        after.revision,
        before.revision + 1,
        "revision advances by one"
    );
    assert_ne!(without_revision(after), without_revision(before));
    result
}

/// Run `call`; the whole snapshot, revision included, must be unchanged.
fn stays<R>(session: &SettingRowSession, call: impl FnOnce() -> R) -> R {
    let before = session.snapshot();
    let result = call();
    assert_eq!(
        session.snapshot(),
        before,
        "revision and snapshot are unchanged"
    );
    result
}

fn dirty_session() -> SettingRowSession {
    let session = SettingRowSession::new(spec("a"));
    session.record_edit(text("b")).unwrap();
    session
}

#[test]
fn unsettled_is_dirty_or_pending() {
    for (dirty, pending, unsettled) in [
        (false, None, false),
        (true, None, true),
        (false, Some(1), true),
        (true, Some(1), true),
    ] {
        let row = SettingRowSnapshot {
            dirty,
            pending_generation: pending,
            ..snapshot()
        };
        assert_eq!(
            row.is_unsettled(),
            unsettled,
            "dirty {dirty}, pending {pending:?}"
        );
    }
}

#[test]
fn a_new_session_starts_at_revision_zero() {
    assert_eq!(snapshot().revision, 0);
}

#[test]
fn observe_advances_only_when_the_snapshot_changes() {
    let clean = SettingRowSession::new(spec("a"));
    stays(&clean, || clean.observe(spec("a")));
    advances(&clean, || clean.observe(spec("z")));
    advances(&clean, || clean.observe(spec_with("Renamed", "z")));
    stays(&clean, || clean.observe(spec_with("Renamed", "z")));

    let dirty = dirty_session();
    stays(&dirty, || dirty.observe(spec("a")));
    advances(&dirty, || dirty.observe(spec("provider")));
    stays(&dirty, || dirty.observe(spec("provider")));

    let applying = dirty_session();
    let generation = applying.snapshot().edit_generation;
    assert!(applying.begin_apply(generation));
    stays(&applying, || {
        applying.observe(applying.snapshot().observed_spec)
    });
    advances(&applying, || applying.observe(spec("other")));
}

#[test]
fn reset_advances_once_and_reset_to_observed_follows() {
    let session = dirty_session();
    advances(&session, || session.reset(spec("a")));
    advances(&session, || session.reset(spec("a")));

    advances(&session, || session.observe(spec("provider")));
    advances(&session, || session.reset_to_observed());
}

#[test]
fn saturated_generations_make_a_reset_a_no_op_for_the_revision() {
    let session = SettingRowSession::new(spec("a"));
    {
        let mut state = session.0.borrow_mut();
        state.edit_generation = u64::MAX;
        state.reset_generation = u64::MAX;
    }
    stays(&session, || session.reset(spec("a")));
    stays(&session, || session.reset_to_observed());
}

#[test]
fn revision_saturates_like_the_generations() {
    let session = SettingRowSession::new(spec("a"));
    session.0.borrow_mut().revision = u64::MAX - 1;
    session.record_edit(text("b")).unwrap();
    assert_eq!(session.snapshot().revision, u64::MAX);
    session.record_edit(text("c")).unwrap();
    assert_eq!(session.snapshot().revision, u64::MAX);
}

#[test]
fn record_edit_advances_only_for_a_real_change() {
    let session = SettingRowSession::new(spec("a"));
    stays(&session, || {
        assert_eq!(session.record_edit(text("a")), None)
    });
    stays(&session, || {
        assert_eq!(session.record_edit(SettingValue::Boolean(true)), None)
    });
    advances(&session, || {
        assert_eq!(session.record_edit(text("b")), Some(1))
    });
    stays(&session, || {
        assert_eq!(session.record_edit(text("b")), None)
    });
    advances(&session, || {
        assert_eq!(session.record_edit(text("a")), Some(2))
    });
    assert!(!session.snapshot().dirty);
}

#[test]
fn begin_apply_and_acknowledge_advance_only_when_admitted() {
    let session = dirty_session();
    let generation = session.snapshot().edit_generation;
    stays(&session, || assert!(!session.begin_apply(generation + 1)));
    advances(&session, || assert!(session.begin_apply(generation)));
    stays(&session, || assert!(!session.begin_apply(generation)));

    stays(&session, || {
        assert_eq!(session.acknowledge(generation + 1, spec("b")), None)
    });
    advances(&session, || {
        assert_eq!(
            session.acknowledge(generation, spec("b")),
            Some(SettingRowAckDisposition::Clean)
        )
    });
    assert!(!session.snapshot().is_unsettled());
}

#[test]
fn acknowledge_keeping_a_newer_draft_advances_once() {
    let session = dirty_session();
    let generation = session.snapshot().edit_generation;
    assert!(session.begin_apply(generation));
    session.record_edit(text("c")).unwrap();
    advances(&session, || {
        assert_eq!(
            session.acknowledge(generation, spec("b")),
            Some(SettingRowAckDisposition::KeepNewerDraft)
        )
    });
}

#[test]
fn fail_advances_unless_it_repeats_the_same_failure() {
    let session = dirty_session();
    let generation = session.snapshot().edit_generation;
    assert!(session.begin_apply(generation));
    advances(&session, || {
        session.fail(SettingRowFailureKind::Persistence, "disk full")
    });
    stays(&session, || {
        session.fail(SettingRowFailureKind::Persistence, "disk full")
    });
    advances(&session, || {
        session.fail(SettingRowFailureKind::Persistence, "disk gone")
    });
    stays(&session, || {
        session.fail(SettingRowFailureKind::Validation, "disk gone")
    });
    advances(&session, || {
        session.fail(SettingRowFailureKind::Conflict, "disk gone")
    });
}

#[test]
fn unavailable_and_resume_advance_only_on_a_change() {
    let session = dirty_session();
    stays(&session, || assert!(!session.resume_available()));
    advances(&session, || session.set_unavailable("owner gone"));
    stays(&session, || session.set_unavailable("owner gone"));
    advances(&session, || session.set_unavailable("owner moved"));
    advances(&session, || assert!(session.resume_available()));
    stays(&session, || assert!(!session.resume_available()));
}

#[test]
fn review_mine_advances_only_out_of_a_conflict() {
    let session = dirty_session();
    stays(&session, || assert!(!session.review_mine()));
    let generation = session.snapshot().edit_generation;
    assert!(session.begin_apply(generation));
    session.fail(SettingRowFailureKind::Conflict, "changed elsewhere");
    advances(&session, || assert!(session.review_mine()));
    stays(&session, || assert!(!session.review_mine()));
}

#[test]
fn equal_revisions_mean_equal_snapshots_across_a_mixed_sequence() {
    let session = SettingRowSession::new(spec("a"));
    let mut seen = vec![session.snapshot()];
    let steps: Vec<Box<dyn Fn(&SettingRowSession)>> = vec![
        Box::new(|s| drop(s.record_edit(text("b")))),
        Box::new(|s| drop(s.record_edit(text("b")))),
        Box::new(|s| drop(s.observe(spec("p")))),
        Box::new(|s| drop(s.begin_apply(s.snapshot().edit_generation))),
        Box::new(|s| s.fail(SettingRowFailureKind::Conflict, "x")),
        Box::new(|s| s.fail(SettingRowFailureKind::Conflict, "x")),
        Box::new(|s| drop(s.review_mine())),
        Box::new(|s| s.reset_to_observed()),
    ];
    for step in steps {
        step(&session);
        seen.push(session.snapshot());
    }
    for pair in seen.windows(2) {
        let same_revision = pair[0].revision == pair[1].revision;
        let same_content = without_revision(pair[0].clone()) == without_revision(pair[1].clone());
        assert_eq!(same_revision, same_content);
    }
}
