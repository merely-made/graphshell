// Copyright 2026 Mark Alan Boykin
// SPDX-License-Identifier: MPL-2.0

use super::{ClientFetchPolicy, Fetched};
use genet_document_resources::{ResourceLimits, ResourceResponse, StagedResourceResolution};
use genet_static_dom::StaticDocument;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use url::Url;

pub fn is_address(address: &str) -> bool {
    matches!(super::scheme_of(address), Some("ipfs" | "ipns"))
}

/// A bounded, script-free page and the same-edition responses discovered by Genet.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PeerAssets {
    pub document_url: String,
    pub resources: BTreeMap<String, Vec<u8>>,
}

struct RawGateway(Url);

impl mere_ipfs_reader::Gateway for RawGateway {
    async fn get(&mut self, path: String, limit: usize) -> Result<Vec<u8>, String> {
        let url = self.0.join(&path).map_err(|e| e.to_string())?;
        let request = netfetcher::Request::get(url);
        let cx = netfetcher::FetchContext::permissive().with_redirect_limit(0);
        let response = netfetcher::fetch(request, &cx).await;
        if response.is_network_error() || response.status != 200 {
            return Err(format!("IPFS gateway response {}", response.status));
        }
        super::read_capped(response.body, limit).await
    }
}

pub(super) async fn load(
    address: &str,
    policy: ClientFetchPolicy,
) -> Result<(Fetched, PeerAssets), String> {
    policy.validate()?;
    let configured = std::env::var("MERE_IPFS_GATEWAY")
        .map_err(|_| "Configure MERE_IPFS_GATEWAY to open IPFS content")?;
    let url = Url::parse(&configured).map_err(|e| e.to_string())?;
    if !matches!(url.scheme(), "https" | "http") || url.cannot_be_a_base() {
        return Err("expected an HTTP(S) IPFS gateway".into());
    }
    let mut gateway = RawGateway(url);
    let input = Url::parse(address).map_err(|e| e.to_string())?;
    let page =
        mere_ipfs_reader::read(&mut gateway, &input, policy.max_page_bytes.min(2 << 20)).await?;
    let body = std::str::from_utf8(&page.bytes)
        .map_err(|_| "expected a UTF-8 static HTML page")?
        .to_owned();
    if !page.address.path().to_ascii_lowercase().ends_with(".html")
        && !page.address.path().to_ascii_lowercase().ends_with(".htm")
    {
        return Err("expected a static HTML entry point".into());
    }
    let mut assets = PeerAssets {
        document_url: page.address.to_string(),
        resources: BTreeMap::new(),
    };
    let mut budget = policy
        .max_page_bytes
        .min(8 << 20)
        .saturating_sub(page.bytes.len());
    loop {
        let pending = pending_resources(&body, &assets)?;
        if pending.is_empty() {
            break;
        }
        for address in pending {
            let url = Url::parse(&address).map_err(|e| e.to_string())?;
            if url.scheme() != "ipfs" || url.host_str() != page.address.host_str() {
                return Err("static edition requires a resource outside its IPFS root".into());
            }
            if assets.resources.len() >= 128 {
                return Err("static edition exceeds resource count".into());
            }
            let file = mere_ipfs_reader::read(&mut gateway, &url, budget).await?;
            budget = budget
                .checked_sub(file.bytes.len())
                .ok_or("edition exceeds byte budget")?;
            assets.resources.insert(address, file.bytes);
        }
    }
    Ok((
        Fetched {
            bytes: page.bytes,
            body,
            content_type: Some("text/html".into()),
            content_disposition: None,
            http_response: None,
        },
        assets,
    ))
}

// A static DOM is thread-local. Finish each discovery pass before awaiting block I/O.
fn pending_resources(body: &str, assets: &PeerAssets) -> Result<Vec<String>, String> {
    let mut staged = StagedResourceResolution::new(
        StaticDocument::parse(body),
        Some(&assets.document_url),
        ResourceLimits::default(),
    );
    loop {
        let pending = staged.pending_requests();
        let mut missing = Vec::new();
        if pending.is_empty() {
            return Ok(missing);
        }
        for request in pending {
            if let Some(bytes) = assets.resources.get(&request.url) {
                let response = ResourceResponse::new(&request.url, bytes.clone());
                staged
                    .provide_response(request, response)
                    .map_err(|_| "asset discovery changed")?;
            } else {
                missing.push(request.url);
            }
        }
        if !missing.is_empty() {
            return Ok(missing);
        }
    }
}
