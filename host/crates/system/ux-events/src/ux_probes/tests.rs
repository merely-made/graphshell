// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Tests for the probe family. Split out of `ux_probes.rs` to keep the
//! parent module under the workspace's 600-LOC ceiling.

use std::sync::Arc;

use hest::actions::ActionId;

use super::*;
use crate::ux_observability::{DismissReason, SurfaceId, UxEvent, UxObservers};

#[test]
fn mutual_exclusion_passes_when_dismissals_precede_opens() {
    let probe = Arc::new(MutualExclusionProbe::new());
    let mut observers = UxObservers::new();
    observers.register(probe_as_observer(Arc::clone(&probe) as Arc<dyn UxProbe>));

    // Open palette, dismiss-superseded, open finder. The host
    // emits the dismissal *before* the new open — invariant holds.
    observers.emit(UxEvent::SurfaceOpened {
        address: None,
        surface: SurfaceId::CommandPalette,
    });
    observers.emit(UxEvent::SurfaceDismissed {
        address: None,
        surface: SurfaceId::CommandPalette,
        reason: DismissReason::Superseded,
    });
    observers.emit(UxEvent::SurfaceOpened {
        address: None,
        surface: SurfaceId::NodeFinder,
    });

    assert!(probe.drain_failures().is_empty());
}

#[test]
fn mutual_exclusion_flags_overlapping_modals() {
    let probe = Arc::new(MutualExclusionProbe::new());
    let mut observers = UxObservers::new();
    observers.register(probe_as_observer(Arc::clone(&probe) as Arc<dyn UxProbe>));

    observers.emit(UxEvent::SurfaceOpened {
        address: None,
        surface: SurfaceId::CommandPalette,
    });
    observers.emit(UxEvent::SurfaceOpened {
        address: None,
        surface: SurfaceId::NodeFinder,
    });

    let failures = probe.drain_failures();
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].probe_name, "mutual_exclusion");
    assert!(failures[0].description.contains("NodeFinder"));
}

#[test]
fn mutual_exclusion_ignores_non_modal_surfaces() {
    let probe = Arc::new(MutualExclusionProbe::new());
    let mut observers = UxObservers::new();
    observers.register(probe_as_observer(Arc::clone(&probe) as Arc<dyn UxProbe>));

    observers.emit(UxEvent::SurfaceOpened {
        address: None,
        surface: SurfaceId::CommandPalette,
    });
    observers.emit(UxEvent::SurfaceOpened {
        address: None,
        surface: SurfaceId::StatusBar,
    });

    assert!(probe.drain_failures().is_empty());
}

#[test]
fn open_dismiss_balance_passes_for_paired_events() {
    let probe = Arc::new(OpenDismissBalanceProbe::new());
    let mut observers = UxObservers::new();
    observers.register(probe_as_observer(Arc::clone(&probe) as Arc<dyn UxProbe>));

    observers.emit(UxEvent::SurfaceOpened {
        address: None,
        surface: SurfaceId::ContextMenu,
    });
    observers.emit(UxEvent::SurfaceDismissed {
        address: None,
        surface: SurfaceId::ContextMenu,
        reason: DismissReason::Cancelled,
    });

    assert!(probe.drain_failures().is_empty());
    assert!(probe.pending_opens().is_empty());
}

#[test]
fn open_dismiss_balance_flags_double_open() {
    let probe = Arc::new(OpenDismissBalanceProbe::new());
    let mut observers = UxObservers::new();
    observers.register(probe_as_observer(Arc::clone(&probe) as Arc<dyn UxProbe>));

    observers.emit(UxEvent::SurfaceOpened {
        address: None,
        surface: SurfaceId::ContextMenu,
    });
    observers.emit(UxEvent::SurfaceOpened {
        address: None,
        surface: SurfaceId::ContextMenu,
    });

    let failures = probe.drain_failures();
    assert_eq!(failures.len(), 1);
    assert!(failures[0].description.contains("opened again"));
}

