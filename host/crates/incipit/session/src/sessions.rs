// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! A mere's sessions over incipit-graph (engine split D256, D257, D260, D265).
//!
//! pandect's sessions are generic over the graph; what a host installs on
//! them is three hooks: how it publishes its graph, how it reads a stored
//! legacy translation, and how its codicils are named and redacted. These
//! are incipit's, each the kernel's hook with this crate's graph, and
//! [`IncipitSessions::new`] installs all three as pandect's
//! `MereSessions::new` installs the kernel's.
//!
//! The session methods pandect defines only for mere-kernel's graph are here
//! as extension traits with the same names, so a caller changes its imports,
//! not its calls. They reach the session's store through pandect's
//! `backend()` (engine split D268), so each keeps pandect's signature.

use std::time::SystemTime;

use kernel::graph::{Author, CoverageLayer, Graph};
use kernel::persistence::GraphSnapshot;
use muniment::Backend;
use pandect::graph_session::LegacyTransactionBackend;
use pandect::{
    GraphSession, GraphSessionManifest, LegacyHook, MereSessions, Publication, PublicationHook,
    PublishedGeneration, SESSIONS_PREFIX, SessionBackend, SessionError, SessionId,
};
use uuid::Uuid;

use crate::addressable_graph::{AddressableGraphStore, ResidencyRoots, ResidentGraph};
use crate::graph_codicil::{
    CodicilHook, GRAPH_SNAPSHOT_SCHEMA_ID, LEGACY_GRAPH_SNAPSHOT_SCHEMA_ID,
    PROFILED_GRAPH_SNAPSHOT_SCHEMA_ID, V2_GRAPH_SNAPSHOT_SCHEMA_ID,
    V3_PROFILED_GRAPH_SNAPSHOT_SCHEMA_ID, redact,
};
use crate::live_graph::{LiveApplied, LiveEditError, ObservedCommand, preflight};

/// incipit-graph's publication: the addressable store's, as the kernel's is.
pub static INCIPIT_PUBLICATION: PublicationHook<Graph> = PublicationHook {
    check: |graph| {
        crate::addressable_graph::refuse_residency_limited(graph).map_err(SessionError::Store)
    },
    prepare: incipit_prepare,
    revision: incipit_revision,
    published: incipit_published,
};

/// incipit-graph's legacy hook: a stored snapshot names its assertion schema.
pub static INCIPIT_LEGACY: LegacyHook<Graph> = LegacyHook {
    assertion_schema: |snapshot: &GraphSnapshot| snapshot.assertion_schema,
};

/// incipit-graph's codicil hook: the same five schema ids as the kernel's
/// (D266, "the kernel's and incipit's both name v4") and the kernel's
/// redaction over this crate's snapshot.
pub static INCIPIT_CODICIL: CodicilHook<Graph> = CodicilHook {
    graph_schema: GRAPH_SNAPSHOT_SCHEMA_ID,
    profiled_schema: PROFILED_GRAPH_SNAPSHOT_SCHEMA_ID,
    v3_profiled_schema: V3_PROFILED_GRAPH_SNAPSHOT_SCHEMA_ID,
    v2_schema: V2_GRAPH_SNAPSHOT_SCHEMA_ID,
    v1_schema: LEGACY_GRAPH_SNAPSHOT_SCHEMA_ID,
    redact,
};

fn incipit_prepare(
    graph: &Graph,
    backend: SessionBackend<'_>,
    container: Uuid,
) -> Result<Publication, SessionError> {
    let store = AddressableGraphStore::new(backend, container);
    let publication = store
        .prepare_recorded_graph(graph)
        .map_err(SessionError::Store)?;
    let generation = publication.generation().to_owned();
    let staged = store.stage(publication);
    Ok(Publication {
        generation,
        writes: Box::new(move |reader: &dyn muniment::TransactionReader| staged.decide(reader)),
    })
}

