// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The `viewer-guest` host (mod projects D34): the graph as
//! `mere.graph-snapshot-and-facets/json` bytes in, a `sceno` scene and a `vector-sheet` out,
//! each a version byte then CBOR, at most 1 MiB, as block-adapter's `view-result`.
//!
//! The host decodes both payloads before anyone paints them, so a pane never receives bytes
//! it cannot read. It contributes nothing itself: the caller owns the pane.

use authority_host::GRAPH_SNAPSHOT_AND_FACETS;
use document_host::mod_loader::InstallRecord;
use vector_sheet::VectorSheet;
use wasmtime::component::{Component, HasSelf};
use wasmtime::Engine;

use crate::{GuestState, Limits, MAX_DOCUMENT_BYTES, MAX_PAYLOAD_BYTES};

mod world {
    wasmtime::component::bindgen!({
        path: "../wit",
        world: "viewer-guest",
        exports: { default: async },
        imports: { default: async },
    });
}
use world::exports::mere::script::viewer as wit;
use world::ViewerGuest;

impl world::mere::script::log::Host for GuestState {
    async fn log(&mut self, message: String) {
        GuestState::log(self, message);
    }
}

impl world::mere::script::caps::Host for GuestState {
    async fn granted(&mut self) -> Vec<String> {
        GuestState::granted(self)
    }
}

/// The version byte of a viewer's scene payload (the sheet carries its own).
pub const SCENE_VERSION: u8 = 1;

/// A viewer's decoded result: what a pane paints.
#[derive(Clone, Debug, PartialEq)]
pub struct ViewedScene {
    pub scene: sceno::Scene,
    pub sheet: VectorSheet,
    pub width: f32,
    pub height: f32,
    /// One line for the view's accessible label.
    pub description: String,
}

impl ViewedScene {
    /// The scene's region labels, in order (the labels a fixture expects).
    pub fn region_labels(&self) -> Vec<String> {
        self.scene
            .regions
            .iter()
            .map(|r| r.label.clone().unwrap_or_default())
            .collect()
    }
}

/// Why a view produced no scene. After `Trapped` the instance is spent: drop it (D20).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ViewFailure {
    Refused(String),
    Trapped(String),
    TooLarge(String),
    /// A payload that did not decode, or carried another version.
    Malformed(String),
}

impl ViewFailure {
    /// Whether the instance must be dropped (D20).
    pub fn drops_instance(&self) -> bool {
        matches!(self, Self::Trapped(_))
    }
}

impl std::fmt::Display for ViewFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Refused(why) => write!(f, "refused: {why}"),
            Self::Trapped(why) => write!(f, "trapped: {why}"),
            Self::TooLarge(why) => write!(f, "too large: {why}"),
            Self::Malformed(why) => write!(f, "malformed view: {why}"),
        }
    }
}

impl std::error::Error for ViewFailure {}

/// One live `viewer-guest` instance; every call arms a fresh deadline.
pub struct ViewerHost {
    store: wasmtime::Store<GuestState>,
    bindings: ViewerGuest,
    limits: Limits,
    title: String,
}

impl ViewerHost {
    /// Instantiate from the exact bytes supplied and read the claim, which must name the
    /// graph format this host sends. With a `record`, the bytes must match the component it
    /// pinned (D4).
    ///
    /// # Errors
    ///
    /// The component is over the cap, differs from the record, does not instantiate, or
    /// reads a graph format other than [`GRAPH_SNAPSHOT_AND_FACETS`].
    pub fn attach_bytes(
        engine: &Engine,
        bytes: &[u8],
        record: Option<&InstallRecord>,
        granted: Vec<String>,
        limits: Limits,
    ) -> wasmtime::Result<Self> {
        crate::admit_bytes(bytes, record)?;
        let component = Component::from_binary(engine, bytes)?;
        let mut linker = crate::base_linker(engine)?;
        world::mere::script::log::add_to_linker::<_, HasSelf<GuestState>>(&mut linker, |s| s)?;
        world::mere::script::caps::add_to_linker::<_, HasSelf<GuestState>>(&mut linker, |s| s)?;
        let mut store = crate::new_store(engine, granted, limits);
        let bindings =
            pollster::block_on(ViewerGuest::instantiate_async(&mut store, &component, &linker))?;
        let claim =
            pollster::block_on(bindings.mere_script_viewer().call_describe(&mut store))?;
        if claim.graph_format != GRAPH_SNAPSHOT_AND_FACETS {
            wasmtime::bail!(
                "the viewer reads '{}', not '{GRAPH_SNAPSHOT_AND_FACETS}'",
                claim.graph_format
            );
        }
        Ok(Self {
            store,
            bindings,
            limits,
            title: claim.title,
        })
    }

    /// The pane's title, from the claim.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// The guest's captured `log` output, oldest first.
    pub fn logs(&self) -> &[String] {
        &self.store.data().logs
    }

    /// Ask for a scene of `graph` (`mere.graph-snapshot-and-facets/json` bytes) at `width`.
    ///
    /// # Errors
    ///
    /// See [`ViewFailure`].
    pub fn view(&mut self, graph: &[u8], width: f32) -> Result<ViewedScene, ViewFailure> {
        if graph.len() > MAX_DOCUMENT_BYTES {
            return Err(ViewFailure::TooLarge(format!(
                "the graph is {} bytes; the cap is {MAX_DOCUMENT_BYTES}",
                graph.len()
            )));
        }
        self.store.set_epoch_deadline(self.limits.epoch_deadline_ticks);
        let result = pollster::block_on(self.bindings.mere_script_viewer().call_view(
            &mut self.store,
            graph,
            wit::ViewAsk { width },
        ))
        .map_err(|trap| ViewFailure::Trapped(crate::first_line(&trap)))?;
        decode(result.map_err(ViewFailure::Refused)?)
    }
}

fn decode(view: wit::ViewResult) -> Result<ViewedScene, ViewFailure> {
    for (name, payload) in [("scene", &view.scene), ("sheet", &view.sheet)] {
        if payload.len() > MAX_PAYLOAD_BYTES {
            return Err(ViewFailure::TooLarge(format!(
                "the {name} is {} bytes; the cap is {MAX_PAYLOAD_BYTES}",
                payload.len()
            )));
        }
    }
    let (version, body) = view
        .scene
        .split_first()
        .ok_or_else(|| ViewFailure::Malformed("the scene is empty".into()))?;
    if *version != SCENE_VERSION {
        return Err(ViewFailure::Malformed(format!(
            "the scene's version {version} is not {SCENE_VERSION}"
        )));
    }
    let scene: sceno::Scene = cbor_core::DecodeOptions::new()
        .decode(body)
        .map_err(|e| ViewFailure::Malformed(format!("scene: {e}")))?
        .deserialized()
        .map_err(|e| ViewFailure::Malformed(format!("scene: {e}")))?;
    let sheet = VectorSheet::decode(&view.sheet)
        .map_err(|e| ViewFailure::Malformed(format!("sheet: {e:?}")))?;
    Ok(ViewedScene {
        scene,
        sheet,
        width: view.width,
        height: view.height,
        description: view.description,
    })
}
