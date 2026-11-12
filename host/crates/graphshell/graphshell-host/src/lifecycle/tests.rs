use super::*;
use crate::admission::{GRAPHSHELL_DOMAIN, PROJECTION_PROTOCOL, admit_session, open_session};
use crate::carrier::projection_policy;
use chirograph::{PresentationManifest, ProjectionSnapshot, ProtocolVersion, SceneSnapshot};
use insigne::delegation::{
    CapabilityScope, DelegationCertificate, DelegationParent, SignedDelegationRevocation,
};
use notochord::{
    CarrierKind, HandshakeLimits, LocalNetworkPolicy, NetworkId, ProfileRef, ProofBinding,
    RequestedAction, SessionClaims, SessionFacts, TrafficClass, TrustedRoot,
};
use personae::IdentityProvider;
use personae::InMemoryProvider;
use personae::delegation::Issue;
use sceno::{InstanceId, Scene};

const ROOT_AUTHORITY: [u8; 32] = [7; 32];
const NETWORK: [u8; 32] = [3; 32];
const NOW_MS: u64 = 50;
const EXPIRY_MS: u64 = 100;

fn owner() -> InMemoryProvider {
    InMemoryProvider::from_seed([1; 32])
}

fn viewer() -> InMemoryProvider {
    InMemoryProvider::from_seed([4; 32])
}

/// A grant to `subject` at `path_prefix`, expiring at [`EXPIRY_MS`].
fn grant(path_prefix: &str) -> SignedDelegationCertificate {
    SignedDelegationCertificate::issue(
        &owner(),
        DelegationCertificate::new(
            DelegationParent::Root(ROOT_AUTHORITY),
            owner().master_public_key().to_bytes(),
            viewer().master_public_key().to_bytes(),
            CapabilityScope {
                domain: GRAPHSHELL_DOMAIN.into(),
                resource: NETWORK.to_vec(),
                path_prefix: path_prefix.into(),
                actions: [CONNECT_ACTION.to_string()].into_iter().collect(),
            },
            5,
            10,
            Some(EXPIRY_MS),
            1,
            [1; 32],
        ),
    )
    .expect("issue certificate")
}

fn principal(session_id: [u8; 32]) -> AdmittedPrincipal {
    AdmittedPrincipal {
        subject: viewer().master_public_key().to_bytes(),
        class: TrafficClass::Interactive,
        session_id,
        action: RequestedAction {
            domain: GRAPHSHELL_DOMAIN.to_string(),
            path: PROJECTION_SERVICE.to_string(),
            action: CONNECT_ACTION.to_string(),
        },
    }
}

fn authority_at(path_prefix: &str, session_id: [u8; 32]) -> SessionAuthority {
    SessionAuthority::retain(principal(session_id), vec![grant(path_prefix)])
}

fn authority() -> SessionAuthority {
    authority_at(PROJECTION_SERVICE, [21; 32])
}

fn admitted_authority() -> AdmittedSession<()> {
    let delegation = grant(PROJECTION_SERVICE);
    AdmittedSession {
        stream: (),
        principal: principal([21; 32]),
        claims: SessionClaims {
            wire_version: 1,
            network: NetworkId(NETWORK),
            profile: ProfileRef {
                id: "mere.base".to_string(),
                revision: 1,
            },
            action: RequestedAction {
                domain: GRAPHSHELL_DOMAIN.to_string(),
                path: PROJECTION_SERVICE.to_string(),
                action: CONNECT_ACTION.to_string(),
            },
            class: TrafficClass::Interactive,
            subject: viewer().master_public_key().to_bytes(),
            delegations: vec![delegation],
        },
        facts: SessionFacts::authenticated(
            b"mere/graphshell/v1",
            CarrierKind::Memory,
            viewer().master_public_key().to_bytes(),
        ),
        limits: HandshakeLimits::default(),
    }
}

fn revoked_ledger(chain: &[SignedDelegationCertificate]) -> RevocationLedger {
    let mut ledger = RevocationLedger::new();
    let statement = SignedDelegationRevocation::issue(
        &owner(),
        insigne::delegation::DelegationRevocation::new(
            chain[0].certificate.id(),
            owner().master_public_key().to_bytes(),
            chain[0].certificate.scope.clone(),
            NOW_MS,
            [2; 32],
        ),
    )
    .expect("issue revocation");
    ledger.fold(statement.check().expect("revocation must check"));
    ledger
}

/// Mount a scene so cache-purge behaviour has something to act on.
fn mount(client: &mut ClientState, session: &ProjectionSession, policy: CachePolicy) {
    let mut snapshot = ProjectionSnapshot {
        version: ProtocolVersion { major: 1, minor: 0 },
        session: session.clone(),
        scene: SceneSnapshot::from_dense(SceneEpoch(1), Revision(1), Scene::new())
            .expect("scene"),
        presentation: PresentationManifest::default(),
        cache_policy: CachePolicy::default(),
    };
    snapshot.cache_policy = policy;
    client.apply_snapshot(snapshot).expect("mount");
}

// --- reconnect ------------------------------------------------------

fn profile() -> ProfileRef {
    ProfileRef {
        id: "mere.base".into(),
        revision: 1,
    }
}

