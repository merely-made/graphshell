use super::*;

const EXPIRES_AT_MS: u64 = 1_000;
const FOUNDER: [u8; 32] = [7; 32];

/// A store-free driver: the state machine both consumers implement over
/// their own commands, with nothing behind it.
struct Reference {
    caps: Capabilities,
    now_ms: u64,
    installed: bool,
    left: bool,
    revoked: bool,
    /// Reports Joined whatever happened, to prove the harness catches it.
    lies_after_revoke: bool,
}

impl Reference {
    fn new(caps: Capabilities) -> Self {
        Self {
            caps,
            now_ms: 50,
            installed: false,
            left: false,
            revoked: false,
            lies_after_revoke: false,
        }
    }

    fn expired(&self) -> bool {
        self.now_ms > EXPIRES_AT_MS
    }

    fn verdict(&self) -> Verdict {
        if !self.installed {
            return if self.left {
                Verdict::Left
            } else {
                Verdict::NotJoined
            };
        }
        if self.revoked {
            Verdict::Revoked
        } else if self.expired() {
            Verdict::Expired
        } else {
            Verdict::Joined
        }
    }
}

impl LifecycleDriver for Reference {
    fn invite_wrong_target(&mut self) -> Result<(), String> {
        Err("this activity's membership does not contain the invited root".into())
    }

