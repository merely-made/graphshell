// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Drive idle endpoint counts from stdin. Each reply marks a settled bind or close
//! boundary for an external process sampler.

use std::collections::BTreeSet;
use std::io::{self, BufRead, Write};
use std::sync::Arc;
use std::time::Instant;

use mere_resident::sync::personal_sync_host::{PersonalSyncHost, PersonalSyncHostConfig};
use pandect::personal_sync::{SyncRoster, SyncSelection};
use personae::{IdentityProvider, InMemoryProvider, UnassociatedKeypair};
use transport::{
    BlobReadAuthorizer, BlobScope, BlobStore, P2pandaHostPolicy, P2pandaOverlayHost,
    P2pandaTransport, PeerID, sync_overlay_topic,
};

type ProbeResult<T> = Result<T, Box<dyn std::error::Error>>;

enum Endpoint {
    Carrier(P2pandaOverlayHost),
    Sync(Box<PersonalSyncHost>),
}

impl Endpoint {
    fn id(&self) -> PeerID {
        match self {
            Self::Carrier(host) => host.local_peer_id(),
            Self::Sync(host) => PeerID::from_bytes(&host.node_id()).expect("resident endpoint id"),
        }
    }

    async fn close(self) -> ProbeResult<()> {
        match self {
            Self::Carrier(host) => host.close().await?,
            Self::Sync(host) => host.close().await?,
        }
        Ok(())
    }
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> ProbeResult<()> {
    let mode = std::env::args().nth(1).unwrap_or_else(|| "sync".into());
    if !matches!(mode.as_str(), "carrier" | "carrier-no-mdns" | "sync") {
        return Err("usage: iroh_endpoint_cost [carrier|carrier-no-mdns|sync]".into());
    }
    let mut policy = P2pandaHostPolicy::default();
    if mode == "carrier-no-mdns" {
        policy.mdns = None;
    }

    let directory = tempfile::tempdir()?;
    let blobs = Arc::new(BlobStore::open(directory.path().join("blobs")).await?);
    let authority = BlobReadAuthorizer::new();
    let identity = InMemoryProvider::from_seed([0x61; 32]);
    let mut endpoints = Vec::<Endpoint>::new();
    report(0, Instant::now())?;

    for command in io::stdin().lock().lines() {
        let count: usize = command?.parse()?;
        let start = Instant::now();
        while endpoints.len() > count {
            endpoints.pop().expect("nonempty endpoints").close().await?;
        }
        while endpoints.len() < count {
            let index = endpoints.len();
            let mut graph = [0; 32];
            graph[..8].copy_from_slice(&(index as u64).to_le_bytes());
            let endpoint = if mode != "sync" {
                let key = UnassociatedKeypair::generate();
                let builder = P2pandaTransport::builder_for(&key).gossip().scoped_blobs(
                    &blobs,
                    BlobScope::new(graph),
                    authority.clone(),
                );
                Endpoint::Carrier(
                    P2pandaOverlayHost::bind(builder, sync_overlay_topic(graph), &policy).await?,
                )
            } else {
                let config = PersonalSyncHostConfig {
                    graph,
                    store_path: directory.path().join(format!("graph-{index}.redb")),
                    roster: SyncRoster::new([identity.master_public_key().to_bytes()]),
                    selection: SyncSelection::default().with_blob_availability(true),
                    peer_tickets: Vec::new(),
                    peer_hints: Vec::new(),
                    paired_nodes: Vec::new(),
                    relay_urls: Vec::new(),
                };
                Endpoint::Sync(Box::new(
                    PersonalSyncHost::open_with_blob_custody(
                        &identity,
                        config,
                        Arc::clone(&blobs),
                        authority.clone(),
                    )
                    .await?,
                ))
            };
            endpoints.push(endpoint);
        }
        let keys: BTreeSet<_> = endpoints
            .iter()
            .map(|endpoint| endpoint.id().to_bytes())
            .collect();
        assert_eq!(keys.len(), count, "endpoint keys must be distinct");
        report(count, start)?;
    }

    for endpoint in endpoints.into_iter().rev() {
        endpoint.close().await?;
    }
    blobs.shutdown().await?;
    Ok(())
}

fn report(count: usize, start: Instant) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    writeln!(stdout, "{count} {}", start.elapsed().as_millis())?;
    stdout.flush()
}