#[test]
fn open_dismiss_balance_flags_unmatched_dismiss() {
    let probe = Arc::new(OpenDismissBalanceProbe::new());
    let mut observers = UxObservers::new();
    observers.register(probe_as_observer(Arc::clone(&probe) as Arc<dyn UxProbe>));

    observers.emit(UxEvent::SurfaceDismissed {
        address: None,
        surface: SurfaceId::ContextMenu,
        reason: DismissReason::Cancelled,
    });

    let failures = probe.drain_failures();
    assert_eq!(failures.len(), 1);
    assert!(
        failures[0]
            .description
            .contains("dismissed without a matching open")
    );
}

#[test]
fn open_dismiss_balance_pending_reports_unclosed_surfaces() {
    let probe = Arc::new(OpenDismissBalanceProbe::new());
    let mut observers = UxObservers::new();
    observers.register(probe_as_observer(Arc::clone(&probe) as Arc<dyn UxProbe>));

    observers.emit(UxEvent::SurfaceOpened {
        address: None,
        surface: SurfaceId::CommandPalette,
    });
    // Forgot the dismissal — pending_opens reports it.
    let pending = probe.pending_opens();
    assert_eq!(pending.get(&SurfaceId::CommandPalette), Some(&1));
}

// ProductiveSelectionProbe ----------------------------------------------

#[test]
fn productive_selection_palette_with_action_dispatch_passes() {
    let probe = Arc::new(ProductiveSelectionProbe::iced_default());
    let mut observers = UxObservers::new();
    observers.register(probe_as_observer(Arc::clone(&probe) as Arc<dyn UxProbe>));

    observers.emit(UxEvent::SurfaceOpened {
        address: None,
        surface: SurfaceId::CommandPalette,
    });
    observers.emit(UxEvent::SurfaceDismissed {
        address: None,
        surface: SurfaceId::CommandPalette,
        reason: DismissReason::Confirmed,
    });
    observers.emit(UxEvent::ActionDispatched {
        address: None,
        action_id: ActionId::PhysicsToggle,
        target: None,
    });

    assert!(probe.drain_failures().is_empty());
}

#[test]
fn productive_selection_palette_routed_to_node_create_passes() {
    let probe = Arc::new(ProductiveSelectionProbe::iced_default());
    let mut observers = UxObservers::new();
    observers.register(probe_as_observer(Arc::clone(&probe) as Arc<dyn UxProbe>));

    // Palette confirms a host-routed action that opens NodeCreate.
    observers.emit(UxEvent::SurfaceDismissed {
        address: None,
        surface: SurfaceId::CommandPalette,
        reason: DismissReason::Confirmed,
    });
    observers.emit(UxEvent::SurfaceOpened {
        address: None,
        surface: SurfaceId::NodeCreate,
    });

    assert!(probe.drain_failures().is_empty());
}

#[test]
fn productive_selection_finder_must_emit_open_node() {
    let probe = Arc::new(ProductiveSelectionProbe::iced_default());
    let mut observers = UxObservers::new();
    observers.register(probe_as_observer(Arc::clone(&probe) as Arc<dyn UxProbe>));

    observers.emit(UxEvent::SurfaceDismissed {
        address: None,
        surface: SurfaceId::NodeFinder,
        reason: DismissReason::Confirmed,
    });
    // ActionDispatched is NOT a productive outcome for NodeFinder —
    // only OpenNodeDispatched satisfies the rule.
    observers.emit(UxEvent::ActionDispatched {
        address: None,
        action_id: ActionId::PhysicsToggle,
        target: None,
    });

    let failures = probe.drain_failures();
    assert_eq!(failures.len(), 1);
    assert!(failures[0].description.contains("NodeFinder"));
}

