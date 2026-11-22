// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! One granted SPARQL query over a mere dataset, served to an invited guest
//! (graphshell:mere-cli Q2; D4, D6, D7, D8).
//!
//! The invitation's scope names the query by its digest
//! ([`Invitation::issue_query`](crate::invitation::Invitation::issue_query)).
//! Admission decides who may ask; [`QueryEndpoint`] checks that they asked for
//! exactly what was granted; the session loop re-reads the revocation ledger
//! and the grant's expiry before every request, so a withdrawn or lapsed grant
//! stops answering mid-session.

use std::sync::Arc;
use std::time::Duration;

use chirograph::{
    ContentHash, EndpointDescriptor, IntentInvocation, IntentResult, MAX_QUERY_TEXT_BYTES,
    ProjectionRequest, ProjectionSnapshot, QueryRequest, QueryResponse, QueryRow,
    ResourceRequest, ResourceResponse,
};
use insigne::delegation::SignedDelegationCertificate;
use mere::dataset::{IndexedDataset, View};
use notochord::{
    AdmittedPrincipal, LocalNetworkPolicy, NetworkId, ProfileRef, ServiceAccess, ServiceRule,
    TrustedRoot,
};
use ostiary::route::ResidentEndpointRoute;

use crate::admission::PROJECTION_SERVICE;
use crate::endpoint_catalog::{ResidentEndpoint, ResidentEndpointCatalog};
use crate::invitation::{InvitationTerms, QUERY_ACTION};
use crate::lifecycle::AdmittedEndpointContext;
use crate::projection_host::{ResidentProjectionHost, SelectedRoute};

/// The catalog route a query session is served from.
pub const QUERY_ROUTE: &str = "mere-query";

const ONLY_QUERIES: &str = "this endpoint answers its one granted query";

/// The owner's policy for a query host: the owner is the only trusted root,
/// and the one action offered is [`QUERY_ACTION`] in `terms`' domain.
pub fn query_policy(
    terms: &InvitationTerms,
    network: NetworkId,
    owner_root: [u8; 32],
    accepted_profiles: Vec<ProfileRef>,
    max_sessions: Option<u32>,
) -> LocalNetworkPolicy {
    let mut policy = LocalNetworkPolicy::closed(network);
    policy.trusted_roots = vec![TrustedRoot {
        authority: owner_root,
        issuer: owner_root,
    }];
    policy.accepted_profiles = accepted_profiles;
    policy.services.insert(
        PROJECTION_SERVICE.to_string(),
        ServiceRule::new(
            ServiceAccess::MemberOnly,
            terms.domain,
            [QUERY_ACTION],
            false,
            max_sessions,
        ),
    );
    policy
}

/// A host serving `dataset` to guests `admits` accepts, each held to the one
/// query its grant names. `admits` is the owner's check of the principal and
/// its chain against the invitations it issued
/// ([`Invitation::admits`](crate::invitation::Invitation::admits)).
pub fn query_host(
    policy: LocalNetworkPolicy,
    terms: InvitationTerms,
    dataset: Arc<IndexedDataset>,
    admits: impl Fn(&AdmittedPrincipal, &[SignedDelegationCertificate]) -> Result<(), String>
    + Send
    + Sync
    + 'static,
) -> Result<ResidentProjectionHost, String> {
    let mut catalog = ResidentEndpointCatalog::new();
    catalog
        .register_erased(QUERY_ROUTE, "Mere query", move |context| {
            QueryEndpoint::bind(Arc::clone(&dataset), &terms, context)
                .map(|endpoint| Box::new(endpoint) as Box<dyn ResidentEndpoint>)
        })
        .map_err(|error| error.to_string())?;
    let selector = Arc::new(
        move |principal: &AdmittedPrincipal, chain: &[SignedDelegationCertificate]| {
            admits(principal, chain)?;
            terms
                .granted_query(&principal.action)
                .ok_or("this grant names no query")?;
            Ok(SelectedRoute {
                route: ResidentEndpointRoute::new(QUERY_ROUTE, Duration::from_millis(100))
                    .map_err(|error| error.to_string())?,
                application: None,
            })
        },
    );
    Ok(ResidentProjectionHost::selecting(policy, catalog, selector))
}

/// One admitted session's endpoint: the granted query over a fixed dataset.
pub struct QueryEndpoint {
    granted: ContentHash,
    dataset: Arc<IndexedDataset>,
    /// The granted query's result, computed once: the dataset does not move.
    result: Option<(Vec<String>, Vec<QueryRow>)>,
}

impl QueryEndpoint {
    /// Bind to the query this session's admitted action names.
    pub fn bind(
        dataset: Arc<IndexedDataset>,
        terms: &InvitationTerms,
        context: &AdmittedEndpointContext,
    ) -> Result<Self, String> {
        let (_, granted) = context
            .action()
            .and_then(|action| terms.granted_query(action))
            .ok_or("this session was not granted a query")?;
        Ok(Self {
            granted,
            dataset,
            result: None,
        })
    }
}

impl ResidentEndpoint for QueryEndpoint {
    fn requires_blocking(&self) -> bool {
        true
    }

    fn describe(&self) -> EndpointDescriptor {
        EndpointDescriptor {
            label: "Mere query".into(),
            projections: Vec::new(),
        }
    }

    fn snapshot(&mut self, _: ProjectionRequest) -> Result<ProjectionSnapshot, String> {
        Err(ONLY_QUERIES.into())
    }

    fn resource(&mut self, _: ResourceRequest) -> Result<ResourceResponse, String> {
        Err(ONLY_QUERIES.into())
    }

    fn invoke(&mut self, _: IntentInvocation) -> Result<IntentResult, String> {
        Err(ONLY_QUERIES.into())
    }

    fn query(&mut self, request: QueryRequest) -> Result<QueryResponse, String> {
        if !request.text_within_bound() {
            return Err(format!("query text exceeds {MAX_QUERY_TEXT_BYTES} bytes"));
        }
        if request.digest() != self.granted {
            return Err("query is not enabled by this invitation".into());
        }
        if self.result.is_none() {
            let rows =
                mere::query::sparql_on(&self.dataset, &request.query, View::Live, Default::default())?;
            self.result = Some((rows.variables, rows.rows));
        }
        let (variables, rows) = self.result.as_ref().expect("computed above");
        QueryResponse::page(&request, variables.clone(), rows).map_err(|error| error.to_string())
    }
}

#[cfg(test)]
#[path = "query_host_tests.rs"]
mod tests;
