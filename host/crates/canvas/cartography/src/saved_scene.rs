// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The saved scene in a facet store: which version a node holds and how a
//! version-4 scene is written (dynamics grammar plan, F142, F151). The stored
//! shapes (`SavedSceneV2`, `SavedSceneV3`, `SavedDynamics`, `SavedRolesV1`) and
//! the facet names are pandect's: they are the grammar its sync lane and
//! codicil carry (engine split D69, D134). Capture, restore and the version
//! 2/3 conversion are `canvas::saved_scene`. Strict own versioning (F98): a
//! version-4 scene is read as version 4 only; the older facets are read,
//! never written.

use pandect::{AcceptAll, FacetId, NodeFacetStore};
use serde_json::Value;
use uuid::Uuid;

pub use pandect::{
    SAVED_SCENE_FACET, SAVED_SCENE_FACET_V2, SAVED_SCENE_FACET_V3, SavedDynamics, SavedRolesV1,
    SavedSceneV2, SavedSceneV3,
};

/// The arrangement a scene that names none opens on.
pub const DEFAULT_ARRANGEMENT: &str = "phyllotaxis.default";

/// The scene saved on `node` in a facet store, as its stored value.
pub fn stored_scene(store: &NodeFacetStore, node: Uuid) -> Option<StoredScene<'_>> {
    let facets = store.facets_of(&node)?;
    StoredScene::find(|id| facets.get(&FacetId::new(id)))
}

/// What a node's scene facets hold: a version-4 value, or an older one that
/// reads as a [`SavedSceneV2`], for the caller to convert.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StoredScene<'a> {
    V4(&'a Value),
    Older(&'a Value),
}

impl<'a> StoredScene<'a> {
    /// The scene in the facets `get` looks up (a graph's node facets, a
    /// facet store's): version 4 first, else version 3, else version 2.
    pub fn find(get: impl Fn(&str) -> Option<&'a Value>) -> Option<Self> {
        if let Some(value) = get(SAVED_SCENE_FACET) {
            return Some(Self::V4(value));
        }
        get(SAVED_SCENE_FACET_V3)
            .or_else(|| get(SAVED_SCENE_FACET_V2))
            .map(Self::Older)
    }
}

/// Save `scene` on `node` under [`SAVED_SCENE_FACET`] (F142: every save
/// writes version 4), clearing the older facets the node may carry so one
/// scene is never held twice.
pub fn write_saved_scene(store: &mut NodeFacetStore, node: Uuid, scene: &SavedSceneV3) {
    for older in [SAVED_SCENE_FACET_V3, SAVED_SCENE_FACET_V2] {
        store.remove(&node, &FacetId::new(older));
    }
    let value = serde_json::to_value(scene).expect("a saved scene always serializes");
    let _ = store.set(node, FacetId::new(SAVED_SCENE_FACET), value, &AcceptAll);
}

#[cfg(test)]
mod tests {
    use super::*;
    use pandect::CartographyGeometry;

    fn scene() -> SavedSceneV3 {
        SavedSceneV3 {
            name: "kept".into(),
            selected: vec![Uuid::from_u128(1)],
            physics_paused: true,
            dynamics: SavedDynamics::from_value(serde_json::json!({"k": 1})),
            camera_offset: (3.0, 4.0),
            camera_zoom: 2.0,
            default_handler: "web".into(),
            cartography: CartographyGeometry::from_positions([(Uuid::from_u128(1), (1.0, 2.0))]),
        }
    }

    #[test]
    fn a_scene_round_trips_through_the_facet_and_replaces_older_ones() {
        let mut store = NodeFacetStore::default();
        let node = Uuid::from_u128(9);
        let _ = store.set(
            node,
            FacetId::new(SAVED_SCENE_FACET_V3),
            serde_json::json!({}),
            &AcceptAll,
        );
        assert!(matches!(
            stored_scene(&store, node),
            Some(StoredScene::Older(_))
        ));
        write_saved_scene(&mut store, node, &scene());
        let Some(StoredScene::V4(value)) = stored_scene(&store, node) else {
            panic!("a version-4 scene is stored");
        };
        assert_eq!(
            serde_json::from_value::<SavedSceneV3>(value.clone()).unwrap(),
            scene()
        );
        assert!(
            store
                .facets_of(&node)
                .unwrap()
                .get(&FacetId::new(SAVED_SCENE_FACET_V3))
                .is_none()
        );
    }
}
