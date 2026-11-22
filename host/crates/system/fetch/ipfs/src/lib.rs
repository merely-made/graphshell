// Copyright 2026 Mark Alan Boykin
// SPDX-License-Identifier: MPL-2.0

//! Read one immutable UnixFS file. The host owns the gateway and transport budgets.
use cid::Cid;
use libp2p_identity::PeerId;
use percent_encoding::percent_decode_str;
use rust_unixfs::{MaybeResolved, file::visit::IdleFileVisit};
use sha2::{Digest, Sha256};
use url::Url;

pub trait Gateway {
    /// Fetch a bounded raw record or block from the configured gateway.
    fn get(
        &mut self,
        path: String,
        limit: usize,
    ) -> impl std::future::Future<Output = Result<Vec<u8>, String>> + Send;
}

pub struct File {
    pub address: Url,
    pub bytes: Vec<u8>,
}

fn message(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// Restrict the adapter to key-based names and hierarchical, credential-free paths.
pub async fn read(gateway: &mut impl Gateway, address: &Url, limit: usize) -> Result<File, String> {
    if !address.username().is_empty() || address.password().is_some() || address.port().is_some() {
        return Err("peer addresses do not accept credentials or ports".into());
    }
    let host = address.host_str().ok_or("missing peer address authority")?;
    let mut immutable = match address.scheme() {
        "ipfs" => address.clone(),
        "ipns" => {
            let peer = match host.parse::<Cid>() {
                Ok(key) if key.codec() == 0x72 => {
                    PeerId::from_multihash(*key.hash()).map_err(|_| "invalid IPNS key")?
                },
                _ => host.parse::<PeerId>().map_err(|_| "invalid IPNS key")?,
            };
            let bytes = gateway
                .get(format!("/ipns/{host}?format=ipns-record"), 10 * 1024)
                .await?;
            if bytes.len() > 10 * 1024 {
                return Err("oversized IPNS record".into());
            }
            let record = rust_ipns::Record::decode(bytes).map_err(message)?;
            record.verify(peer).map_err(message)?;
            let data = record.data().map_err(message)?;
            let target = std::str::from_utf8(data.value()).map_err(message)?;
            let target = target
                .strip_prefix("/ipfs/")
                .ok_or("expected an immutable IPNS target")?;
            let (cid, prefix) = target.split_once('/').unwrap_or((target, ""));
            let root: Cid = cid.parse().map_err(message)?;
            let mut url = Url::parse(&format!("ipfs://{root}/")).map_err(message)?;
            {
                let mut path = url.path_segments_mut().map_err(|_| "invalid IPNS target")?;
                path.pop_if_empty();
                for segment in prefix
                    .trim_end_matches('/')
                    .split('/')
                    .filter(|s| !s.is_empty())
                {
                    if matches!(segment, "." | "..") {
                        return Err("invalid IPNS target path".into());
                    }
                    path.push(segment);
                }
            }
            let prefix = url.as_str().trim_end_matches('/');
            let path = if address.path().is_empty() {
                "/"
            } else {
                address.path()
            };
            Url::parse(&format!("{prefix}{path}")).map_err(message)?
        },
        _ => return Err("expected an IPFS or IPNS address".into()),
    };
    if immutable.path().ends_with('/') || immutable.path().is_empty() {
        immutable
            .path_segments_mut()
            .map_err(|_| "invalid directory URL")?
            .pop_if_empty()
            .push("index.html");
    }
    immutable.set_fragment(None);
    immutable.set_query(address.query());
    let mut cid: Cid = immutable
        .host_str()
        .ok_or("missing root CID")?
        .parse()
        .map_err(message)?;
    let mut allowance = 1024;
    for segment in immutable.path_segments().ok_or("invalid UnixFS path")? {
        let segment = percent_decode_str(segment).decode_utf8().map_err(message)?;
        if segment.is_empty()
            || matches!(segment.as_ref(), "." | "..")
            || segment.contains(['/', '\0'])
        {
            return Err("invalid UnixFS path segment".into());
        }
        if cid.codec() != 0x70 {
            return Err("expected a UnixFS directory".into());
        }
        let block = verified_block(gateway, cid, &mut allowance).await?;
        let mut cache = None;
        let mut found = rust_unixfs::resolve(&block, &segment, &mut cache).map_err(message)?;
        cid = loop {
            match found {
                MaybeResolved::Found(cid) => break cid,
                MaybeResolved::NotFound => return Err(format!("missing UnixFS entry {segment}")),
                MaybeResolved::NeedToLoadMore(lookup) => {
                    let block =
                        verified_block(gateway, *lookup.pending_links().0, &mut allowance).await?;
                    found = lookup.continue_walk(&block, &mut cache).map_err(message)?;
                },
            }
        };
    }
    let block = verified_block(gateway, cid, &mut allowance).await?;
    let (first, size, _, mut rest) = match cid.codec() {
        0x55 => IdleFileVisit::default().start_from_raw(&block),
        0x70 => IdleFileVisit::default().start(&block).map_err(message)?,
        _ => return Err("unsupported UnixFS file codec".into()),
    };
    if size > limit as u64 || first.len() > limit {
        return Err("UnixFS file exceeds byte budget".into());
    }
    let mut bytes = first.to_vec();
    let mut cache = None;
    while let Some(visit) = rest {
        let block = verified_block(gateway, *visit.pending_links().0, &mut allowance).await?;
        let (chunk, next) = visit.continue_walk(&block, &mut cache).map_err(message)?;
        if chunk.len() > limit.saturating_sub(bytes.len()) {
            return Err("UnixFS file exceeds byte budget".into());
        }
        bytes.extend_from_slice(chunk);
        rest = next;
    }
    if bytes.len() as u64 != size {
        return Err("incomplete UnixFS file".into());
    }
    Ok(File {
        address: immutable,
        bytes,
    })
}

async fn verified_block(
    gateway: &mut impl Gateway,
    cid: Cid,
    allowance: &mut usize,
) -> Result<Vec<u8>, String> {
    *allowance = allowance
        .checked_sub(1)
        .ok_or("UnixFS traversal limit exceeded")?;
    if cid.hash().code() != 0x12 {
        return Err("unsupported block multihash".into());
    }
    let block = gateway
        .get(format!("/ipfs/{cid}?format=raw"), 2 * 1024 * 1024)
        .await?;
    if block.len() > 2 * 1024 * 1024 || Sha256::digest(&block).as_slice() != cid.hash().digest() {
        return Err("gateway block differs from its CID".into());
    }
    Ok(block)
}
