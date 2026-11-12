// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Product endpoints selected by one resident Graphshell host.
//!
//! A catalog is an in-process composition seam. The host admits the carrier,
//! derives an [`AdmittedEndpointContext`], selects one registered endpoint,
//! and keeps its authority loop around that endpoint. Registrations receive no
//! vault, delegation, or transport: only the already-admitted session and
//! subject they must bind into product truth.

use std::collections::BTreeMap;
use std::fmt::Display;

use chirograph::{
    CarrierNotice, EndpointDescriptor, IntentInvocation, IntentResult, ProjectionRequest,
    ProjectionSnapshot, ResourceChunkRequest, ResourceChunkResponse, ResourceRequest,
    ResourceResponse, ResumeReply, ResumeRequest, RetainedPagesRequest, RetainedPagesResponse,
};
use graphshell_endpoint::{
    IntentSink, PresentationSource, ProjectionCatalog, ProjectionNoticeSource, ProjectionSource,
    ResumableProjectionSource,
};

use crate::lifecycle::AdmittedEndpointContext;

/// One endpoint prepared for a resident admitted session.
///
/// This object-safe boundary is the catalog's type erasure. Product adapters
/// ordinarily enter through [`ResidentEndpointCatalog::register`] or
/// [`ResidentEndpointCatalog::register_notifying`], which preserve their
/// ordinary Graphshell endpoint implementations without exposing error types
/// across product registrations.
pub trait ResidentEndpoint: Send {
    /// Complete any per-session setup outside the catalog lock.
    fn prepare(&mut self) -> Result<(), String> {
        Ok(())
    }
    /// Whether this endpoint must run on a bounded blocking worker.
    fn requires_blocking(&self) -> bool {
        false
    }
    fn cancellation(&self) -> Option<tokio::sync::watch::Receiver<bool>> {
        None
    }
    /// Whether the owner still permits this endpoint to serve.
    fn is_live(&self) -> bool {
        true
    }
    fn describe(&self) -> EndpointDescriptor;
    fn snapshot(&mut self, request: ProjectionRequest) -> Result<ProjectionSnapshot, String>;
    fn resource(&mut self, request: ResourceRequest) -> Result<ResourceResponse, String>;

    fn resource_chunk(
        &mut self,
        request: ResourceChunkRequest,
    ) -> Result<ResourceChunkResponse, String> {
        let whole = self.resource(ResourceRequest {
            session: request.session,
            resource: request.resource,
        })?;
        Ok(ResourceChunkResponse::slice(
            &whole,
            request.offset,
            request.length,
        ))
    }

    fn invoke(&mut self, intent: IntentInvocation) -> Result<IntentResult, String>;

    /// The page text this endpoint has retained for the session the client
    /// holds. Defaulted: most endpoints keep none.
    fn retained_pages(
        &mut self,
        _request: RetainedPagesRequest,
    ) -> Result<RetainedPagesResponse, String> {
        Err("this endpoint retains no page text".to_string())
    }

    /// A product that does not emit revision bells remains a valid resident
    /// endpoint. A notifying registration overrides this through the typed
    /// adapter below.
    fn poll_notice(&mut self) -> Result<Option<CarrierNotice>, String> {
        Ok(None)
    }

    /// Resume is product-specific. Registrations that need it can supply a
    /// hand-written [`ResidentEndpoint`] rather than make every endpoint claim
    /// a diff history it does not hold.
    fn resume(&mut self, _request: ResumeRequest) -> Result<ResumeReply, String> {
        Err("endpoint does not support projection resume".to_string())
    }
}

type Factory =
    dyn FnMut(&AdmittedEndpointContext) -> Result<Box<dyn ResidentEndpoint>, String> + Send;

struct Registration {
    owner: Option<u64>,
    label: String,
    factory: Box<Factory>,
}

/// A host-visible registered endpoint. The id is a local routing key, not a
/// browser-supplied authority claim.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResidentEndpointOffer {
    pub id: String,
    pub label: String,
}

/// Why resident endpoint selection failed before the session loop started.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ResidentEndpointCatalogError {
    #[error("resident endpoint id is empty or contains whitespace: {id:?}")]
    InvalidId { id: String },
    #[error("resident endpoint {id:?} is already registered")]
    Duplicate { id: String },
    #[error("resident endpoint {id:?} is not registered")]
    Unknown { id: String },
    #[error("resident endpoint {id:?} could not open: {reason}")]
    Open { id: String, reason: String },
}

