// Copyright 2026 Mark Alan Boykin
// SPDX-License-Identifier: MPL-2.0

//! Each phase runs a fresh resident with its own authenticated app connections.
#![cfg(feature = "saved-views")]

use castellan::{
    authority::PersonaeHost,
    custody::{IdentityVault, InMemoryStorage, Profile},
};
use chirograph::{CarrierRequestBody, CarrierResponseBody, IntentInvocation, IntentResult};
use dramatis::view::VaultProtectionView;
use mere_ontology::{SavedView, ViewQuery, WEB_ANNOTATION};
use mere_overmap::saved_views::{SavedViewsClient, SourceMaterial};
use mere_overmap::{
    materialization::{self, GraphRead},
    wire,
};
use mere_resident::{
    applications::grant_for,
    embedded_reservoir::{ReservoirAttachment, ReservoirAttachmentOptions},
    resident_reservoir::mere_client_declaration,
};
use ostiary::{
    app_admission::{AppId, AppRouteId},
    declaration::{AppDeclaration, NamePattern},
};
use pandect::SessionId;
use personae::{Ed25519Keypair, PersonaId, ProfileId};
use std::{path::Path, process::Command, sync::Arc, time::Duration};
use uuid::Uuid;

const LENS: &str = r#"id = "urn:fixture:shared-notes-lens"
version = "0.1.0"
kind = ["lens"]
licence = "MPL-2.0"
[lens]
select = [{pack = "https://mere.computer/packs/web-annotation", version = "0.1.0"}]
"#;
const NAME: &str = "Rehearsal note targets";
const NOTE: &str = "https://example.test/Notes/Mix%2BOne";
const HAS_TARGET: &str = "http://www.w3.org/ns/oa#hasTarget";
const FIRST: &str = "https://example.test/score#bar=7";
const SECOND: &str = "https://example.test/score#bar=9";
const SOURCE: &str = "urn:uuid:a0000000-0000-4000-8000-000000000001";
const OTHER: &str = "urn:uuid:a0000000-0000-4000-8000-000000000002";
const DOMAIN: &str = "rehearsal-notes";
const WRITER: &str = "turnstone";
const READER: &str = "graphshell";
const CLI: &str = "mere-reservoir-view";

#[test]
fn admitted_apps_reopen_live_views_after_owner_replacement() {
    let root = tempfile::tempdir().unwrap();
    for phase in ["create", "reopen", "withdrawn"] {
        let report = Command::new(std::env::current_exe().unwrap())
            .args(["--ignored", "--exact", "resident_view_child", "--nocapture"])
            .env("RESIDENT_VIEW_ROOT", root.path())
            .env("RESIDENT_VIEW_PHASE", phase)
            .output()
            .unwrap();
        assert!(
            report.status.success(),
            "phase {phase}: {}\n{}",
            String::from_utf8_lossy(&report.stdout),
            String::from_utf8_lossy(&report.stderr)
        );
    }
}

fn declaration() -> AppDeclaration {
    AppDeclaration {
        routes: vec![NamePattern::exact("mere/rehearsal-notes")],
        ..Default::default()
    }
}

async fn connect(
    root: &Path,
    app: &str,
    session: Option<SessionId>,
) -> Result<SavedViewsClient, String> {
    SavedViewsClient::connect_existing(
        &options(root, WRITER).endpoint,
        AppId::new(app),
        declaration(),
        DOMAIN,
        session,
    )
    .await
}

fn source(id: &str, revision: &str, address: &str) -> SourceMaterial {
    SourceMaterial {
        id: id.into(),
        revision: revision.into(),
        statements: vec![target(address)],
    }
}

