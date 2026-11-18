// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

use eidetic::kept_text::{DEFAULT_KEPT_BYTES, FetchRecordV1, KeptPage};

use crate::{Fetch, NetFetch, Stores};

/// Most HTML one capture reads, before extraction.
const MAX_HTML_BYTES: u64 = 1 << 20;

/// The bounded text of one page at `address`, fetched once because the person
/// asked. Declared HTML only; script and style are not text; the text is cut
/// at the per-page bound on a character boundary.
pub fn capture_kept_page(address: &str) -> Result<(KeptPage, FetchRecordV1), String> {
    let parsed = url::Url::parse(address).map_err(|_| "That is not a web address.".to_string())?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("Only http and https pages can be kept.".into());
    }
    let fetched_at_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or_default();
    let net = NetFetch::new(&Stores::in_memory())
        .map_err(|error| format!("Could not start the page fetch: {error}"))?;
    let body = <NetFetch as Fetch>::read_all(&net, address, Some("text/html"), MAX_HTML_BYTES)
        .map_err(|error| format!("Could not read that page: {error}"))?;
    let html = String::from_utf8_lossy(&body.bytes);
    let (_, text) = fleece::page_text_from_html(body.facts.content_type.as_deref(), &html)
        .ok_or("That address did not return an HTML page.")?;
    let mut text = text.trim().to_owned();
    if text.is_empty() {
        return Err("The page had no text to keep.".into());
    }
    if text.len() > DEFAULT_KEPT_BYTES {
        let mut end = DEFAULT_KEPT_BYTES;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text.truncate(end);
    }
    let record = FetchRecordV1 {
        requested: address.to_owned(),
        final_url: body.facts.final_url.clone(),
        fetched_at_ms,
        // Only a successful answer yields a body; anything else was refused above.
        status: 200,
        content_type: body.facts.content_type.clone(),
        html_bytes: body.bytes.len() as u64,
        text_bytes: text.len() as u64,
        text_version: eidetic::kept_text::text_version(&text),
    };
    let page = KeptPage {
        url: body.facts.final_url,
        text,
        at_ms: 0,
        private: false,
    };
    Ok((page, record))
}
