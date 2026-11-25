// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Validation of a manifest's `document_script_origins` (mod projects MP4, D8): an
//! origin is an exact host or `*.suffix`, and a wildcard may span at most one
//! registrable domain. `*.example.com` is fine; `*`, `*.com` and `*.co.uk` are not.
//!
//! "Registrable" is the Public Suffix List's meaning (the `psl` crate, already in
//! the workspace's graph through netfetcher): a suffix that is itself public, such
//! as `com`, `co.uk` or `github.io`, covers many owners, so no wildcard may sit on it.

/// Why an origin pattern was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginRefusal {
    pub origin: String,
    pub why: &'static str,
}

impl OriginRefusal {
    pub fn words(&self) -> String {
        format!("origin '{}' refused: {}", self.origin, self.why)
    }
}

impl std::fmt::Display for OriginRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.words())
    }
}

impl std::error::Error for OriginRefusal {}

/// Whether `origin` may be granted: an exact host, or `*.suffix` where `suffix` is a
/// registrable domain or below one.
///
/// # Errors
///
/// The refusal, naming why.
pub fn validate_origin(origin: &str) -> Result<(), OriginRefusal> {
    let refuse = |why| {
        Err(OriginRefusal {
            origin: origin.to_string(),
            why,
        })
    };
    let pattern = origin.to_ascii_lowercase();
    if pattern.is_empty() {
        return refuse("empty");
    }
    if pattern.contains(|c: char| c.is_whitespace() || matches!(c, '/' | ':' | '@' | '?' | '#')) {
        return refuse("a host or `*.domain`, with no scheme, port, path or credentials");
    }
    let Some(suffix) = pattern.strip_prefix("*.") else {
        return if pattern.contains('*') {
            refuse("a wildcard is only allowed as the leading `*.` label")
        } else {
            Ok(())
        };
    };
    if suffix.contains('*') {
        return refuse("a wildcard is only allowed as the leading `*.` label");
    }
    if suffix.split('.').any(str::is_empty) {
        return refuse("empty label");
    }
    if !suffix.contains('.') {
        return refuse("a wildcard on a single label spans every domain under it");
    }
    // A public suffix has no registrable domain of its own: `com`, `co.uk`, `github.io`.
    match psl::domain(suffix.as_bytes()) {
        Some(_) => Ok(()),
        None => refuse("a wildcard on a public suffix spans many owners"),
    }
}

#[cfg(test)]
mod tests {
    use super::validate_origin;

    #[test]
    fn a_registrable_domain_admits() {
        for ok in [
            "*.example.com",
            "*.weather.test",
            "*.api.example.co.uk",
            "*.example.co.uk",
            "example.com",
            "localhost",
            "127.0.0.1",
        ] {
            assert!(validate_origin(ok).is_ok(), "{ok}");
        }
    }

    #[test]
    fn a_broad_wildcard_is_refused() {
        for bad in [
            "*", "*.", "*.com", "*.co.uk", "*.uk", "*.github.io", "*.test", "*.*.com",
            "*example.com", "foo.*.com", "*.example.*", "", "*..com", "*.example.com/path",
            "https://example.com", "user@example.com", "example.com:80",
        ] {
            assert!(validate_origin(bad).is_err(), "{bad}");
        }
    }
}