fn options(root: &Path, app: &str) -> ReservoirAttachmentOptions {
    let owner_declaration = mere_client_declaration();
    ReservoirAttachmentOptions {
        endpoint: format!(
            "/tmp/mv-{}.sock",
            root.file_name().unwrap().to_string_lossy()
        ),
        application: AppId::new(app),
        grants: [READER, CLI]
            .into_iter()
            .map(|app| grant_for(&AppId::new(app), &declaration(), None))
            .collect(),
        declaration: owner_declaration,
        shared_root: root.join("shared"),
        persona: PersonaId::from_uuid(Uuid::from_u128(0xee01)),
        domain: pandect::DomainId::new("rehearsal-notes").unwrap(),
        session_duration: Duration::from_secs(600),
    }
}

async fn session_action(
    app: &mut ReservoirAttachment,
    name: &str,
    intent: &str,
    action: wire::SessionsActionV1,
) {
    let client = app.client_mut();
    let state = materialization::read(client, name, GraphRead::Complete)
        .await
        .unwrap();
    let snapshot = state.sessions;
    let response = client
        .request_body(CarrierRequestBody::Intent(IntentInvocation {
            session: snapshot.session,
            target: sceno::InstanceId(0),
            observed_epoch: snapshot.scene.epoch,
            observed_revision: snapshot.scene.revision,
            intent: intent.into(),
            payload: chirograph::wire::encode(&action).unwrap(),
        }))
        .await
        .unwrap();
    assert!(
        matches!(
            response,
            CarrierResponseBody::Intent(IntentResult::Accepted)
        ),
        "{response:?}"
    );
}

