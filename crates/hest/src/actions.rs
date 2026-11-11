// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The shared commands as a typed vocabulary.
//!
//! [`ActionId`] is the typed form of this crate's [`catalogue`] (Scenograph
//! editor plan, SE49 and SE50): one variant per shared id, its key and label
//! read from the catalogue, so a command is defined once. ux-events
//! carries it where a typed id is wanted; hosts register the same
//! ids as strings. The egui-era catalogue it replaces (68 actions, four
//! categories) was audited in the plan's F12.

use crate::catalogue::{self, ids};
use serde::{Deserialize, Serialize};

/// A shared command.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, Serialize, Deserialize)]
pub enum ActionId {
    NodeNew,
    NodeDelete,
    NodeEdit,
    NodePin,
    NodeUnpin,
    NodePinToggle,
    NodeViewerAuto,
    RelationAdd,
    RelationAddBoth,
    RelationRetract,
    IdentityWebfinger,
    IdentityActivitypub,
    IdentityNip05,
    WorkbenchSplitBeside,
    WorkbenchSplitOut,
    WorkbenchStackOnto,
    ViewFit,
    ViewZoomIn,
    ViewZoomOut,
    PhysicsToggle,
    PhysicsSettings,
    SessionUndo,
    SessionRedo,
    SessionSave,
    PaneSettings,
    PaneTrail,
    PaneWorkbench,
    PaletteOpen,
    ImportBookmarks,
    NavBack,
    NavForward,
    NavReload,
    NavStop,
    FoldSelection,
    FoldCollapseDescendants,
    FoldExpand,
    ArrangementResetToSeed,
    ArrangementSetSeed,
    ArrangementRestoreRecipeSeed,
    ArrangementDismissRetained,
    NodeForegroundPin,
    TileGrow,
    TileShrink,
    ViewTerritory,
}

impl ActionId {
    /// Every variant, in the catalogue's order.
    pub const ALL: [Self; 44] = [
        Self::NodeNew,
        Self::NodeDelete,
        Self::NodeEdit,
        Self::NodePin,
        Self::NodeUnpin,
        Self::NodePinToggle,
        Self::NodeViewerAuto,
        Self::RelationAdd,
        Self::RelationAddBoth,
        Self::RelationRetract,
        Self::IdentityWebfinger,
        Self::IdentityActivitypub,
        Self::IdentityNip05,
        Self::WorkbenchSplitBeside,
        Self::WorkbenchSplitOut,
        Self::WorkbenchStackOnto,
        Self::ViewFit,
        Self::ViewZoomIn,
        Self::ViewZoomOut,
        Self::PhysicsToggle,
        Self::PhysicsSettings,
        Self::SessionUndo,
        Self::SessionRedo,
        Self::SessionSave,
        Self::PaneSettings,
        Self::PaneTrail,
        Self::PaneWorkbench,
        Self::PaletteOpen,
        Self::ImportBookmarks,
        Self::NavBack,
        Self::NavForward,
        Self::NavReload,
        Self::NavStop,
        Self::FoldSelection,
        Self::FoldCollapseDescendants,
        Self::FoldExpand,
        Self::ArrangementResetToSeed,
        Self::ArrangementSetSeed,
        Self::ArrangementRestoreRecipeSeed,
        Self::ArrangementDismissRetained,
        Self::NodeForegroundPin,
        Self::TileGrow,
        Self::TileShrink,
        Self::ViewTerritory,
    ];

