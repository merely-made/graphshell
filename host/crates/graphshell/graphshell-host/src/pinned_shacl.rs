// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! A native host's validation and evidence read at one kept-source revision.

use mere::acquire::page_claims::KeptPageClaims;
use mere::dataset::View;
use mere_shacl::{GraphSelector, ValidationError, ValidationReport};
use muniment::Backend;
use oxrdf::{Graph, NamedNode};
use pandect::{GraphSession, PinnedView, SessionError};

pub use pandect::GraphRevision as ValidationRevision;

/// A report and its block evidence, read together at one pinned revision.
pub struct PinnedValidation {
    pin: PinnedView,
    revision: ValidationRevision,
    kept: KeptPageClaims,
    report: ValidationReport,
}

impl PinnedValidation {
    /// Run caller-supplied shapes against just the named kept capture.
    ///
    /// The capture facet provides its own RDF dataset and block provenance.
    /// Both come from the pinned graph, which this report holds.
    ///
    /// # Errors
    ///
    /// Returns an error if the capture is absent, its claims cannot be read,
    /// or the shapes cannot be evaluated.
    pub fn validate(
        pin: PinnedView,
        source: &NamedNode,
        shapes: &Graph,
    ) -> Result<Self, PinnedValidationError> {
        let revision = pin.revision_identity();
        let kept = pin
            .kept_page_claims(source)?
            .ok_or(PinnedValidationError::MissingSource)?;
        let report = mere_shacl::validate(
            kept.dataset(),
            View::Live,
            GraphSelector::Named(source.clone().into()),
            shapes,
        )?;
        Ok(Self {
            pin,
            revision,
            kept,
            report,
        })
    }

    pub fn pin(&self) -> &PinnedView {
        &self.pin
    }

    pub fn revision(&self) -> &ValidationRevision {
        &self.revision
    }

    pub fn kept(&self) -> &KeptPageClaims {
        &self.kept
    }

    pub fn report(&self) -> &ValidationReport {
        &self.report
    }
}

/// Check a displayed report against the owner's current session immediately
/// before activating its evidence. A stale report can still be displayed.
///
/// # Errors
///
/// Returns an error if the session's live revision cannot be read, or
/// `PinnedValidationError::Stale` if it differs from the displayed revision.
pub fn require_current_revision<B: Backend>(
    displayed: &ValidationRevision,
    session: &GraphSession<B>,
) -> Result<(), PinnedValidationError> {
    let current = session.live_revision()?;
    if &current != displayed {
        return Err(PinnedValidationError::Stale {
            displayed: displayed.clone(),
            current,
        });
    }
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum PinnedValidationError {
    #[error("Could not read the graph revision or its kept claims: {0}")]
    Session(#[from] SessionError),
    #[error("The kept source is absent at this graph revision")]
    MissingSource,
    #[error("Could not validate the kept source: {0}")]
    Validation(#[from] ValidationError),
    #[error("The graph changed after this report was displayed. Validate again to open evidence.")]
    Stale {
        displayed: ValidationRevision,
        current: ValidationRevision,
    },
}

#[cfg(test)]
mod tests;
