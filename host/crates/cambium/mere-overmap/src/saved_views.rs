// Copyright 2026 Mark Alan Boykin
// SPDX-License-Identifier: MPL-2.0

//! Named linked-data questions in a selected, owner-granted resident session.
//! Each read rematerializes that session. Every write uses its checked journal.

mod records;
pub use records::SourceMaterial;

use crate::{
    materialization::{self, GraphRead},
    route, wire,
};
use chirograph::{
    CarrierRequestBody, CarrierResponseBody, ContentHash, IntentInvocation, IntentResult,
    PortableCardV1, ProjectionSnapshot,
};
use graphshell_door::projection::AppBrokerClient;
use kernel::graph::{CapturedDelta, Graph};
use mere_ontology::{Profile, SavedView, SourceResult};
use ostiary::{
    app_admission::{AppId, AppRouteId},
    declaration::AppDeclaration,
};
use pandect::{DomainId, SessionId};
use sceno::InstanceId;
use std::{
    borrow::BorrowMut,
    collections::{BTreeSet, HashMap},
};
use uuid::Uuid;

pub struct SavedViewsClient<C = AppBrokerClient> {
    client: C,
    application: String,
    domain: String,
    session: SessionId,
    scope: String,
}

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn owner_id(
    snapshot: &ProjectionSnapshot,
    cards: &HashMap<ContentHash, Vec<u8>>,
    domain: &str,
) -> Result<Uuid, String> {
    let catalog_id = format!("mere:{domain}");
    let (instance, _) = route::items(snapshot, wire::SESSIONS_SOURCE)
        .find(|(_, source)| source.id == catalog_id)
        .ok_or("Missing explicit mere catalog card")?;
    let hash = route::card_hash(snapshot, instance)?;
    let card: PortableCardV1 =
        serde_json::from_slice(cards.get(&hash).ok_or("Missing verified mere catalog")?)
            .map_err(err)?;
    let value = card
        .values
        .iter()
        .find(|v| v.label == wire::MERE_ID_LABEL)
        .ok_or("Resident did not disclose this mere's owner-bound ID")?;
    Uuid::parse_str(&value.value).map_err(err)
}

fn scope(mere: Uuid, session: SessionId) -> String {
    format!("urn:mere:saved-views:{mere}:{}", session.as_uuid())
}

async fn invoke(
    client: &mut AppBrokerClient,
    snapshot: &ProjectionSnapshot,
    target: InstanceId,
    intent: &str,
    payload: Vec<u8>,
) -> Result<(), String> {
    let answer = client
        .request_body(CarrierRequestBody::Intent(IntentInvocation {
            session: snapshot.session.clone(),
            target,
            observed_epoch: snapshot.scene.epoch,
            observed_revision: snapshot.scene.revision,
            intent: intent.into(),
            payload,
        }))
        .await
        .map_err(err)?;
    match answer {
        CarrierResponseBody::Intent(IntentResult::Accepted) => Ok(()),
        CarrierResponseBody::Intent(result) => {
            Err(format!("Resident refused {intent}: {result:?}"))
        },
        _ => Err(format!("Unexpected response to {intent}")),
    }
}

fn session_card(
    catalog: &ProjectionSnapshot,
    cards: &HashMap<ContentHash, Vec<u8>>,
    session: SessionId,
) -> Result<(InstanceId, PortableCardV1), String> {
    let id = session.as_uuid().to_string();
    let (instance, _) = route::items(catalog, wire::SESSIONS_SOURCE)
        .find(|(_, s)| s.id == id)
        .ok_or("The owner did not offer the requested session")?;
    let hash = route::card_hash(catalog, instance)?;
    let card: PortableCardV1 =
        serde_json::from_slice(cards.get(&hash).ok_or("Missing verified session card")?)
            .map_err(err)?;
    Ok((instance, card))
}

async fn select(client: &mut AppBrokerClient, session: SessionId) -> Result<(), String> {
    let (catalog, cards) = materialization::catalog(client).await?;
    let (instance, card) = session_card(&catalog, &cards, session)?;
    // The owner offers attach only on the other sessions. The subsequent complete
    // materialization independently checks the requested session UUID.
    if !card.badges.iter().any(|badge| badge == "Attached") {
        invoke(
            client,
            &catalog,
            instance,
            wire::ATTACH_SESSION_INTENT,
            chirograph::wire::encode(&wire::SessionsActionV1::on(session)).map_err(err)?,
        )
        .await?;
    }
    Ok(())
}

impl SavedViewsClient {
    /// Open the caller's existing explicit domain route on a running resident.
    /// An optional session is selected through the ordinary catalog intent.
    pub async fn connect_existing(
        endpoint: &str,
        application: AppId,
        declaration: AppDeclaration,
        domain: &str,
        session: Option<SessionId>,
    ) -> Result<Self, String> {
        let domain = DomainId::new(domain).map_err(err)?;
        let route =
            wire::RemoteMereRequest::explicit(domain.as_str(), application.as_str()).route_id();
        let app = application.as_str().to_owned();
        let mut client = AppBrokerClient::open_declared_route_at(
            endpoint,
            application,
            AppRouteId::new(route).map_err(err)?,
            declaration,
        )
        .await
        .map_err(err)?;
        if let Some(selected) = session {
            select(&mut client, selected).await?;
        }
        Self::bind(client, &app, domain.as_str(), session).await
    }

    /// Borrow the pane's active app connection for an operation without disturbing its attachment.
    pub async fn borrow_held<'a>(
        client: &'a mut AppBrokerClient,
        application: &str,
        domain: &str,
        session: SessionId,
    ) -> Result<SavedViewsClient<&'a mut AppBrokerClient>, String> {
        SavedViewsClient::from_held(client, application, domain, session).await
    }

    pub async fn close(self) -> Result<(), String> {
        self.client.close().await.map_err(err)
    }
}

