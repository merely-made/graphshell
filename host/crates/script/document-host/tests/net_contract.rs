// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Mod projects MP4: the `NetFetcher` contract is uncredentialed, and manifest
//! origins span at most one registrable domain (D8).

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use document_host::mod_loader::{
    AdmissionLevel, ModCapability, ModManifest, ModType, WasmRefusal,
};
use document_host::runtime::DocumentScriptRuntime;
use document_host::{
    DocumentScript, Grant, NetFetcher, NetResponse, Quota, TurnOutcome, url_carries_credentials,
};
use genet_scripted_dom::ScriptedDom;
use layout_dom_api::{LayoutDom, LayoutDomMut, LocalName, Namespace, QualName};

fn guest() -> PathBuf {
    let p = std::env::var("DOC_HOST_GUEST_WASM").unwrap_or_else(|_| {
        "guest/target/wasm32-wasip2/release/document_core_guest.wasm".to_string()
    });
    let path = PathBuf::from(&p);
    assert!(path.exists(), "fixture guest missing at {p}");
    path
}

/// `NetFetcher` takes only a URL, so a request can carry credentials only as URL
/// userinfo. This fetcher records every request and every one that carried any;
/// a test ends by asserting `violations` is empty.
#[derive(Default)]
struct UncredentialedFetcher {
    seen: Mutex<Vec<String>>,
    violations: Mutex<Vec<String>>,
}

impl UncredentialedFetcher {
    fn seen(&self) -> Vec<String> {
        self.seen.lock().expect("lock").clone()
    }

    fn assert_clean(&self) {
        let v = self.violations.lock().expect("lock");
        assert!(v.is_empty(), "requests carried credentials: {v:?}");
    }
}

impl NetFetcher for UncredentialedFetcher {
    fn fetch(&self, url: &str) -> Result<NetResponse, String> {
        self.seen.lock().expect("lock").push(url.to_string());
        if url_carries_credentials(url) {
            self.violations.lock().expect("lock").push(url.to_string());
            return Err("credentials".into());
        }
        Ok(NetResponse {
            status: 200,
            content_type: Some("text/plain".into()),
            body: format!("fetched:{url}"),
        })
    }
}

fn qual(local: &str) -> QualName {
    QualName::new(None, Namespace::from(""), LocalName::from(local))
}

fn page() -> ScriptedDom {
    let mut dom = ScriptedDom::new();
    let root = dom.document();
    let body = dom.create_element(qual("body"));
    dom.append_child(root, body);
    let p = dom.create_element(qual("p"));
    dom.append_child(body, p);
    let t = dom.create_text("Hello");
    dom.append_child(p, t);
    dom
}

fn attach(fetcher: &Arc<UncredentialedFetcher>) -> DocumentScript {
    let f: Arc<dyn NetFetcher> = fetcher.clone();
    DocumentScript::attach(
        &guest(),
        page(),
        &Grant::allow_all(),
        Quota::default(),
        Some(f),
        vec!["example.com".to_string()],
    )
    .expect("attach")
}

#[test]
fn the_test_fetcher_catches_credentials() {
    // The positive control: the instrument flags what it exists to flag.
    let f = UncredentialedFetcher::default();
    assert!(url_carries_credentials("https://user:pw@example.com/"));
    assert!(url_carries_credentials("https://token@example.com"));
    assert!(!url_carries_credentials("https://example.com/a@b?c=d@e"));
    assert!(f.fetch("https://user:pw@example.com/").is_err());
    assert_eq!(f.violations.lock().expect("lock").len(), 1);
}

#[test]
fn a_script_request_reaches_the_backend_with_no_credentials() {
    let fetcher = Arc::new(UncredentialedFetcher::default());
    let mut script = attach(&fetcher);
    let out = script
        .deliver_event("fetch", "https://example.com/data")
        .expect("turn");
    assert!(matches!(out, TurnOutcome::Applied(_)), "{out:?}");
    assert_eq!(fetcher.seen(), vec!["https://example.com/data".to_string()]);
    fetcher.assert_clean();
}

#[test]
fn a_url_with_credentials_never_reaches_the_backend() {
    let fetcher = Arc::new(UncredentialedFetcher::default());
    let mut script = attach(&fetcher);
    let out = script
        .deliver_event("fetch", "https://user:pw@example.com/data")
        .expect("turn");
    let TurnOutcome::Refused(why) = out else {
        panic!("wanted a refusal, got {out:?}");
    };
    assert!(why.contains("credentials"), "{why}");
    assert!(fetcher.seen().is_empty(), "the backend was called");
    fetcher.assert_clean();
}

fn manifest_with(origins: &[&str]) -> ModManifest {
    let mut m = ModManifest::new(
        "mod:origins",
        "origins",
        ModType::Wasm,
        vec![],
        vec![],
        vec![ModCapability::Network],
    );
    m.document_script_origins = origins.iter().map(|s| (*s).to_string()).collect();
    m
}

#[test]
fn admission_refuses_broad_wildcards_and_admits_one_registrable_domain() {
    let rt = DocumentScriptRuntime::permissive();
    for bad in ["*", "*.com", "*.co.uk"] {
        let r = rt.admit(
            &manifest_with(&["example.com", bad]),
            "1",
            &guest(),
            AdmissionLevel::Conformant,
            false,
        );
        let Err(WasmRefusal::BadOrigin { refusal, .. }) = r else {
            panic!("{bad} should be refused, got {r:?}");
        };
        assert_eq!(refusal.origin, bad);
    }
    assert!(rt.record("mod:origins").is_none(), "nothing admitted");

    let record = rt
        .admit(
            &manifest_with(&["*.example.com", "api.example.org"]),
            "1",
            &guest(),
            AdmissionLevel::Conformant,
            false,
        )
        .expect("a registrable domain admits");
    assert!(record.provided.contains(&"document-script:*.example.com".to_string()));
}