    fn invite(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn join(&mut self) -> Result<(), String> {
        self.installed = true;
        self.left = false;
        Ok(())
    }

    fn report(&mut self) -> LifecycleReport {
        let verdict = if self.lies_after_revoke && self.revoked {
            Verdict::Joined
        } else {
            self.verdict()
        };
        let reason = match verdict {
            Verdict::Joined => None,
            Verdict::Left => Some("not joined: this peer left the activity".into()),
            Verdict::NotJoined => Some("not joined: no invitation is installed".into()),
            Verdict::Expired => Some(format!(
                "expired: this peer's grant ended at {EXPIRES_AT_MS} ms; clock is {} ms",
                self.now_ms
            )),
            Verdict::Revoked => Some("revoked: the issuer revoked this peer's grant".into()),
            Verdict::NotAdmitted => Some("not admitted here".into()),
        };
        let mut report = LifecycleReport::new(verdict, reason, self.now_ms);
        if self.installed {
            report.membership = Some(Membership {
                members: 2,
                access: Access::Write,
            });
            report.grant = Some(Grant {
                expires_at_ms: Some(EXPIRES_AT_MS),
                expired_at_ms: self.expired().then_some(EXPIRES_AT_MS),
            });
        }
        if self.revoked {
            report.revocation = Some(Revocation {
                by: Some(FOUNDER),
                at_ms: Some(self.now_ms),
            });
            report.reading = if self.caps.cuts_reading_on_revoke {
                Reading::Cut
            } else {
                Reading::Continues
            };
        }
        report
    }

    fn leave(&mut self) -> Result<(), String> {
        self.installed = false;
        self.left = true;
        Ok(())
    }

    fn rejoin(&mut self) -> Result<(), String> {
        self.join()
    }

    fn restart(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn reconnect(&mut self) -> Result<(), String> {
        if !self.installed {
            return Err("not joined: nothing is installed here".into());
        }
        if self.revoked {
            return Err("revoked: the issuer revoked this peer's grant".into());
        }
        if self.expired() && self.caps.grant_is_all_authority {
            return Err(format!(
                "expired: this peer's grant ended at {EXPIRES_AT_MS} ms"
            ));
        }
        Ok(())
    }

    fn expire(&mut self) -> Result<(), String> {
        self.now_ms = EXPIRES_AT_MS + 1;
        Ok(())
    }

    fn revoke(&mut self) -> Result<(), String> {
        self.revoked = true;
        Ok(())
    }

    fn replay_duplicate(&mut self) -> Result<(), String> {
        Ok(())
    }
}

/// The practice fixture's profile: one grant is all authority, reading
/// survives revocation, duplicate replay is receipted, clock is fixed.
fn fixture_profile() -> Capabilities {
    Capabilities {
        grant_is_all_authority: true,
        cuts_reading_on_revoke: false,
        receipts_duplicate_replay: true,
        clock: Clock::Fixed,
    }
}

/// Turnstone's profile: membership outlives the grant, revocation rotates
/// the group so reading is cut, duplicate replay is not separately
/// receipted. Its clock is the system's in product; this store-free
/// driver advances deterministically, so it declares Fixed.
fn turnstone_profile() -> Capabilities {
    Capabilities {
        grant_is_all_authority: false,
        cuts_reading_on_revoke: true,
        receipts_duplicate_replay: false,
        clock: Clock::Fixed,
    }
}

#[test]
fn the_reference_driver_conforms_under_the_fixture_profile() {
    let caps = fixture_profile();
    let conformance = conform(&mut Reference::new(caps), &caps).expect("fixture profile");
    assert_eq!(
        conformance.at(Step::Join).map(|report| report.verdict),
        Some(Verdict::Joined)
    );
    assert_eq!(
        conformance.at(Step::Revoke).map(|report| report.reading),
        Some(Reading::Continues)
    );
    assert!(conformance.at(Step::ReplayDuplicate).is_some());
}

#[test]
fn the_reference_driver_conforms_under_the_turnstone_profile() {
    let caps = turnstone_profile();
    let conformance = conform(&mut Reference::new(caps), &caps).expect("turnstone profile");
    assert_eq!(
        conformance.at(Step::Revoke).map(|report| report.reading),
        Some(Reading::Cut)
    );
    // Not declared, so the walk does not reach it.
    assert!(conformance.at(Step::ReplayDuplicate).is_none());
}

#[test]
fn every_step_is_walked_in_order() {
    let caps = fixture_profile();
    let conformance = conform(&mut Reference::new(caps), &caps).expect("fixture profile");
    let walked: Vec<_> = conformance
        .steps
        .iter()
        .map(|(step, _)| step.name())
        .collect();
    assert_eq!(
        walked,
        [
            "fresh",
            "invite_wrong_target",
            "join",
            "leave",
            "rejoin",
            "reconnect",
            "expire",
            "reconnect_after_expiry",
            "revoke",
            "replay_duplicate",
        ]
    );
}

#[test]
fn a_driver_that_reports_joined_after_revoke_fails_at_the_revoke_step() {
    let caps = turnstone_profile();
    let mut driver = Reference::new(caps);
    driver.lies_after_revoke = true;
    let failure = conform(&mut driver, &caps).expect_err("the lie must be caught");
    assert_eq!(failure.step, Step::Revoke);
    assert!(failure.mismatch.contains("joined"), "{failure}");
}

#[test]
fn a_declared_capability_does_not_skip_the_verdict_check() {
    // Declaring "reading is cut" while the driver says it continues is a
    // failure, not a waiver.
    let caps = Capabilities {
        cuts_reading_on_revoke: true,
        ..fixture_profile()
    };
    let failure = conform(&mut Reference::new(fixture_profile()), &caps)
        .expect_err("the declaration must be checked");
    assert_eq!(failure.step, Step::Revoke);
    assert!(failure.mismatch.contains("Cut"), "{failure}");
}

#[test]
fn a_grant_that_is_all_authority_must_refuse_reconnect_after_expiry() {
    // The driver behaves like Turnstone; the declaration says fixture.
    let caps = fixture_profile();
    let failure = conform(&mut Reference::new(turnstone_profile()), &caps)
        .expect_err("an admitted reconnect must fail here");
    assert_eq!(failure.step, Step::ReconnectAfterExpiry);
    assert!(failure.mismatch.contains("refusal was due"), "{failure}");
}

#[test]
fn joined_has_no_reason_and_every_other_verdict_has_one() {
    assert!(
        LifecycleReport::new(Verdict::Joined, None, 50)
            .check_invariants()
            .is_ok()
    );
    assert!(
        LifecycleReport::new(Verdict::Joined, Some("because".into()), 50)
            .check_invariants()
            .is_err()
    );
    for verdict in [
        Verdict::Left,
        Verdict::Expired,
        Verdict::Revoked,
        Verdict::NotJoined,
        Verdict::NotAdmitted,
    ] {
        assert!(
            LifecycleReport::new(verdict, None, 50)
                .check_invariants()
                .is_err(),
            "{} must name a reason",
            verdict.name()
        );
        assert!(
            LifecycleReport::new(verdict, Some("  ".into()), 50)
                .check_invariants()
                .is_err(),
            "{} must name a non-empty reason",
            verdict.name()
        );
        assert!(
            LifecycleReport::new(verdict, Some("named".into()), 50)
                .check_invariants()
                .is_ok()
        );
    }
}

fn pinned_report() -> LifecycleReport {
    LifecycleReport {
        version: VERSION,
        verdict: Verdict::Revoked,
        reason: Some("revoked: the issuer revoked this peer's delegation".into()),
        membership: Some(Membership {
            members: 2,
            access: Access::Write,
        }),
        grant: Some(Grant {
            expires_at_ms: Some(1_000),
            expired_at_ms: None,
        }),
        revocation: Some(Revocation {
            by: None,
            at_ms: Some(1_001),
        }),
        reading: Reading::Continues,
        now_ms: 1_001,
    }
}

#[test]
fn a_report_round_trips_through_serde() {
    let report = pinned_report();
    let json = serde_json::to_string(&report).expect("serialize");
    let back: LifecycleReport = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(report, back);
}

/// Pinned wire shape. Changing this string is a version bump.
const PINNED_JSON: &str = r#"{"version":1,"verdict":"revoked","reason":"revoked: the issuer revoked this peer's delegation","membership":{"members":2,"access":"write"},"grant":{"expires_at_ms":1000,"expired_at_ms":null},"revocation":{"by":null,"at_ms":1001},"reading":"continues","now_ms":1001}"#;

#[test]
fn the_wire_shape_is_pinned() {
    assert_eq!(
        serde_json::to_string(&pinned_report()).unwrap(),
        PINNED_JSON
    );
    assert_eq!(
        serde_json::from_str::<LifecycleReport>(PINNED_JSON).unwrap(),
        pinned_report()
    );
}

#[test]
fn a_conformance_walk_round_trips_through_serde() {
    let caps = fixture_profile();
    let conformance = conform(&mut Reference::new(caps), &caps).expect("fixture profile");
    let json = serde_json::to_string(&conformance).expect("serialize");
    let back: Conformance = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(conformance, back);
}
