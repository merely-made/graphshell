// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The `net.fetch` rules a guest meets, as the native host applies them.
//!
//! `FetchGate` mirrors `document-host`'s `src/host.rs` (the `net::Host::fetch`
//! impl): the per-turn count first, then the origin check, in that order with
//! the same error strings. The origin check is the native `net.rs`, compiled
//! into this crate from the same file. Fetching itself is faked, as in the
//! native tests: `EchoFetcher` returns the URL back. No network is reached.

use crate::net::{NetFetcher, NetResponse, host_in_origins};

/// The faked backend of the native tests (`tests/document_script.rs`).
pub struct EchoFetcher;

impl NetFetcher for EchoFetcher {
    fn fetch(&self, url: &str) -> Result<NetResponse, String> {
        Ok(NetResponse {
            status: 200,
            content_type: Some("text/plain".into()),
            body: format!("fetched:{url}"),
        })
    }
}

/// The count and origin rules for one script.
pub struct FetchGate {
    origins: Vec<String>,
    fetches_this_turn: u32,
    max_fetches_per_turn: u32,
    fetcher: Box<dyn NetFetcher>,
}

impl FetchGate {
    pub fn new(origins: Vec<String>, max_fetches_per_turn: u32) -> Self {
        Self::with_fetcher(origins, max_fetches_per_turn, Box::new(EchoFetcher))
    }

    pub fn with_fetcher(
        origins: Vec<String>,
        max_fetches_per_turn: u32,
        fetcher: Box<dyn NetFetcher>,
    ) -> Self {
        Self { origins, fetches_this_turn: 0, max_fetches_per_turn, fetcher }
    }

    /// A fresh per-turn budget, as `deliver_event` does at the start of a turn.
    pub fn begin_turn(&mut self) {
        self.fetches_this_turn = 0;
    }

    /// `net.fetch` for `url`: refused past the per-turn cap, refused outside
    /// the origins (a refused fetch still counts), otherwise the backend's reply.
    pub fn fetch(&mut self, url: &str) -> Result<NetResponse, String> {
        if self.fetches_this_turn >= self.max_fetches_per_turn {
            return Err(format!(
                "net.fetch: per-turn request limit reached ({})",
                self.max_fetches_per_turn
            ));
        }
        self.fetches_this_turn += 1;
        if !host_in_origins(url, &self.origins) {
            return Err(format!("net.fetch: {url} is outside this script's net origins"));
        }
        self.fetcher.fetch(url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const URL: &str = "https://example.com/x";
    const OUTSIDE: &str = "https://attacker.test/leak";

    fn gate(max: u32) -> FetchGate {
        FetchGate::new(vec!["example.com".to_string()], max)
    }

    #[test]
    fn an_allowed_url_is_fetched_through_the_backend() {
        let response = gate(1).fetch(URL).unwrap();
        assert_eq!(response.status, 200);
        assert_eq!(response.content_type.as_deref(), Some("text/plain"));
        assert_eq!(response.body, "fetched:https://example.com/x");
    }

    #[test]
    fn a_url_outside_the_origins_is_refused_with_the_native_text() {
        let error = gate(1).fetch(OUTSIDE).err().unwrap();
        assert_eq!(
            error,
            "net.fetch: https://attacker.test/leak is outside this script's net origins"
        );
    }

    #[test]
    fn no_origins_reach_nothing() {
        assert!(FetchGate::new(Vec::new(), 1).fetch(URL).is_err());
    }

    #[test]
    fn a_wildcard_origin_matches_its_subdomains() {
        let mut gate = FetchGate::new(vec!["*.example.com".to_string()], 1);
        assert!(gate.fetch("https://a.example.com/x").is_ok());
    }

    #[test]
    fn a_cap_of_one_refuses_the_second_fetch_with_the_native_text() {
        let mut gate = gate(1);
        assert!(gate.fetch(URL).is_ok());
        assert_eq!(
            gate.fetch(URL).err().unwrap(),
            "net.fetch: per-turn request limit reached (1)"
        );
    }

    #[test]
    fn a_cap_of_two_allows_two_and_refuses_the_third() {
        let mut gate = gate(2);
        assert!(gate.fetch(URL).is_ok());
        assert!(gate.fetch(URL).is_ok());
        assert!(gate.fetch(URL).is_err());
    }

    #[test]
    fn a_new_turn_resets_the_count() {
        let mut gate = gate(1);
        assert!(gate.fetch(URL).is_ok());
        assert!(gate.fetch(URL).is_err());
        gate.begin_turn();
        assert!(gate.fetch(URL).is_ok());
    }

    #[test]
    fn the_count_is_checked_before_the_origin() {
        // Spent budget wins over a bad origin, so the message is the count's.
        let mut gate = gate(1);
        let _ = gate.fetch(OUTSIDE);
        let error = gate.fetch(OUTSIDE).err().unwrap();
        assert!(error.contains("per-turn request limit"), "{error}");
    }

    #[test]
    fn a_refused_origin_still_spends_the_budget() {
        let mut gate = gate(1);
        assert!(gate.fetch(OUTSIDE).is_err());
        let error = gate.fetch(URL).err().unwrap();
        assert!(error.contains("per-turn request limit"), "{error}");
    }
}