/// Host-owned registrations for the endpoints it can compose.
#[derive(Default)]
pub struct ResidentEndpointCatalog {
    registrations: BTreeMap<String, Registration>,
}

impl ResidentEndpointCatalog {
    pub fn new() -> Self {
        Self::default()
    }

    /// List local route keys and labels without opening product state.
    pub fn offers(&self) -> Vec<ResidentEndpointOffer> {
        self.registrations
            .iter()
            .map(|(id, registration)| ResidentEndpointOffer {
                id: id.clone(),
                label: registration.label.clone(),
            })
            .collect()
    }

    /// Register an endpoint that already implements the erased catalog
    /// contract. This is the escape hatch for product-specific carrier
    /// behavior such as projection resume.
    pub fn register_erased(
        &mut self,
        id: impl Into<String>,
        label: impl Into<String>,
        factory: impl FnMut(&AdmittedEndpointContext) -> Result<Box<dyn ResidentEndpoint>, String>
        + Send
        + 'static,
    ) -> Result<(), ResidentEndpointCatalogError> {
        self.insert(id.into(), label.into(), None, factory)
    }

    /// Register a lease-owned endpoint; owner tokens must be unique per lease.
    pub fn register_owned(
        &mut self,
        id: impl Into<String>,
        label: impl Into<String>,
        owner: u64,
        factory: impl FnMut(&AdmittedEndpointContext) -> Result<Box<dyn ResidentEndpoint>, String>
        + Send
        + 'static,
    ) -> Result<(), ResidentEndpointCatalogError> {
        self.insert(id.into(), label.into(), Some(owner), factory)
    }

    /// Remove only the exact lease, leaving any replacement intact.
    pub fn remove_owned(&mut self, id: &str, owner: u64) -> bool {
        if self
            .registrations
            .get(id)
            .is_some_and(|r| r.owner == Some(owner))
        {
            self.registrations.remove(id);
            true
        } else {
            false
        }
    }

    /// Register a silent endpoint factory. The factory runs once when this
    /// catalog selects the route for an already-admitted session.
    pub fn register<E, F>(
        &mut self,
        id: impl Into<String>,
        label: impl Into<String>,
        mut factory: F,
    ) -> Result<(), ResidentEndpointCatalogError>
    where
        E: ProjectionCatalog + ProjectionSource + PresentationSource + IntentSink + Send + 'static,
        <E as ProjectionSource>::Error: Display,
        <E as PresentationSource>::Error: Display,
        <E as IntentSink>::Error: Display,
        F: FnMut(&AdmittedEndpointContext) -> Result<E, String> + Send + 'static,
    {
        self.register_erased(id, label, move |context| {
            let endpoint = factory(context)?;
            Ok(Box::new(TypedResidentEndpoint::silent(endpoint)))
        })
    }

    /// Register an endpoint that supplies revision notices *and* answers
    /// resume, which is what a product endpoint over durable source generally
    /// is.
    ///
    /// Without this, such an endpoint has to enter through
    /// [`Self::register_erased`] and restate the whole erased contract, or
    /// lose resume silently: the typed wrappers answer resume with a refusal,
    /// so a visitor recovering after a bell is told the endpoint cannot do the
    /// thing it demonstrably can.
    pub fn register_resumable_notifying<E, F>(
        &mut self,
        id: impl Into<String>,
        label: impl Into<String>,
        mut factory: F,
    ) -> Result<(), ResidentEndpointCatalogError>
    where
        E: ProjectionCatalog
            + ProjectionSource
            + PresentationSource
            + IntentSink
            + ProjectionNoticeSource
            + ResumableProjectionSource
            + Send
            + 'static,
        <E as ProjectionSource>::Error: Display,
        <E as PresentationSource>::Error: Display,
        <E as IntentSink>::Error: Display,
        <E as ProjectionNoticeSource>::Error: Display,
        <E as ResumableProjectionSource>::Error: Display,
        F: FnMut(&AdmittedEndpointContext) -> Result<E, String> + Send + 'static,
    {
        self.register_erased(id, label, move |context| {
            let endpoint = factory(context)?;
            Ok(Box::new(TypedResidentEndpoint::resumable_notifying(
                endpoint,
            )))
        })
    }