fn policy() -> LocalNetworkPolicy {
    projection_policy(
        NetworkId(NETWORK),
        vec![TrustedRoot {
            authority: ROOT_AUTHORITY,
            issuer: owner().master_public_key().to_bytes(),
        }],
        vec![profile()],
        None,
    )
}

/// The viewer's signed hello carrying `nonce`, bound to `link`.
fn hello_over(link: [u8; 16], nonce: [u8; 32]) -> Vec<u8> {
    let binding = ProofBinding::initiator(PROJECTION_PROTOCOL, None, Some(link));
    open_session(
        &viewer(),
        NetworkId(NETWORK),
        profile(),
        TrafficClass::Interactive,
        nonce,
        &binding,
        vec![grant(PROJECTION_SERVICE)],
    )
    .expect("issue hello")
    .encode(&policy().limits.clamped())
    .expect("encode hello")
}

/// Admit `hello` on a carrier that reports `link`.
fn admitted_over(hello: &[u8], link: [u8; 16]) -> SessionAuthority {
    let facts = SessionFacts::new(PROJECTION_PROTOCOL, CarrierKind::Other)
        .with_ingress(None, Some(link));
    let (_, outcome) = admit_session(
        &policy(),
        &RevocationLedger::default(),
        hello,
        &facts,
        NOW_MS,
        0,
    );
    let principal = outcome.expect("the hello is admitted");
    SessionAuthority::retain(principal, vec![grant(PROJECTION_SERVICE)])
}

#[test]
fn a_reconnect_is_a_new_session_when_the_link_or_the_nonce_is_fresh() {
    let (link, fresh_link) = ([1; 16], [2; 16]);
    let hello = hello_over(link, [5; 32]);

    // Admission keeps no replay state: one hello over one link is one
    // session, however many times it is admitted.
    let first = admitted_over(&hello, link);
    assert_eq!(
        first.session(),
        admitted_over(&hello, link).session(),
        "a reused nonce over a reused link lands on the same session"
    );

    // The proof binds both, so each fresh case is a newly signed hello.
    let relinked = admitted_over(&hello_over(fresh_link, [5; 32]), fresh_link);
    let renonced = admitted_over(&hello_over(link, [6; 32]), link);
    assert_ne!(first.session(), relinked.session());
    assert_ne!(first.session(), renonced.session());

    // The point of deriving the id: state cached under the first session
    // is unreachable from the second, without anyone enforcing a rule.
    let mut client = ClientState::default();
    mount(&mut client, first.session(), CachePolicy::default());
    assert!(client.mounted(first.session()).is_some());
    assert!(client.mounted(relinked.session()).is_none());
}

#[test]
fn endpoint_context_carries_only_the_admitted_session_and_subject() {
    let authority = authority();
    let context = authority.endpoint_context();

    assert_eq!(context.session(), authority.session());
    assert_eq!(context.subject(), authority.principal().subject);
    assert_eq!(context.application(), None, "only a door names one");
    assert_eq!(
        context,
        AdmittedEndpointContext::new(
            authority.session().clone(),
            authority.principal().subject,
        )
    );
}

// --- expiry ---------------------------------------------------------

#[test]
fn authority_holds_until_its_deadline_and_then_reports_when_it_ended() {
    let authority = authority();
    let ledger = RevocationLedger::default();
    assert_eq!(authority.deadline_ms(), Some(EXPIRY_MS));
    assert_eq!(authority.lapse(&ledger, EXPIRY_MS), None);
    assert_eq!(
        authority.lapse(&ledger, EXPIRY_MS + 1),
        // The deadline, not the observation time.
        Some(Lapse::Expired { at_ms: EXPIRY_MS })
    );
    assert_eq!(
        authority.status(&ledger, EXPIRY_MS + 1),
        SessionStatus::Expired
    );
}

// --- revocation -----------------------------------------------------

#[test]
fn a_revoked_grant_lapses_before_its_deadline_and_reports_revocation() {
    let chain = vec![grant(PROJECTION_SERVICE)];
    let authority = SessionAuthority::retain(principal([21; 32]), chain.clone());
    let ledger = revoked_ledger(&chain);
    assert_eq!(authority.lapse(&ledger, NOW_MS), Some(Lapse::Revoked));
    assert_eq!(authority.status(&ledger, NOW_MS), SessionStatus::Revoked);
}

#[test]
fn a_carrier_session_retains_enough_authority_to_notice_revocation() {
    let admitted = admitted_authority();
    let authority = SessionAuthority::retain_admitted(&admitted);
    let ledger = revoked_ledger(&admitted.claims.delegations);

    assert_eq!(authority.lapse(&ledger, NOW_MS), Some(Lapse::Revoked));
    assert_eq!(authority.principal(), &admitted.principal);
}

#[test]
fn revocation_outranks_expiry_when_both_are_true() {
    let chain = vec![grant(PROJECTION_SERVICE)];
    let authority = SessionAuthority::retain(principal([21; 32]), chain.clone());
    let ledger = revoked_ledger(&chain);
    assert_eq!(
        authority.lapse(&ledger, EXPIRY_MS + 1),
        Some(Lapse::Revoked),
        "an owner's withdrawal is the more useful thing to report"
    );
}

