// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0
//! Acceptance preparation through ordinary admitted application routes.
//! No direct store writes and no co-op invitation or edit API is used here.
use chirograph::*;
use graphshell_door::projection::AppBrokerClient;
use mere_overmap::wire::*;
use ostiary::app_admission::{AppId, AppRouteId};
use kernel::graph::CapturedDelta;
use sceno::InstanceId;
use serde_json::json;

async fn invoke(
    c: &mut AppBrokerClient,
    s: &ProjectionSnapshot,
    target: InstanceId,
    intent: &str,
    payload: Vec<u8>,
) {
    let reply = c
        .request_body(CarrierRequestBody::Intent(IntentInvocation {
            session: s.session.clone(),
            target,
            observed_epoch: s.scene.epoch,
            observed_revision: s.scene.revision,
            intent: intent.into(),
            payload,
        }))
        .await
        .unwrap();
    assert!(
        matches!(reply, CarrierResponseBody::Intent(IntentResult::Accepted)),
        "{reply:?}"
    );
}
async fn open(route: &str) -> (AppBrokerClient, EndpointDescriptor) {
    let mut c = AppBrokerClient::open_declared_route(
        AppId::new(graphshell_door::declaration::APP),
        AppRouteId::new(route).unwrap(),
        graphshell_door::declaration::declaration(),
    )
    .await
    .unwrap();
    let d = c.open_session().await.unwrap().descriptor;
    (c, d)
}
async fn attach(c: &mut AppBrokerClient, d: &EndpointDescriptor, id: pandect::SessionId) {
    let current = c.snapshot(d.projections[1].request.clone()).await.unwrap();
    if current.scene.tables.sources.iter().flatten().any(|source| {
        source.adapter == SESSION_ITEM_SOURCE && source.id == id.as_uuid().to_string()
    }) {
        return;
    }
    let s = c.snapshot(d.projections[0].request.clone()).await.unwrap();
    let Some(intent) = mere_overmap::route::intent(
        &mere_view::MereViewRequest::Session(id, mere_view::SessionStep::Switch),
        &s,
    ) else {
        let current = c.snapshot(d.projections[1].request.clone()).await.unwrap();
        assert!(
            current
                .scene
                .tables
                .sources
                .iter()
                .flatten()
                .any(|source| source.adapter == SESSION_ITEM_SOURCE
                    && source.id == id.as_uuid().to_string()),
            "the requested session is neither attached nor attachable"
        );
        return;
    };
    let reply = c
        .request_body(CarrierRequestBody::Intent(intent))
        .await
        .unwrap();
    assert!(
        matches!(reply, CarrierResponseBody::Intent(IntentResult::Accepted)),
        "{reply:?}"
    );
}
async fn graph_view(
    command: &str,
    domain: &str,
    id: pandect::SessionId,
    app: &str,
    choices: &[String],
) {
    assert!(matches!(app, "graphshell" | "turnstone"), "fixture app");
    let route = format!("mere/{domain}");
    let declaration = if app == graphshell_door::declaration::APP {
        graphshell_door::declaration::declaration()
    } else {
        ostiary::declaration::AppDeclaration {
            routes: vec![ostiary::declaration::NamePattern::exact(&route)],
            ..Default::default()
        }
    };
    let mut client = AppBrokerClient::open_declared_route(
        AppId::new(app),
        AppRouteId::new(route).unwrap(),
        declaration,
    )
    .await
    .unwrap();
    let descriptor = client.open_session().await.unwrap().descriptor;
    attach(&mut client, &descriptor, id).await;
    let request = descriptor.projections[1].request.clone();
    let mut snapshot = client.snapshot(request.clone()).await.unwrap();
    let view = format!("{app}.mere");
    if command == "view" {
        let [focus, layout] = choices else {
            panic!("view DOMAIN SESSION APP FOCUS_URL LAYOUT");
        };
        let state = incipit::view::ViewIntent {
            strategy: Some(layout.clone()),
            ..Default::default()
        };
        let target = view_item(&snapshot);
        invoke(
            &mut client,
            &snapshot,
            target,
            SET_VIEW_INTENT,
            serde_json::to_vec(&SetViewV1::new(&view, state)).unwrap(),
        )
        .await;
        snapshot = client.snapshot(request.clone()).await.unwrap();
        let state = incipit::view::AttentionState {
            focus: Some(focus.clone()),
            ..Default::default()
        };
        let target = view_item(&snapshot);
        invoke(
            &mut client,
            &snapshot,
            target,
            SET_ATTENTION_INTENT,
            serde_json::to_vec(&SetAttentionV1::new(&view, state)).unwrap(),
        )
        .await;
        snapshot = client.snapshot(request).await.unwrap();
    } else {
        assert!(choices.is_empty(), "view-read DOMAIN SESSION APP");
    }
    let offer = snapshot
        .presentation
        .offers_for(view_item(&snapshot))
        .unwrap();
    let bytes = client
        .resource(snapshot.session.clone(), offer[0].resource)
        .await
        .unwrap();
    let card = serde_json::from_slice::<PortableCardV1>(&bytes).unwrap();
    let bytes = client
        .resource(snapshot.session.clone(), card.media[0])
        .await
        .unwrap();
    let material = serde_json::from_slice::<SessionGraphV1>(&bytes).unwrap();
    assert_eq!(material.session, id);
    assert_eq!(material.application, app);
    println!(
        "{}",
        json!({
            "session": id.as_uuid().to_string(),
            "app": app,
            "focus": material.attention.get(&view).and_then(|state|state.focus.as_deref()),
            "layout": material.views.get(&view).and_then(|state|state.strategy.as_deref()),
        }),
    );
}