    /// Register an endpoint that supplies revision notices. It is otherwise
    /// the same composition contract as [`Self::register`].
    pub fn register_notifying<E, F>(
        &mut self,
        id: impl Into<String>,
        label: impl Into<String>,
        mut factory: F,
    ) -> Result<(), ResidentEndpointCatalogError>
    where
        E: ProjectionCatalog
            + ProjectionSource
            + PresentationSource
            + IntentSink
            + ProjectionNoticeSource
            + Send
            + 'static,
        <E as ProjectionSource>::Error: Display,
        <E as PresentationSource>::Error: Display,
        <E as IntentSink>::Error: Display,
        <E as ProjectionNoticeSource>::Error: Display,
        F: FnMut(&AdmittedEndpointContext) -> Result<E, String> + Send + 'static,
    {
        self.register_erased(id, label, move |context| {
            let endpoint = factory(context)?;
            Ok(Box::new(TypedResidentEndpoint::notifying(endpoint)))
        })
    }

    /// Open exactly one endpoint under an already-admitted context.
    pub fn open(
        &mut self,
        id: &str,
        context: &AdmittedEndpointContext,
    ) -> Result<ResidentEndpointSession, ResidentEndpointCatalogError> {
        let registration = self
            .registrations
            .get_mut(id)
            .ok_or_else(|| ResidentEndpointCatalogError::Unknown { id: id.to_string() })?;
        let endpoint = (registration.factory)(context).map_err(|reason| {
            ResidentEndpointCatalogError::Open {
                id: id.to_string(),
                reason,
            }
        })?;
        Ok(ResidentEndpointSession { endpoint })
    }

    fn insert(
        &mut self,
        id: String,
        label: String,
        owner: Option<u64>,
        factory: impl FnMut(&AdmittedEndpointContext) -> Result<Box<dyn ResidentEndpoint>, String>
        + Send
        + 'static,
    ) -> Result<(), ResidentEndpointCatalogError> {
        if id.is_empty() || id.chars().any(char::is_whitespace) {
            return Err(ResidentEndpointCatalogError::InvalidId { id });
        }
        if self.registrations.contains_key(&id) {
            return Err(ResidentEndpointCatalogError::Duplicate { id });
        }
        self.registrations.insert(
            id,
            Registration {
                owner,
                label,
                factory: Box::new(factory),
            },
        );
        Ok(())
    }
}

/// The erased endpoint selected for one admitted session.
pub struct ResidentEndpointSession {
    endpoint: Box<dyn ResidentEndpoint>,
}

impl ResidentEndpointSession {
    pub fn prepare(&mut self) -> Result<(), String> {
        self.endpoint.prepare()
    }
    pub fn requires_blocking(&self) -> bool {
        self.endpoint.requires_blocking()
    }
    pub fn is_live(&self) -> bool {
        self.endpoint.is_live()
    }
    pub fn cancellation(&self) -> Option<tokio::sync::watch::Receiver<bool>> {
        self.endpoint.cancellation()
    }
    pub fn resume(&mut self, request: ResumeRequest) -> Result<ResumeReply, String> {
        self.endpoint.resume(request)
    }
}

impl ProjectionCatalog for ResidentEndpointSession {
    fn describe(&self) -> EndpointDescriptor {
        self.endpoint.describe()
    }
}

impl ProjectionSource for ResidentEndpointSession {
    type Error = String;

    fn snapshot(&mut self, request: ProjectionRequest) -> Result<ProjectionSnapshot, Self::Error> {
        self.endpoint.snapshot(request)
    }
}

impl PresentationSource for ResidentEndpointSession {
    type Error = String;

    fn resource(&mut self, request: ResourceRequest) -> Result<ResourceResponse, Self::Error> {
        self.endpoint.resource(request)
    }

    fn resource_chunk(
        &mut self,
        request: ResourceChunkRequest,
    ) -> Result<ResourceChunkResponse, Self::Error> {
        self.endpoint.resource_chunk(request)
    }

    fn retained_pages(
        &mut self,
        request: RetainedPagesRequest,
    ) -> Result<Option<RetainedPagesResponse>, Self::Error> {
        self.endpoint.retained_pages(request).map(Some)
    }
}

impl IntentSink for ResidentEndpointSession {
    type Error = String;

    fn invoke(&mut self, intent: IntentInvocation) -> Result<IntentResult, Self::Error> {
        self.endpoint.invoke(intent)
    }
}

impl ProjectionNoticeSource for ResidentEndpointSession {
    type Error = String;