impl<C: BorrowMut<AppBrokerClient>> SavedViewsClient<C> {
    /// Reuse an active application connection without opening a route or switching its session.
    pub async fn from_held(
        mut client: C,
        application: &str,
        domain: &str,
        session: SessionId,
    ) -> Result<Self, String> {
        let (catalog, cards) = materialization::catalog(client.borrow_mut()).await?;
        let (_, card) = session_card(&catalog, &cards, session)?;
        if !card.badges.iter().any(|badge| badge == "Attached") {
            return Err("The requested session is not attached on this connection".into());
        }
        Self::bind(client, application, domain, Some(session)).await
    }

    async fn bind(
        mut client: C,
        application: &str,
        domain: &str,
        session: Option<SessionId>,
    ) -> Result<Self, String> {
        let state =
            materialization::read(client.borrow_mut(), application, GraphRead::Complete).await?;
        let held = state
            .graph_material
            .ok_or("The resident has no attached graph")?
            .session;
        if session.is_some_and(|selected| selected != held) {
            return Err("The owner did not attach the requested session".into());
        }
        let mere = owner_id(&state.sessions, &state.cards, domain)?;
        Ok(Self {
            client,
            application: application.into(),
            domain: domain.into(),
            session: held,
            scope: scope(mere, held),
        })
    }

    pub fn scope(&self) -> &str {
        &self.scope
    }
    pub fn session(&self) -> SessionId {
        self.session
    }
    pub fn into_inner(self) -> C {
        self.client
    }

    async fn fresh(&mut self) -> Result<(Graph, ProjectionSnapshot, bool), String> {
        let state = materialization::read(
            self.client.borrow_mut(),
            &self.application,
            GraphRead::Complete,
        )
        .await?;
        let material = state
            .graph_material
            .ok_or("The owner no longer offers the attached graph")?;
        let mere = owner_id(&state.sessions, &state.cards, &self.domain)?;
        if scope(mere, material.session) != self.scope {
            return Err("The owner or attached session has changed".into());
        }
        let read_only = material.read_only;
        Ok((material.into_graph()?, state.graph, read_only))
    }

    async fn edit(
        &mut self,
        prepare: impl FnOnce(&Graph) -> Result<Vec<CapturedDelta>, String>,
        permitted: impl FnOnce() -> bool,
    ) -> Result<(), String> {
        let (graph, snapshot, read_only) = self.fresh().await?;
        if read_only {
            return Err("Attach a writable session before editing saved views".into());
        }
        let edits = prepare(&graph)?;
        let payload = if edits.is_empty() {
            None
        } else {
            Some(
                chirograph::wire::encode(&wire::ApplyEditsV1::checked(snapshot.scene.revision, edits))
                    .map_err(err)?,
            )
        };
        if !permitted() {
            return Err("Permission to share changed before the edit was sent".into());
        }
        let Some(payload) = payload else {
            return Ok(());
        };
        invoke(
            self.client.borrow_mut(),
            &snapshot,
            InstanceId(0),
            wire::APPLY_EDITS_INTENT,
            payload,
        )
        .await
    }

    pub async fn list(&mut self) -> Result<Vec<String>, String> {
        let (graph, _, _) = self.fresh().await?;
        records::list(&graph, &self.scope)
    }
    pub async fn open(&mut self, name: &str) -> Result<SavedView, String> {
        let (graph, _, _) = self.fresh().await?;
        records::open(&graph, &self.scope, name)
    }
    pub async fn evaluate(
        &mut self,
        name: &str,
        profile: &Profile,
    ) -> Result<Vec<SourceResult>, String> {
        let (graph, _, _) = self.fresh().await?;
        records::open(&graph, &self.scope, name)?.evaluate_loaded(&self.scope, profile, |id| {
            records::index(&graph, id, profile)
        })
    }
    pub async fn save(&mut self, view: &SavedView, profile: &Profile) -> Result<(), String> {
        view.check_scope(&self.scope, profile)?;
        self.edit(|graph| records::save(graph, view), || true).await
    }
    /// Publish the definition and exactly its selected source graphs together.
    pub async fn share(
        &mut self,
        view: &SavedView,
        profile: &Profile,
        sources: &[SourceMaterial],
    ) -> Result<(), String> {
        self.share_if(view, profile, sources, || true).await
    }
    /// Recheck the application's current authority immediately before submitting the complete edit.
    pub async fn share_if(
        &mut self,
        view: &SavedView,
        profile: &Profile,
        sources: &[SourceMaterial],
        permitted: impl FnOnce() -> bool,
    ) -> Result<(), String> {
        view.check_scope(&self.scope, profile)?;
        let expected: BTreeSet<_> = view.sources.iter().map(String::as_str).collect();
        let received: BTreeSet<_> = sources.iter().map(|s| s.id.as_str()).collect();
        if sources.len() != received.len() || received != expected {
            return Err("Share requires each selected source exactly once".into());
        }
        self.edit(
            |graph| {
                let mut edits = Vec::new();
                for source in sources {
                    edits.extend(records::disclose(graph, source, profile)?);
                }
                edits.extend(records::save(graph, view)?);
                Ok(edits)
            },
            permitted,
        )
        .await
    }
    pub async fn disclose(
        &mut self,
        source: &SourceMaterial,
        profile: &Profile,
    ) -> Result<(), String> {
        self.edit(|graph| records::disclose(graph, source, profile), || true)
            .await
    }
    pub async fn withdraw(&mut self, source: &str) -> Result<(), String> {
        self.edit(|graph| records::withdraw(graph, source), || true)
            .await
    }
}