#[test]
fn productive_selection_finder_with_open_node_passes() {
    let probe = Arc::new(ProductiveSelectionProbe::iced_default());
    let mut observers = UxObservers::new();
    observers.register(probe_as_observer(Arc::clone(&probe) as Arc<dyn UxProbe>));

    let dummy = kernel::graph::NodeKey::new(0);
    observers.emit(UxEvent::SurfaceDismissed {
        address: None,
        surface: SurfaceId::NodeFinder,
        reason: DismissReason::Confirmed,
    });
    observers.emit(UxEvent::OpenNodeDispatched { node_key: dummy });

    assert!(probe.drain_failures().is_empty());
}

#[test]
fn productive_selection_ignores_cancelled_dismissals() {
    let probe = Arc::new(ProductiveSelectionProbe::iced_default());
    let mut observers = UxObservers::new();
    observers.register(probe_as_observer(Arc::clone(&probe) as Arc<dyn UxProbe>));

    // Cancelled dismissals carry no productive expectation — the user
    // chose not to act and the probe must not flag.
    observers.emit(UxEvent::SurfaceDismissed {
        address: None,
        surface: SurfaceId::CommandPalette,
        reason: DismissReason::Cancelled,
    });

    assert!(probe.drain_failures().is_empty());
}

#[test]
fn productive_selection_context_menu_destructive_path_passes() {
    let probe = Arc::new(ProductiveSelectionProbe::iced_default());
    let mut observers = UxObservers::new();
    observers.register(probe_as_observer(Arc::clone(&probe) as Arc<dyn UxProbe>));

    observers.emit(UxEvent::SurfaceDismissed {
        address: None,
        surface: SurfaceId::ContextMenu,
        reason: DismissReason::Confirmed,
    });
    observers.emit(UxEvent::SurfaceOpened {
        address: None,
        surface: SurfaceId::ConfirmDialog,
    });

    assert!(probe.drain_failures().is_empty());
}

// DestructiveActionGateProbe -------------------------------------------

fn destructive_gate() -> DestructiveActionGateProbe {
    let mut commands = hest::menu::CommandSet::new();
    for id in [
        hest::catalogue::ids::NODE_DELETE,
        hest::catalogue::ids::PHYSICS_TOGGLE,
    ] {
        commands.register(hest::catalogue::command(id).unwrap());
    }
    DestructiveActionGateProbe::new(commands)
}

#[test]
fn destructive_gate_passes_when_confirm_dialog_grants() {
    let probe = Arc::new(destructive_gate());
    let mut observers = UxObservers::new();
    observers.register(probe_as_observer(Arc::clone(&probe) as Arc<dyn UxProbe>));

    // Standard destructive flow: ConfirmDialog Confirmed → destructive
    // action dispatched.
    observers.emit(UxEvent::SurfaceDismissed {
        address: None,
        surface: SurfaceId::ConfirmDialog,
        reason: DismissReason::Confirmed,
    });
    observers.emit(UxEvent::ActionDispatched {
        address: None,
        action_id: ActionId::NodeDelete,
        target: None,
    });

    assert!(probe.drain_failures().is_empty());
}

#[test]
fn destructive_gate_flags_unconfirmed_destructive() {
    let probe = Arc::new(destructive_gate());
    let mut observers = UxObservers::new();
    observers.register(probe_as_observer(Arc::clone(&probe) as Arc<dyn UxProbe>));

    // Destructive action fires without any preceding ConfirmDialog.
    observers.emit(UxEvent::ActionDispatched {
        address: None,
        action_id: ActionId::NodeDelete,
        target: None,
    });

    let failures = probe.drain_failures();
    assert_eq!(failures.len(), 1);
    assert!(failures[0].description.contains("NodeDelete"));
}