/// Snapshot reconstruction synthesizes visit times; the exact authoritative
/// facet sidecar replaces them before the generation is named.
fn incipit_revision(
    graph: &Graph,
    backend: SessionBackend<'_>,
    container: Uuid,
) -> Result<String, SessionError> {
    let mut normalized = Graph::try_from_recorded_snapshot(&graph.to_snapshot_at(0))
        .map_err(|error| SessionError::Validation(error.to_string()))?;
    normalized
        .install_facets(graph.facets().clone())
        .map_err(|error| SessionError::Validation(error.to_string()))?;
    let publication =
        AddressableGraphStore::new(backend, container).prepare_recorded_graph(&normalized)?;
    Ok(publication.generation().to_owned())
}

fn incipit_published(backend: SessionBackend<'_>, container: Uuid) -> PublishedGeneration<'_> {
    Box::pin(async move {
        let resident = AddressableGraphStore::new(backend, container)
            .open(ResidencyRoots::default())
            .await?;
        Ok(resident.generation().to_owned())
    })
}

/// The refusals before an addressable publication: an unqualified writer
/// and a residency-limited graph (engine split D171).
fn check_resident_publishable<B: Backend>(
    session: &GraphSession<B, Graph>,
) -> Result<(), SessionError> {
    if session.placement().is_none() {
        return Err(SessionError::Validation(
            "addressable publication requires qualified recorded placement".into(),
        ));
    }
    crate::addressable_graph::refuse_residency_limited(session.graph()).map_err(SessionError::Store)
}

/// A mere's sessions over incipit-graph: pandect's `MereSessions` methods
/// that are defined only for mere-kernel's graph, over this crate's.
#[allow(async_fn_in_trait)]
pub trait IncipitSessions<B: Backend + Clone>: Sized {
    /// Sessions with incipit's publication, legacy and codicil hooks, as
    /// pandect's `MereSessions::new` installs the kernel's.
    fn new(backend: B) -> Self;

    /// Open a session, reading a stored legacy translation through
    /// [`INCIPIT_LEGACY`].
    async fn open(&self, id: SessionId) -> Result<GraphSession<B, Graph>, SessionError>;

    /// Begin a new session in memory, from `baseline` or empty.
    fn begin(&self, author: Author, baseline: Option<Graph>) -> GraphSession<B, Graph>;

    /// Begin from a caller-qualified recorded graph.
    fn begin_recorded(&self, author: Author, baseline: Option<Graph>) -> GraphSession<B, Graph>;

    /// Open an archive as a new read-only session after checking it.
    async fn open_codicil_checked(
        &self,
        id: eidetic::ManifestId,
        author: Author,
        validate: impl FnOnce(&Graph) -> Result<(), String>,
    ) -> Result<GraphSessionManifest, SessionError>
    where
        B: eidetic::Store;

    /// [`open_codicil_checked`](Self::open_codicil_checked) from the
    /// resident's own Eidetic store.
    async fn open_codicil_from_checked(
        &self,
        store: &mut dyn eidetic::Store,
        id: eidetic::ManifestId,
        author: Author,
        validate: impl FnOnce(&Graph) -> Result<(), String>,
    ) -> Result<GraphSessionManifest, SessionError>;

    /// The resident graph of session `id`'s root container.
    async fn open_resident_graph(
        &self,
        id: SessionId,
        roots: ResidencyRoots,
    ) -> Result<ResidentGraph<B>, SessionError>;
}

impl<B: Backend + Clone> IncipitSessions<B> for MereSessions<B, Graph> {
    fn new(backend: B) -> Self {
        MereSessions::new_with(backend)
            .with_publication(&INCIPIT_PUBLICATION)
            .with_legacy(&INCIPIT_LEGACY)
            .with_codicil(&INCIPIT_CODICIL)
    }

    async fn open(&self, id: SessionId) -> Result<GraphSession<B, Graph>, SessionError> {
        self.open_with(id).await
    }

    fn begin(&self, author: Author, baseline: Option<Graph>) -> GraphSession<B, Graph> {
        self.begin_with(author, baseline)
    }

    fn begin_recorded(&self, author: Author, baseline: Option<Graph>) -> GraphSession<B, Graph> {
        self.begin_recorded_with(author, baseline)
    }

    async fn open_codicil_checked(
        &self,
        id: eidetic::ManifestId,
        author: Author,
        validate: impl FnOnce(&Graph) -> Result<(), String>,
    ) -> Result<GraphSessionManifest, SessionError>
    where
        B: eidetic::Store,
    {
        self.open_codicil_checked_with(id, author, validate).await
    }

    async fn open_codicil_from_checked(
        &self,
        store: &mut dyn eidetic::Store,
        id: eidetic::ManifestId,
        author: Author,
        validate: impl FnOnce(&Graph) -> Result<(), String>,
    ) -> Result<GraphSessionManifest, SessionError> {
        self.open_codicil_from_checked_with(store, id, author, validate)
            .await
    }

    async fn open_resident_graph(
        &self,
        id: SessionId,
        roots: ResidencyRoots,
    ) -> Result<ResidentGraph<B>, SessionError> {
        let backend = self.backend();
        let key = format!("{SESSIONS_PREFIX}/{}/manifest.json", id.as_uuid());
        let bytes = backend.get(&key).await?.ok_or(SessionError::Missing(id))?;
        let manifest: GraphSessionManifest = serde_json::from_slice(&bytes)
            .map_err(|error| SessionError::Corrupt(format!("{key}: {error}")))?;
        if manifest.session_id != id {
            return Err(SessionError::Corrupt(
                "resident session manifest identity mismatch".into(),
            ));
        }
        AddressableGraphStore::new(backend.clone(), *manifest.root_graph_id.as_uuid())
            .open(roots)
            .await
            .map_err(SessionError::Store)
    }
}

/// A session over incipit-graph: pandect's `GraphSession` methods that are
/// defined only for mere-kernel's graph, over this crate's.
#[allow(async_fn_in_trait)]
pub trait IncipitSession<B: Backend + Clone>: Sized {
    /// Open session `id` in `backend`, with incipit's hooks.
    async fn open(backend: B, id: SessionId) -> Result<Self, SessionError>;

    /// The addressable store of this session's root graph, for a host that
    /// stages its own settings writes beside the policy.
    fn resident_store(&self) -> AddressableGraphStore<B>;

    /// Checkpoint, then publish the whole graph to the addressable store.
    async fn publish_resident_checkpoint(&mut self) -> Result<String, SessionError>
    where
        B: LegacyTransactionBackend;

    /// Preflight observed stable-ID commands against this complete writer,
    /// then apply their live deltas and the resulting addressable
    /// publication in one atomic write. Returns the published generation.
    async fn transact_live(
        &mut self,
        author: Author,
        commands: Vec<ObservedCommand>,
        at: SystemTime,
    ) -> Result<(LiveApplied, String), LiveEditError>
    where
        B: LegacyTransactionBackend;
}

impl<B: Backend + Clone> IncipitSession<B> for GraphSession<B, Graph> {
    async fn open(backend: B, id: SessionId) -> Result<Self, SessionError> {
        <MereSessions<B, Graph> as IncipitSessions<B>>::new(backend)
            .open_with(id)
            .await
    }

    fn resident_store(&self) -> AddressableGraphStore<B> {
        AddressableGraphStore::new(
            self.backend().clone(),
            *self.manifest().root_graph_id.as_uuid(),
        )
    }

    async fn publish_resident_checkpoint(&mut self) -> Result<String, SessionError>
    where
        B: LegacyTransactionBackend,
    {
        if self.placement().is_none() {
            return Err(SessionError::Validation(
                "addressable publication requires qualified recorded placement".into(),
            ));
        }
        // Refuse before checkpointing too: a limited view cannot become the
        // authority for an existing session's baseline or journal.
        if self
            .graph()
            .known_coverage()
            .limits
            .iter()
            .any(|l| l.layer == CoverageLayer::Residency)
        {
            return Err(SessionError::Validation(
                "cannot checkpoint a residency-limited graph".into(),
            ));
        }
        self.checkpoint().await?;
        self.resident_store()
            .write_recorded_graph(self.graph())
            .await
            .map_err(SessionError::Store)
    }

    async fn transact_live(
        &mut self,
        author: Author,
        commands: Vec<ObservedCommand>,
        at: SystemTime,
    ) -> Result<(LiveApplied, String), LiveEditError>
    where
        B: LegacyTransactionBackend,
    {
        self.ensure_writable()?;
        // The complete current records are what external observations were
        // taken against; the resident graph's filtered copy is never hashed.
        check_resident_publishable(self)?;
        let source = self
            .resident_store()
            .prepare_recorded_graph(self.graph())
            .map_err(SessionError::Store)?;
        let program = preflight(self.graph(), &source, &author, commands)?;
        let created = program.created.clone();
        let (applied, generation) = self
            .transact_change(at, true, move |candidate| {
                // pandect's own `try_edit_as` is private; `edit_now` records
                // the same change, and a refusal from the program leaves the
                // detached candidate uncommitted.
                let (refused, applied) =
                    candidate.edit_now(author.clone(), |graph| program.apply(graph, &author))?;
                refused.map(|()| applied)
            })
            .await?;
        Ok((
            LiveApplied { applied, created },
            generation.expect("published write"),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kernel::graph::CapturedDelta;
    use muniment::MemoryBackend;

    fn person() -> Author {
        Author::person("persona-a").via("turnstone")
    }

    fn add(id: u128) -> CapturedDelta {
        CapturedDelta::ReplayAddNodeWithIdIfMissing {
            id: Uuid::from_u128(id).to_string(),
            url: format!("https://{id}.test/"),
            position: [0.0, 0.0],
        }
    }

    /// pandect's `publication_preflights_refuse_without_preparing` (Mere
    /// 36a13570b, `graph_session_transaction_tests.rs`) through incipit's
    /// publication hook, without the parts that reach pandect's private
    /// fields: a revision prepares once, a reconcile without a receipt
    /// prepares nothing, and an unqualified writer is refused before any.
    #[test]
    fn publication_preflights_refuse_without_preparing() {
        let prepares = || crate::addressable_graph::PREPARES.with(std::cell::Cell::get);
        pollster::block_on(async {
            let mere = <MereSessions<MemoryBackend, Graph> as IncipitSessions<_>>::new(
                MemoryBackend::new(),
            );
            let made = mere.mint(person(), Some("Preflight".into())).await.unwrap();
            let mut session = IncipitSessions::open(&mere, made.session_id).await.unwrap();
            session.apply(person(), vec![add(1), add(2)]).await.unwrap();
            let before = prepares();
            let revision = session.operation_revision().unwrap();
            assert_eq!(prepares() - before, 1, "only the revision's own prepare");
            assert_eq!(session.operation_revision().unwrap(), revision);
            let before = prepares();
            assert!(
                session
                    .reconcile_operation([7; 32])
                    .await
                    .unwrap()
                    .is_none()
            );
            assert_eq!(prepares(), before);
            let mut legacy = IncipitSessions::begin(&mere, person(), Some(Graph::new()));
            legacy
                .flush(std::time::SystemTime::UNIX_EPOCH)
                .await
                .unwrap();
            let unqualified = |result: Result<_, SessionError>| matches!(result, Err(SessionError::Validation(m)) if m.contains("qualified recorded"));
            assert!(unqualified(legacy.operation_revision().map(|_| ())));
            assert!(unqualified(
                legacy.reconcile_operation([7; 32]).await.map(|_| ())
            ));
            assert_eq!(prepares(), before);
        });
    }
}