    fn poll_notice(&mut self) -> Result<Option<CarrierNotice>, Self::Error> {
        self.endpoint.poll_notice()
    }
}

enum NoticeMode<E> {
    Silent,
    Poll(fn(&mut E) -> Result<Option<CarrierNotice>, String>),
}

struct TypedResidentEndpoint<E> {
    endpoint: E,
    notices: NoticeMode<E>,
    resume: ResumeMode<E>,
}

/// Whether this endpoint can recover a client that fell behind, or only
/// re-send whole snapshots.
enum ResumeMode<E> {
    Unsupported,
    Delegate(fn(&mut E, ResumeRequest) -> Result<ResumeReply, String>),
}

impl<E> TypedResidentEndpoint<E> {
    fn silent(endpoint: E) -> Self {
        Self {
            endpoint,
            notices: NoticeMode::Silent,
            resume: ResumeMode::Unsupported,
        }
    }
}

impl<E> TypedResidentEndpoint<E>
where
    E: ProjectionNoticeSource + ResumableProjectionSource,
    <E as ProjectionNoticeSource>::Error: Display,
    <E as ResumableProjectionSource>::Error: Display,
{
    fn resumable_notifying(endpoint: E) -> Self {
        Self {
            endpoint,
            notices: NoticeMode::Poll(poll_notice::<E>),
            resume: ResumeMode::Delegate(resume_projection::<E>),
        }
    }
}

fn resume_projection<E>(endpoint: &mut E, request: ResumeRequest) -> Result<ResumeReply, String>
where
    E: ResumableProjectionSource,
    <E as ResumableProjectionSource>::Error: Display,
{
    endpoint.resume(request).map_err(|error| error.to_string())
}

impl<E> TypedResidentEndpoint<E>
where
    E: ProjectionNoticeSource,
    E::Error: Display,
{
    fn notifying(endpoint: E) -> Self {
        Self {
            endpoint,
            notices: NoticeMode::Poll(poll_notice::<E>),
            resume: ResumeMode::Unsupported,
        }
    }
}

fn poll_notice<E>(endpoint: &mut E) -> Result<Option<CarrierNotice>, String>
where
    E: ProjectionNoticeSource,
    E::Error: Display,
{
    endpoint.poll_notice().map_err(|error| error.to_string())
}

impl<E> ResidentEndpoint for TypedResidentEndpoint<E>
where
    E: ProjectionCatalog + ProjectionSource + PresentationSource + IntentSink + Send,
    <E as ProjectionSource>::Error: Display,
    <E as PresentationSource>::Error: Display,
    <E as IntentSink>::Error: Display,
{
    fn describe(&self) -> EndpointDescriptor {
        self.endpoint.describe()
    }

    fn snapshot(&mut self, request: ProjectionRequest) -> Result<ProjectionSnapshot, String> {
        self.endpoint
            .snapshot(request)
            .map_err(|error| error.to_string())
    }

    fn resource(&mut self, request: ResourceRequest) -> Result<ResourceResponse, String> {
        self.endpoint
            .resource(request)
            .map_err(|error| error.to_string())
    }

    fn resource_chunk(
        &mut self,
        request: ResourceChunkRequest,
    ) -> Result<ResourceChunkResponse, String> {
        self.endpoint
            .resource_chunk(request)
            .map_err(|error| error.to_string())
    }

    fn invoke(&mut self, intent: IntentInvocation) -> Result<IntentResult, String> {
        self.endpoint
            .invoke(intent)
            .map_err(|error| error.to_string())
    }

    fn retained_pages(
        &mut self,
        request: RetainedPagesRequest,
    ) -> Result<RetainedPagesResponse, String> {
        self.endpoint
            .retained_pages(request)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "this endpoint retains no page text".to_string())
    }

    fn poll_notice(&mut self) -> Result<Option<CarrierNotice>, String> {
        match &self.notices {
            NoticeMode::Silent => Ok(None),
            NoticeMode::Poll(poll) => poll(&mut self.endpoint),
        }
    }

    fn resume(&mut self, request: ResumeRequest) -> Result<ResumeReply, String> {
        match &self.resume {
            ResumeMode::Unsupported => {
                Err("endpoint does not support projection resume".to_string())
            },
            ResumeMode::Delegate(resume) => resume(&mut self.endpoint, request),
        }
    }
}

#[cfg(test)]
mod tests;