#[test]
fn destructive_gate_consumes_grant_after_one_destructive() {
    // A confirmation grants ONE destructive dispatch, not many. A
    // second destructive without re-confirmation must trip the probe.
    let probe = Arc::new(destructive_gate());
    let mut observers = UxObservers::new();
    observers.register(probe_as_observer(Arc::clone(&probe) as Arc<dyn UxProbe>));

    observers.emit(UxEvent::SurfaceDismissed {
        address: None,
        surface: SurfaceId::ConfirmDialog,
        reason: DismissReason::Confirmed,
    });
    observers.emit(UxEvent::ActionDispatched {
        address: None,
        action_id: ActionId::NodeDelete,
        target: None,
    });
    // First passed; second fires without a fresh confirm.
    observers.emit(UxEvent::ActionDispatched {
        address: None,
        action_id: ActionId::NodeDelete,
        target: None,
    });

    let failures = probe.drain_failures();
    assert_eq!(failures.len(), 1);
}

#[test]
fn destructive_gate_cancelled_confirm_does_not_grant() {
    let probe = Arc::new(destructive_gate());
    let mut observers = UxObservers::new();
    observers.register(probe_as_observer(Arc::clone(&probe) as Arc<dyn UxProbe>));

    observers.emit(UxEvent::SurfaceDismissed {
        address: None,
        surface: SurfaceId::ConfirmDialog,
        reason: DismissReason::Cancelled,
    });
    observers.emit(UxEvent::ActionDispatched {
        address: None,
        action_id: ActionId::NodeDelete,
        target: None,
    });

    let failures = probe.drain_failures();
    assert_eq!(failures.len(), 1);
}

#[test]
fn destructive_gate_ignores_non_destructive_actions() {
    let probe = Arc::new(destructive_gate());
    let mut observers = UxObservers::new();
    observers.register(probe_as_observer(Arc::clone(&probe) as Arc<dyn UxProbe>));

    // Non-destructive action without a confirm — fine.
    observers.emit(UxEvent::ActionDispatched {
        address: None,
        action_id: ActionId::PhysicsToggle,
        target: None,
    });

    assert!(probe.drain_failures().is_empty());
}

#[test]
fn destructive_gate_intervening_action_consumes_grant() {
    // ConfirmDialog Confirmed grants the very next destructive
    // dispatch. A non-destructive ActionDispatched between confirm
    // and the destructive consumes the grant defensively, so the
    // destructive that follows trips the probe.
    let probe = Arc::new(destructive_gate());
    let mut observers = UxObservers::new();
    observers.register(probe_as_observer(Arc::clone(&probe) as Arc<dyn UxProbe>));

    observers.emit(UxEvent::SurfaceDismissed {
        address: None,
        surface: SurfaceId::ConfirmDialog,
        reason: DismissReason::Confirmed,
    });
    observers.emit(UxEvent::ActionDispatched {
        address: None,
        action_id: ActionId::PhysicsToggle,
        target: None,
    });
    observers.emit(UxEvent::ActionDispatched {
        address: None,
        action_id: ActionId::NodeDelete,
        target: None,
    });

    let failures = probe.drain_failures();
    assert_eq!(failures.len(), 1);
}

#[test]
fn destructive_gate_reads_the_offered_declaration_instead_of_an_action_list() {
    let mut commands = hest::menu::CommandSet::new();
    let toggle = hest::catalogue::command(hest::catalogue::ids::PHYSICS_TOGGLE)
        .unwrap()
        .with_destructive(true);
    commands.register(toggle);
    let probe = DestructiveActionGateProbe::new(commands);
    probe.observe(&UxEvent::ActionDispatched {
        address: None,
        action_id: ActionId::PhysicsToggle,
        target: None,
    });
    assert_eq!(
        probe.drain_failures().len(),
        1,
        "the offered flag supplies the fact"
    );
}

#[test]
fn destructive_gate_does_not_infer_destructiveness_from_the_shared_id() {
    let mut commands = hest::menu::CommandSet::new();
    commands.register(
        hest::catalogue::command(hest::catalogue::ids::NODE_DELETE)
            .unwrap()
            .with_destructive(false),
    );
    let probe = DestructiveActionGateProbe::new(commands);
    probe.observe(&UxEvent::ActionDispatched {
        address: None,
        action_id: ActionId::NodeDelete,
        target: None,
    });
    assert!(probe.drain_failures().is_empty());
}