async fn command(root: &Path, session: SessionId, args: &[&str]) -> String {
    let output = tokio::process::Command::new(env!("CARGO_BIN_EXE_mere-reservoir-view"))
        .env(
            ostiary::app_admission::APP_ENDPOINT_ENV,
            options(root, CLI).endpoint,
        )
        .args(["rehearsal-notes", &session.as_uuid().to_string()])
        .args(args)
        .output()
        .await
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn target(address: &str) -> [String; 3] {
    [NOTE.to_owned(), HAS_TARGET.to_owned(), address.to_owned()]
}

fn identity() -> Arc<dyn dramatis_resident::custody_host::ResidentCustody> {
    Arc::new(PersonaeHost::new(
        IdentityVault::with_profile(
            InMemoryStorage::new(),
            Profile::new(
                ProfileId("fixture".into()),
                "Public fixture",
                Ed25519Keypair::from_seed([0x51; 32]),
            ),
        ),
        None,
        VaultProtectionView::Ephemeral,
    ))
}

#[test]
#[ignore = "fresh resident process launched by admitted_apps_reopen_live_views_after_owner_replacement"]
fn resident_view_child() {
    let root = std::path::PathBuf::from(std::env::var_os("RESIDENT_VIEW_ROOT").unwrap());
    let phase = std::env::var("RESIDENT_VIEW_PHASE").unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let mut writer =
            ReservoirAttachment::open(options(&root, WRITER), || async { Ok(identity()) })
                .await
                .unwrap();
        assert!(writer.is_embedded());
        assert!(connect(&root, "ungranted-fixture", None).await.is_err());
        assert!(
            connect(&root, READER, Some(SessionId(Uuid::from_u128(0x919))))
                .await
                .is_err()
        );
        let profile = mere_ontology::load(LENS, &[WEB_ANNOTATION], None).unwrap();
        let state_file = root.join("session-and-revision.json");
        if phase == "create" {
            let mut owner = connect(&root, WRITER, None).await.unwrap();
            let session = owner.session();
            let view = SavedView::new(
                NAME.into(),
                owner.scope().into(),
                vec![SOURCE.into(), OTHER.into()],
                ViewQuery::Outgoing {
                    record: NOTE.into(),
                },
                &profile,
            )
            .unwrap();
            let sources = [
                source(SOURCE, "record:r1", FIRST),
                source(OTHER, "score:r3", FIRST),
            ];
            assert!(owner.share(&view, &profile, &sources[..1]).await.is_err());
            assert!(owner.list().await.unwrap().is_empty());
            assert!(
                owner
                    .share_if(&view, &profile, &sources, || false)
                    .await
                    .is_err()
            );
            assert!(owner.list().await.unwrap().is_empty());
            owner.share(&view, &profile, &sources).await.unwrap();
            let mut held = graphshell_door::projection::AppBrokerClient::open_declared_route_at(
                &options(&root, WRITER).endpoint,
                AppId::new(READER),
                AppRouteId::new("mere/rehearsal-notes").unwrap(),
                declaration(),
            )
            .await
            .unwrap();
            let mut shared = SavedViewsClient::borrow_held(&mut held, READER, DOMAIN, session)
                .await
                .unwrap();
            assert_eq!(shared.list().await.unwrap(), [NAME.to_owned()]);
            assert_eq!(shared.open(NAME).await.unwrap(), view);
            let portable = command(&root, session, &["open", NAME]).await;
            assert_eq!(SavedView::from_json(&portable).unwrap(), view);
            let rows = shared.evaluate(NAME, &profile).await.unwrap();
            assert_eq!(rows.len(), 2);
            assert_eq!(rows[0].source, SOURCE);
            assert_eq!(rows[1].source, OTHER);
            assert_eq!(rows[0].statements, rows[1].statements);
            assert_eq!(rows[0].statements[0].subject, NOTE);
            assert_eq!(rows[0].statements[0].predicate, HAS_TARGET);
            assert_eq!(rows[0].statements[0].object, FIRST);
            assert_eq!(rows[0].revision.as_deref(), Some("record:r1"));
            assert_eq!(rows[1].revision.as_deref(), Some("score:r3"));
            assert!(
                owner
                    .disclose(&source(SOURCE, "record:r1", SECOND), &profile)
                    .await
                    .is_err()
            );
            let alternate_profile = mere_ontology::load(
                &LENS.replace("urn:fixture:shared-notes-lens", "urn:fixture:other-lens"),
                &[WEB_ANNOTATION],
                None,
            )
            .unwrap();
            let alternate = SavedView::new(
                "Separate lens".into(),
                owner.scope().into(),
                vec![SOURCE.into()],
                view.query.clone(),
                &alternate_profile,
            )
            .unwrap();
            owner.save(&alternate, &alternate_profile).await.unwrap();
            assert!(
                shared
                    .evaluate(&alternate.name, &alternate_profile)
                    .await
                    .unwrap()[0]
                    .revision
                    .is_none()
            );
            owner
                .share(
                    &alternate,
                    &alternate_profile,
                    &[source(SOURCE, "record:r1", "urn:alternate:target")],
                )
                .await
                .unwrap();
            assert_eq!(
                shared
                    .evaluate(&alternate.name, &alternate_profile)
                    .await
                    .unwrap()[0]
                    .statements[0]
                    .object,
                "urn:alternate:target"
            );
            assert_eq!(
                shared.evaluate(NAME, &profile).await.unwrap()[0].statements[0].object,
                FIRST
            );
            owner
                .disclose(&source(SOURCE, "record:r2", SECOND), &profile)
                .await
                .unwrap();
            let changed = shared.evaluate(NAME, &profile).await.unwrap();
            assert!(
                shared
                    .evaluate(&alternate.name, &alternate_profile)
                    .await
                    .unwrap()[0]
                    .revision
                    .is_none()
            );
            assert_ne!(changed[0].revision, rows[0].revision);
            assert_eq!(changed[0].statements[0].object, SECOND);
            assert_eq!(changed[1], rows[1]);
            let revision = changed[0].revision.as_ref().unwrap();
            std::fs::write(
                state_file,
                serde_json::to_vec(&(session, revision)).unwrap(),
            )
            .unwrap();
            session_action(
                &mut writer,
                WRITER,
                wire::MINT_SESSION_INTENT,
                wire::SessionsActionV1::mint(Some("Separate notes".into())),
            )
            .await;
            let other =
                materialization::read(writer.client_mut(), WRITER, GraphRead::ProjectionOnly)
                    .await
                    .unwrap()
                    .sessions
                    .scene
                    .tables
                    .sources
                    .iter()
                    .flatten()
                    .filter(|source| source.adapter == wire::SESSIONS_SOURCE)
                    .filter_map(|source| Uuid::parse_str(&source.id).ok())
                    .find(|id| id != session.as_uuid())
                    .map(SessionId)
                    .unwrap();
            let mut separate = connect(&root, READER, Some(other)).await.unwrap();
            assert_ne!(separate.session(), session);
            assert!(separate.list().await.unwrap().is_empty());
            assert!(separate.open(NAME).await.is_err());
            assert!(separate.save(&view, &profile).await.is_err());
            assert!(separate.evaluate(NAME, &profile).await.is_err());
        } else {
            let (session, revision): (SessionId, String) =
                serde_json::from_slice(&std::fs::read(state_file).unwrap()).unwrap();
            let mut owner = connect(&root, WRITER, Some(session)).await.unwrap();
            let mut shared = connect(&root, READER, Some(session)).await.unwrap();
            assert_eq!(shared.open(NAME).await.unwrap().sources, [SOURCE, OTHER]);
            if phase == "reopen" {
                let results = shared.evaluate(NAME, &profile).await.unwrap();
                assert_eq!(results[0].revision.as_deref(), Some(revision.as_str()));
                assert_eq!(results[0].statements[0].object, SECOND);
                owner.withdraw(SOURCE).await.unwrap();
            }
            let results = shared.evaluate(NAME, &profile).await.unwrap();
            assert_eq!(results[0].source, SOURCE);
            assert_eq!(results[0].revision, None);
            assert!(results[0].statements.is_empty());
            assert_eq!(results[1].source, OTHER);
            assert_eq!(results[1].statements[0].object, FIRST);
            let lens = root.join("lens");
            std::fs::create_dir_all(&lens).unwrap();
            std::fs::write(lens.join("lens.toml"), LENS).unwrap();
            let output = command(&root, session, &["run", NAME, lens.to_str().unwrap()]).await;
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&output).unwrap(),
                serde_json::to_value(&results).unwrap()
            );
            assert!(results[1].revision.is_some());
        }
        writer.close().await.unwrap();
    });
}