// --- cache purge ----------------------------------------------------

#[test]
fn a_lapse_purges_the_cache_when_the_policy_says_so() {
    let authority = authority();
    let mut client = ClientState::default();
    mount(&mut client, authority.session(), CachePolicy::default());

    let purged = apply_lapse(&mut client, authority.session(), Lapse::Revoked);
    assert!(purged);
    assert!(
        client.mounted(authority.session()).is_none(),
        "the scene must be gone, not merely relabelled"
    );
}

#[test]
fn a_retaining_policy_keeps_the_scene_under_an_honest_status() {
    let authority = authority();
    let mut client = ClientState::default();
    mount(
        &mut client,
        authority.session(),
        CachePolicy {
            purge_on_revocation: false,
            ..CachePolicy::default()
        },
    );

    let purged = apply_lapse(&mut client, authority.session(), Lapse::Revoked);
    assert!(!purged);
    assert_eq!(
        client.mounted(authority.session()).map(|m| m.status),
        Some(SessionStatus::Revoked),
        "a retained scene must not still read as Live"
    );
}

// --- denied score ---------------------------------------------------

#[test]
fn a_service_wide_grant_discloses_any_score() {
    let authority = authority();
    let ledger = RevocationLedger::default();
    assert_eq!(authority.authorize_score(&ledger, NOW_MS, "spiral"), Ok(()));
}

#[test]
fn a_single_score_grant_does_not_reach_a_neighbour_that_shares_its_prefix() {
    let authority = authority_at(&score_path("spiral"), [21; 32]);
    let ledger = RevocationLedger::default();
    assert_eq!(authority.authorize_score(&ledger, NOW_MS, "spiral"), Ok(()));
    assert_eq!(
        authority.authorize_score(&ledger, NOW_MS, "spiral-admin"),
        Err(ScoreDenial::NotCovered {
            score: "spiral-admin".to_string()
        }),
        "the scope grammar's `/` boundary is what makes per-score grants safe"
    );
}

#[test]
fn a_lapsed_session_discloses_no_score_at_all() {
    let chain = vec![grant(PROJECTION_SERVICE)];
    let authority = SessionAuthority::retain(principal([21; 32]), chain.clone());
    let ledger = revoked_ledger(&chain);
    assert_eq!(
        authority.authorize_score(&ledger, NOW_MS, "spiral"),
        Err(ScoreDenial::Lapsed(Lapse::Revoked))
    );
}

// --- stale intent ---------------------------------------------------

fn invocation(
    session: &ProjectionSession,
    epoch: SceneEpoch,
    revision: Revision,
) -> IntentInvocation {
    IntentInvocation {
        session: session.clone(),
        target: InstanceId(1),
        observed_epoch: epoch,
        observed_revision: revision,
        intent: "open".to_string(),
        payload: Vec::new(),
    }
}

#[test]
fn an_intent_on_the_current_revision_is_accepted() {
    let authority = authority();
    let result = adjudicate_intent(
        &authority,
        &RevocationLedger::default(),
        NOW_MS,
        &invocation(authority.session(), SceneEpoch(1), Revision(4)),
        SceneEpoch(1),
        Revision(4),
    );
    assert_eq!(result, IntentResult::Accepted);
}

#[test]
fn an_intent_against_a_superseded_revision_is_stale() {
    let authority = authority();
    let result = adjudicate_intent(
        &authority,
        &RevocationLedger::default(),
        NOW_MS,
        &invocation(authority.session(), SceneEpoch(1), Revision(3)),
        SceneEpoch(1),
        Revision(4),
    );
    assert_eq!(
        result,
        IntentResult::Stale {
            current_epoch: SceneEpoch(1),
            current_revision: Revision(4),
        }
    );
}

/// The ordering that matters: a lapsed session is refused outright rather
/// than told what the current revision is.
#[test]
fn a_lapsed_session_is_refused_without_being_told_the_current_revision() {
    let chain = vec![grant(PROJECTION_SERVICE)];
    let authority = SessionAuthority::retain(principal([21; 32]), chain.clone());
    let ledger = revoked_ledger(&chain);
    let result = adjudicate_intent(
        &authority,
        &ledger,
        NOW_MS,
        &invocation(authority.session(), SceneEpoch(1), Revision(3)),
        SceneEpoch(1),
        Revision(4),
    );
    match result {
        IntentResult::Rejected { reason } => {
            assert!(reason.contains("revoked"), "reason was: {reason}");
        },
        other => panic!("a lapsed session must not receive Stale: {other:?}"),
    }
}

#[test]
fn an_intent_naming_another_session_is_rejected() {
    let authority = authority();
    let other = authority_at(PROJECTION_SERVICE, [99; 32]);
    let result = adjudicate_intent(
        &authority,
        &RevocationLedger::default(),
        NOW_MS,
        &invocation(other.session(), SceneEpoch(1), Revision(4)),
        SceneEpoch(1),
        Revision(4),
    );
    assert!(matches!(result, IntentResult::Rejected { .. }));
}