fn view_item(snapshot: &ProjectionSnapshot) -> InstanceId {
    snapshot
        .scene
        .active_items_in_order()
        .into_iter()
        .find(|(_, item)| {
            snapshot.scene.tables.sources[item.source.0 as usize]
                .as_ref()
                .is_some_and(|source| source.adapter == SESSION_ITEM_SOURCE)
        })
        .map(|(instance, _)| instance)
        .expect("the graph must disclose one attached session item")
}

fn turnstone_coop_declaration(domain: &str) -> ostiary::declaration::AppDeclaration {
    use ostiary::declaration::{
        AppDeclaration, CustodyOperation, CustodyUse, NamePattern, SaltPattern, SaltUse,
    };
    let salt = std::str::from_utf8(graphshell_host::coop::TRANSPORT_SALT).unwrap();
    AppDeclaration {
        routes: vec![
            NamePattern::exact(castellan_client::custody::CUSTODY_ROUTE),
            NamePattern::exact(format!("mere/{domain}")),
            NamePattern::exact(format!("coop/{domain}")),
        ],
        custody: CustodyUse {
            operations: [
                CustodyOperation::Status,
                CustodyOperation::WatchLock,
                CustodyOperation::Release,
                CustodyOperation::Attest,
            ]
            .into(),
            salts: vec![SaltUse::persona(SaltPattern::Exact(salt.into()))],
            ..Default::default()
        },
        ..Default::default()
    }
}

#[cfg(test)]
#[test]
fn turnstone_fixture_grants_only_this_mere_and_its_peer_transport() {
    let grant = mere_resident::applications::grant_for(
        &AppId::new("turnstone"),
        &turnstone_coop_declaration("acceptance"),
        None,
    );
    let routes = serde_json::to_value(&grant.routes).unwrap();
    assert_eq!(
        routes,
        json!([{"exact": "custody"}, {"exact": "mere/acceptance"}, {"exact": "coop/acceptance"}])
    );
    assert!(grant.custody.acts.is_empty());
    assert_eq!(
        serde_json::to_value(&grant.custody.salts).unwrap(),
        json!([{"source": "persona", "name": {"exact": "graphshell/endpoint-session/coop/v1"}}])
    );
}

