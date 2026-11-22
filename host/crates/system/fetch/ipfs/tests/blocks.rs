// Copyright 2026 Mark Alan Boykin
// SPDX-License-Identifier: MPL-2.0

use cid::{Cid, multihash::Multihash};
use mere_ipfs_reader::{Gateway, read};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use url::Url;

fn cid(codec: u64, bytes: &[u8]) -> Cid {
    Cid::new_v1(
        codec,
        Multihash::wrap(0x12, &Sha256::digest(bytes)).unwrap(),
    )
}

struct Blocks(BTreeMap<String, Vec<u8>>);

impl Gateway for Blocks {
    async fn get(&mut self, path: String, limit: usize) -> Result<Vec<u8>, String> {
        self.0
            .get(&path)
            .filter(|block| block.len() <= limit)
            .cloned()
            .ok_or(path)
    }
}

// A DAG-PB UnixFS directory linking one raw file named index.html.
fn site(content: &[u8], served: &[u8]) -> (Blocks, Url) {
    let file = cid(0x55, content);
    let mut link = vec![0x0a, 36];
    link.extend(file.to_bytes());
    link.extend(b"\x12\x0aindex.html");
    let mut dir = vec![0x12, link.len() as u8];
    dir.extend(link);
    dir.extend(b"\x0a\x02\x08\x01");
    let root = cid(0x70, &dir);
    let entries = [
        (format!("/ipfs/{root}?format=raw"), dir),
        (format!("/ipfs/{file}?format=raw"), served.to_vec()),
    ];
    (
        Blocks(entries.into()),
        Url::parse(&format!("ipfs://{root}/")).unwrap(),
    )
}

#[test]
fn directory_index_uses_the_immutable_base() {
    let (mut blocks, url) = site(b"edition A", b"edition A");
    let file = pollster::block_on(read(&mut blocks, &url, 9)).unwrap();
    assert_eq!(file.bytes, b"edition A");
    assert_eq!(file.address, url.join("index.html").unwrap());
}

#[test]
fn replacement_bytes_cannot_satisfy_the_previous_edition() {
    let (mut blocks, url) = site(b"edition A", b"edition B");
    let err = pollster::block_on(read(&mut blocks, &url, 9))
        .err()
        .unwrap();
    assert!(err.contains("differs from its CID"), "{err}");
}