/// Isolated owner for cross-binary product qualification. Only the parent harness
/// reads readiness and requests shutdown. Both apps reach records through their doors.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "started explicitly by the native Turnstone-to-reservoir qualification"]
async fn serve_turnstone_view_fixture() {
    let root = std::path::PathBuf::from(
        std::env::var_os("RESIDENT_VIEW_FIXTURE_ROOT").expect("parent-owned private directory"),
    );
    assert!(root.is_dir() && !root.join("ready.json").exists() && !root.join("stop").exists());
    const FIXTURE: &str = "linked-views-fixture";
    let mut start = options(&root, FIXTURE);
    start
        .grants
        .push(grant_for(&AppId::new(WRITER), &declaration(), None));
    let endpoint = start.endpoint.clone();
    let owner = ReservoirAttachment::open(start, || async { Ok(identity()) })
        .await
        .unwrap();
    assert!(owner.is_embedded());
    let client = connect(&root, FIXTURE, None).await.unwrap();
    let message = serde_json::json!({"endpoint":endpoint,"domain":DOMAIN,"session":client.session(),"scope":client.scope()});
    std::fs::write(
        root.join("ready.tmp"),
        serde_json::to_vec(&message).unwrap(),
    )
    .unwrap();
    std::fs::rename(root.join("ready.tmp"), root.join("ready.json")).unwrap();
    client.close().await.unwrap();
    tokio::time::timeout(Duration::from_secs(600), async {
        while !root.join("stop").exists() {
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    })
    .await
    .expect("cross-binary fixture shutdown deadline");
    owner.close().await.unwrap();
}
