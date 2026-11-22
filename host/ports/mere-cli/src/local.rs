// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! A dataset on this machine, and a query over it.

use std::path::Path;

use anyhow::{Context, Result};
use chirograph::QueryRow;
use mere::dataset::{IndexedDataset, View};
use mere::formats::RdfFormat;

pub fn dataset(path: &Path) -> Result<IndexedDataset> {
    let format = path
        .extension()
        .and_then(|extension| extension.to_str())
        .and_then(RdfFormat::from_extension)
        .context("expected an RDF file extension such as .trig, .ttl, .nq or .jsonld")?;
    let bytes = std::fs::read(path).with_context(|| format!("read {}", path.display()))?;
    mere::formats::parse::parse(format, &bytes, None, mere::ContextCache::full())
        .map_err(|error| anyhow::anyhow!("parse {}: {error}", path.display()))
}

pub fn query(dataset: &IndexedDataset, query: &str) -> Result<Vec<QueryRow>> {
    Ok(mere::query::sparql_on(dataset, query, View::Live, Default::default())
        .map_err(anyhow::Error::msg)?
        .rows)
}