#[test]
fn same_kind_grafted_panes_keep_distinct_event_addresses_and_balance() {
    use crate::ux_diagnostics::event_channel;
    use crate::ux_observability::EventAddress;
    use accesskit::NodeId;
    use uxtree::envelope::{Envelope, FederatedAddress};

    let address = |path| {
        let tree = uxtree::tree_id_for_path(path);
        let pane: Envelope<_, _, ()> = Envelope::life(
            FederatedAddress::<()>::grafted((tree, NodeId(9))),
            "private witness",
        );
        let address = EventAddress::from_envelope(&pane, |_| panic!("graft uses its tree"));
        assert_eq!(
            address,
            EventAddress::Grafted {
                tree: tree.0,
                local: 9
            }
        );
        address
    };
    let first = address("p8/first-pane");
    let second = address("p8/second-pane");
    assert_ne!(first, second);
    let surface = SurfaceId::TilePane;
    let probe = Arc::new(OpenDismissBalanceProbe::new());
    let mut observers = UxObservers::new();
    observers.register(probe_as_observer(Arc::clone(&probe) as Arc<dyn UxProbe>));

    let opened = [first, second].map(|address| UxEvent::SurfaceOpened {
        surface,
        address: Some(address),
    });
    let emissions = opened.each_ref().map(event_channel);
    assert_eq!(emissions[0].channel_id, emissions[1].channel_id);
    assert_eq!(
        emissions.map(|event| event.address),
        [Some(first), Some(second)]
    );
    for event in opened {
        let json = serde_json::to_string(&event).unwrap();
        assert!(!json.contains("private witness"));
        assert_eq!(serde_json::from_str::<UxEvent>(&json).unwrap(), event);
        observers.emit(event);
    }
    assert!(
        probe.drain_failures().is_empty(),
        "two distinct panes may be open together"
    );
    assert_eq!(probe.pending_opens().get(&surface), Some(&2));

    // Dismissing the second pane twice cannot balance the first pane's open.
    for address in [second, second] {
        observers.emit(UxEvent::SurfaceDismissed {
            surface,
            reason: DismissReason::Cancelled,
            address: Some(address),
        });
    }
    assert_eq!(probe.drain_failures().len(), 1);
    assert_eq!(probe.pending_opens().get(&surface), Some(&1));
    observers.emit(UxEvent::SurfaceDismissed {
        surface,
        reason: DismissReason::Confirmed,
        address: Some(first),
    });
    assert!(probe.pending_opens().is_empty());
    assert!(probe.drain_failures().is_empty());
}

#[test]
fn producer_address_is_optional_and_distinct_from_the_graph_target() {
    use crate::ux_diagnostics::event_channel;
    use crate::ux_observability::EventAddress;
    use accesskit::NodeId;
    use kernel::graph::NodeKey;
    use uuid::Uuid;
    use uxtree::envelope::{Envelope, FederatedAddress};

    let instance = Uuid::from_u128(11);
    let envelope: Envelope<_, _, u64> =
        Envelope::life(FederatedAddress::producer(instance, NodeId(4)), "pane");
    let address = EventAddress::from_envelope(&envelope, |id| *id);
    assert_eq!(address, EventAddress::Producer { instance, local: 4 });
    let event = UxEvent::ActionDispatched {
        action_id: ActionId::NodePinToggle,
        target: Some(NodeKey::new(22)),
        address: Some(address),
    };
    assert_eq!(event_channel(&event).address, Some(address));

    let old = serde_json::json!({"SurfaceOpened": {"surface": "TilePane"}});
    let decoded: UxEvent = serde_json::from_value(old.clone()).unwrap();
    assert_eq!(decoded.address(), None);
    assert_eq!(serde_json::to_value(decoded).unwrap(), old);
}