    /// The shared id, `namespace:name`.
    pub fn key(self) -> &'static str {
        match self {
            Self::NodeNew => ids::NODE_NEW,
            Self::NodeDelete => ids::NODE_DELETE,
            Self::NodeEdit => ids::NODE_EDIT,
            Self::NodePin => ids::NODE_PIN,
            Self::NodeUnpin => ids::NODE_UNPIN,
            Self::NodePinToggle => ids::NODE_PIN_TOGGLE,
            Self::NodeViewerAuto => ids::NODE_VIEWER_AUTO,
            Self::RelationAdd => ids::RELATION_ADD,
            Self::RelationAddBoth => ids::RELATION_ADD_BOTH,
            Self::RelationRetract => ids::RELATION_RETRACT,
            Self::IdentityWebfinger => ids::IDENTITY_WEBFINGER,
            Self::IdentityActivitypub => ids::IDENTITY_ACTIVITYPUB,
            Self::IdentityNip05 => ids::IDENTITY_NIP05,
            Self::WorkbenchSplitBeside => ids::WORKBENCH_SPLIT_BESIDE,
            Self::WorkbenchSplitOut => ids::WORKBENCH_SPLIT_OUT,
            Self::WorkbenchStackOnto => ids::WORKBENCH_STACK_ONTO,
            Self::ViewFit => ids::VIEW_FIT,
            Self::ViewZoomIn => ids::VIEW_ZOOM_IN,
            Self::ViewZoomOut => ids::VIEW_ZOOM_OUT,
            Self::PhysicsToggle => ids::PHYSICS_TOGGLE,
            Self::PhysicsSettings => ids::PHYSICS_SETTINGS,
            Self::SessionUndo => ids::SESSION_UNDO,
            Self::SessionRedo => ids::SESSION_REDO,
            Self::SessionSave => ids::SESSION_SAVE,
            Self::PaneSettings => ids::PANE_SETTINGS,
            Self::PaneTrail => ids::PANE_TRAIL,
            Self::PaneWorkbench => ids::PANE_WORKBENCH,
            Self::PaletteOpen => ids::PALETTE_OPEN,
            Self::ImportBookmarks => ids::IMPORT_BOOKMARKS,
            Self::NavBack => ids::NAV_BACK,
            Self::NavForward => ids::NAV_FORWARD,
            Self::NavReload => ids::NAV_RELOAD,
            Self::NavStop => ids::NAV_STOP,
            Self::FoldSelection => ids::FOLD_SELECTION,
            Self::FoldCollapseDescendants => ids::FOLD_COLLAPSE_DESCENDANTS,
            Self::FoldExpand => ids::FOLD_EXPAND,
            Self::ArrangementResetToSeed => ids::ARRANGEMENT_RESET_TO_SEED,
            Self::ArrangementSetSeed => ids::ARRANGEMENT_SET_SEED,
            Self::ArrangementRestoreRecipeSeed => ids::ARRANGEMENT_RESTORE_RECIPE_SEED,
            Self::ArrangementDismissRetained => ids::ARRANGEMENT_DISMISS_RETAINED,
            Self::NodeForegroundPin => ids::NODE_FOREGROUND_PIN,
            Self::TileGrow => ids::TILE_GROW,
            Self::TileShrink => ids::TILE_SHRINK,
            Self::ViewTerritory => ids::VIEW_TERRITORY,
        }
    }

    /// The catalogue's label.
    pub fn label(self) -> &'static str {
        catalogue::label(self.key()).expect("every variant is in the catalogue")
    }

    /// What it acts on: the id's namespace.
    pub fn namespace(self) -> &'static str {
        catalogue::namespace(self.key())
    }

    /// The variant for a shared id.
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|action| action.key() == key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalogue::CATALOGUE;

    #[test]
    fn every_shared_command_round_trips_through_the_typed_catalogue() {
        assert_eq!(ActionId::ALL.len(), catalogue::CATALOGUE.len());
        for action in ActionId::ALL {
            assert_eq!(ActionId::from_key(action.key()), Some(action));
            assert_eq!(catalogue::label(action.key()), Some(action.label()));
        }
        for (key, action) in [
            ("fold:selection", ActionId::FoldSelection),
            (
                "fold:collapse_descendants",
                ActionId::FoldCollapseDescendants,
            ),
            ("fold:expand", ActionId::FoldExpand),
        ] {
            assert_eq!(ActionId::from_key(key), Some(action));
            assert_eq!(action.namespace(), "fold");
        }
        assert_eq!(ActionId::from_key("fold:unknown"), None);
    }

    #[test]
    fn the_variants_are_the_catalogue_one_for_one() {
        assert_eq!(ActionId::ALL.len(), CATALOGUE.len());
        for (action, entry) in ActionId::ALL.into_iter().zip(CATALOGUE) {
            assert_eq!(
                action.key(),
                entry.id,
                "{action:?} out of the catalogue's order"
            );
            assert_eq!(action.label(), entry.label);
            assert_eq!(ActionId::from_key(entry.id), Some(action));
        }
        assert_eq!(ActionId::from_key("frame:select"), None, "retired");
    }

    #[test]
    fn a_variant_names_what_it_acts_on() {
        assert_eq!(ActionId::RelationRetract.namespace(), "relation");
        assert_eq!(ActionId::NavStop.namespace(), "nav");
    }

    #[test]
    fn action_id_serde_json_round_trips() {
        for action in ActionId::ALL {
            let json = serde_json::to_string(&action).expect("serializes");
            let back: ActionId = serde_json::from_str(&json).expect("deserializes");
            assert_eq!(back, action);
        }
    }
}