#[tokio::main]
async fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let domain = args.get(1).map(String::as_str).unwrap_or("coop-acceptance");
    if args[0] == "grant" {
        // What an owner's yes writes for this declaration (D25), from the
        // resident's own `grant_for`; the harness puts it in the owner
        // settings' `applications` before Djinn starts (engine-split D130).
        let (application, declaration) = match args.get(1).map(String::as_str) {
            None => (
                graphshell_door::declaration::APP,
                graphshell_door::declaration::declaration(),
            ),
            Some("turnstone") => (
                "turnstone",
                turnstone_coop_declaration(args.get(2).expect("grant turnstone DOMAIN")),
            ),
            _ => panic!("grant | grant turnstone DOMAIN"),
        };
        let grant =
            mere_resident::applications::grant_for(&AppId::new(application), &declaration, None);
        println!("{}", serde_json::to_string(&grant).unwrap());
    } else if args[0] == "seed" {
        let (mut reservoir, descriptor) =
            open(mere_resident::resident_reservoir::RESIDENT_RESERVOIR_ROUTE).await;
        let s = reservoir
            .snapshot(descriptor.projections[0].request.clone())
            .await
            .unwrap();
        invoke(
            &mut reservoir,
            &s,
            InstanceId(0),
            mere_resident::resident_reservoir::RESERVOIR_ENSURE_MERE_INTENT,
            serde_json::to_vec(&mere_resident::resident_reservoir::EnsureMereV1::new(
                domain,
            ))
            .unwrap(),
        )
        .await;
        let (mut c, d) = open(&format!("mere/{domain}")).await;
        let mut ids = vec![];
        for (i, url) in [
            "https://private.invalid/owner-only",
            "https://shared.invalid/existing",
        ]
        .iter()
        .enumerate()
        {
            if i > 0 {
                let s = c.snapshot(d.projections[0].request.clone()).await.unwrap();
                let intent =
                    mere_overmap::route::intent(&mere_view::MereViewRequest::Mint, &s).unwrap();
                let r = c
                    .request_body(CarrierRequestBody::Intent(intent))
                    .await
                    .unwrap();
                assert!(matches!(
                    r,
                    CarrierResponseBody::Intent(IntentResult::Accepted)
                ));
                let listed = c.snapshot(d.projections[0].request.clone()).await.unwrap();
                let minted = listed
                    .scene
                    .tables
                    .sources
                    .iter()
                    .flatten()
                    .filter(|source| source.adapter == SESSIONS_SOURCE)
                    .filter_map(|source| uuid::Uuid::parse_str(&source.id).ok())
                    .find(|id| !ids.contains(id))
                    .expect("mint disclosed a new session");
                attach(&mut c, &d, pandect::SessionId(minted)).await;
            }
            let s = c.snapshot(d.projections[1].request.clone()).await.unwrap();
            let session_item = s
                .scene
                .active_items_in_order()
                .into_iter()
                .find_map(|(instance, item)| {
                    s.scene.tables.sources[item.source.0 as usize]
                        .as_ref()
                        .filter(|source| source.adapter == SESSION_ITEM_SOURCE)
                        .map(|source| (instance, source.id.clone()))
                })
                .unwrap();
            let id = uuid::Uuid::parse_str(&session_item.1).unwrap();
            ids.push(id);
            let edits = vec![CapturedDelta::ReplayAddNodeWithIdIfMissing {
                id: uuid::Uuid::new_v4().to_string(),
                url: url.to_string(),
                position: [i as f32 * 150., 0.],
            }];
            invoke(
                &mut c,
                &s,
                session_item.0,
                APPLY_EDITS_INTENT,
                serde_json::to_vec(&ApplyEditsV1::new(edits)).unwrap(),
            )
            .await;
        }
        assert_ne!(
            ids[0], ids[1],
            "the private and chosen graphs must be distinct"
        );
        println!("{}", json!({"private":ids[0],"chosen":ids[1]}));
    } else if args[0] == "read" {
        let id = pandect::SessionId(uuid::Uuid::parse_str(&args[2]).unwrap());
        let (mut c, d) = open(&format!("mere/{domain}")).await;
        attach(&mut c, &d, id).await;
        let s = c.snapshot(d.projections[1].request.clone()).await.unwrap();
        let mut cards = vec![];
        let mut graphs = vec![];
        for offer in s.presentation.offers.values().flatten() {
            let bytes = c.resource(s.session.clone(), offer.resource).await.unwrap();
            let card = serde_json::from_slice::<PortableCardV1>(&bytes).unwrap();
            for media in &card.media {
                let bytes = c.resource(s.session.clone(), *media).await.unwrap();
                if let Ok(graph) = serde_json::from_slice::<SessionGraphV1>(&bytes) {
                    graphs.push(graph);
                }
            }
            cards.push(card);
        }
        println!("{}", json!({"snapshot":s,"cards":cards,"graphs":graphs}));
    } else if matches!(args[0].as_str(), "view" | "view-read") {
        graph_view(
            &args[0],
            domain,
            pandect::SessionId(uuid::Uuid::parse_str(&args[2]).unwrap()),
            &args[3],
            &args[4..],
        )
        .await;
    } else if args[0] == "ordinary-add" {
        let id = pandect::SessionId(uuid::Uuid::parse_str(&args[2]).unwrap());
        let (mut c, d) = open(&format!("mere/{domain}")).await;
        attach(&mut c, &d, id).await;
        let s = c.snapshot(d.projections[1].request.clone()).await.unwrap();
        let target = s
            .scene
            .active_items_in_order()
            .into_iter()
            .find_map(|(instance, item)| {
                s.scene.tables.sources[item.source.0 as usize]
                    .as_ref()
                    .filter(|source| source.adapter == SESSION_ITEM_SOURCE)
                    .map(|_| instance)
            })
            .unwrap();
        let from = s
            .scene
            .tables
            .sources
            .iter()
            .flatten()
            .find(|source| source.adapter == "mere.graph")
            .unwrap()
            .id
            .clone();
        let added = uuid::Uuid::new_v4();
        invoke(
            &mut c,
            &s,
            target,
            APPLY_EDITS_INTENT,
            serde_json::to_vec(&ApplyEditsV1::new(vec![
                CapturedDelta::ReplayAddNodeWithIdIfMissing {
                    id: added.to_string(),
                    url: args[3].clone(),
                    position: [150., 150.],
                },
                CapturedDelta::ReplayAssertRelationByIds {
                    from_id: from,
                    to_id: added.to_string(),
                    assertion: mere::model::EdgeAssertion::Semantic {
                        sub_kind: mere::model::SemanticSubKind::Cites,
                        label: None,
                    },
                },
            ]))
            .unwrap(),
        )
        .await;
        println!("saved");
    } else {
        panic!(
            "grant [turnstone DOMAIN] | seed DOMAIN | read DOMAIN UUID | ordinary-add DOMAIN UUID URL | view DOMAIN SESSION APP FOCUS_URL LAYOUT | view-read DOMAIN SESSION APP"
        )
    }
}
