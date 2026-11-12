use super::*;
use kernel::graph::apply::{GraphDelta, add_node, apply_graph_delta};

fn graph() -> (Graph, [Uuid; 2]) {
    let mut graph = Graph::new();
    let members = [Uuid::from_u128(1), Uuid::from_u128(2)];
    for member in members {
        add_node(
            &mut graph,
            Some(member),
            "https://example.org/same".into(),
            Default::default(),
        );
    }
    (graph, members)
}

#[test]
fn duplicate_addresses_keep_stable_member_and_tile_identity() {
    let (graph, [a, b]) = graph();
    let mut workspace = FormeWorkspace::<NoPreferences>::new(Uuid::from_u128(10));
    workspace.open(a, &graph).unwrap();
    workspace.open(b, &graph).unwrap();
    let tile = workspace.tile_for_member(b).unwrap();
    workspace.event(TileEvent::Closed(tile), &graph).unwrap();
    workspace.open(b, &graph).unwrap();
    assert_eq!(workspace.tile_for_member(b), Some(tile));
    assert_ne!(workspace.tile_for_member(a), Some(tile));
    assert_eq!(workspace.members().len(), 2);
}

#[test]
fn region_projection_is_layout_only_and_uses_world_bounds() {
    let (graph, [a, b]) = graph();
    let mut workspace = FormeWorkspace::<NoPreferences>::new(Uuid::from_u128(10));
    workspace.open(a, &graph).unwrap();
    workspace.open(b, &graph).unwrap();
    workspace.bounds = [10., 20., 210., 120.];
    let region = workspace
        .project_region(workspace.document.graph_id, &graph)
        .unwrap()
        .unwrap();
    assert_eq!(region.cells.len(), 2);
    let mut world: Vec<_> = region
        .cells
        .iter()
        .map(|cell| region.cell_world_bounds(cell))
        .collect();
    world.sort_by(|a, b| a[0].total_cmp(&b[0]));
    assert_eq!(world, vec![[10., 20., 110., 120.], [110., 20., 210., 120.]]);
    assert_eq!(workspace.members().len(), 2);
}

#[test]
fn invalid_divider_edit_leaves_the_canonical_arrangement_unchanged() {
    let (graph, [a, b]) = graph();
    let mut workspace = FormeWorkspace::<NoPreferences>::new(Uuid::from_u128(10));
    workspace.open(a, &graph).unwrap();
    workspace.open(b, &graph).unwrap();
    workspace.locked = false;
    let before = workspace.document.arrangement.clone();
    let result = workspace.event(
        TileEvent::DividerMoved {
            split: workbench::TilePath(vec![]),
            fractions: vec![0.0],
        },
        &graph,
    );
    assert!(result.is_err());
    assert_eq!(workspace.document.arrangement, before);
}

#[test]
fn current_workspace_json_roundtrips_the_flat_host_preferences_payload() {
    #[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
    struct Preferences {
        marker: u32,
    }
    impl WorkspacePreferences for Preferences {}
    let (graph, [a, _]) = graph();
    let mut workspace = FormeWorkspace::<Preferences>::new(Uuid::from_u128(10));
    workspace.open(a, &graph).unwrap();
    workspace.preferences.marker = 42;
    let bytes = serde_json::to_vec(&workspace).unwrap();
    let restored: FormeWorkspace<Preferences> = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(restored.preferences.marker, 42);
    assert_eq!(restored.members(), vec![a]);
}

#[test]
fn edge_drag_creates_nested_projection_and_locked_focus_close_remain_available() {
    let (mut graph, [a, b]) = graph();
    let c = Uuid::from_u128(3);
    add_node(
        &mut graph,
        Some(c),
        "https://example.org/other".into(),
        Default::default(),
    );
    let mut workspace = FormeWorkspace::<NoPreferences>::new(Uuid::from_u128(10));
    for member in [a, b, c] {
        workspace.open(member, &graph).unwrap();
    }
    workspace.locked = false;
    workspace
        .event(
            TileEvent::Dragged {
                tile: workspace.tile_for_member(c).unwrap(),
                to: DropTarget::Edge {
                    tile: workspace.tile_for_member(b).unwrap(),
                    edge: workbench::Edge::Bottom,
                },
            },
            &graph,
        )
        .unwrap();
    let before = workspace.geometry.clone();
    let encoded = serde_json::to_vec(&workspace).unwrap();
    let mut restored: FormeWorkspace = serde_json::from_slice(&encoded).unwrap();
    restored.validate(workspace.document.graph_id).unwrap();
    assert_eq!(restored.geometry, before);
    assert_eq!(restored.region().cells.len(), 3);
    restored.locked = true;
    let tile = restored.tile_for_member(b).unwrap();
    restored.event(TileEvent::Activated(tile), &graph).unwrap();
    restored
        .event(
            TileEvent::Closed(restored.tile_for_member(a).unwrap()),
            &graph,
        )
        .unwrap();
    assert_eq!(restored.members(), vec![b, c]);
}

#[test]
fn shared_resource_addresses_remain_distinct_access_tiles() {
    let (graph, [a, b]) = graph();
    let key_a = graph.get_node_by_id(a).unwrap().0;
    let key_b = graph.get_node_by_id(b).unwrap().0;
    assert_ne!(key_a, key_b);
    assert_eq!(
        graph.shown_resource_id(key_a),
        graph.shown_resource_id(key_b)
    );
    let mut workspace = FormeWorkspace::<NoPreferences>::new(Uuid::from_u128(10));
    workspace.open(a, &graph).unwrap();
    workspace.open(b, &graph).unwrap();
    workspace.open(a, &graph).unwrap();
    assert_eq!(workspace.layout().tile_count(), 2);
    let region = workspace.region();
    assert_eq!(region.cells.len(), 2);
    let mut cells = region.cells.iter().collect::<Vec<_>>();
    cells.sort_by(|a, b| a.bounds[0].total_cmp(&b.bounds[0]));
    assert!(cells[0].bounds[2] <= cells[1].bounds[0]);
    assert_eq!(graph.node_count(), 2);
    assert_eq!(graph.fields().count(), 0);
}

#[test]
fn malformed_and_collapsed_geometry_is_rejected_before_projection() {
    let (graph, [a, b]) = graph();
    let mut workspace = FormeWorkspace::<NoPreferences>::new(Uuid::from_u128(10));
    workspace.open(a, &graph).unwrap();
    workspace.open(b, &graph).unwrap();
    let valid_geometry = workspace.geometry.clone();
    workspace.geometry = Some(TreeGeometry::Stack {
        members: vec![a, a],
        active: 0,
    });
    assert!(workspace.validate(workspace.document.graph_id).is_err());
    workspace.geometry = Some(TreeGeometry::Stack {
        members: vec![a, b],
        active: 99,
    });
    assert!(workspace.validate(workspace.document.graph_id).is_err());
    workspace.geometry = valid_geometry;
    workspace.bounds = [
        1.0e20_f32,
        0.0,
        f32::from_bits(1.0e20_f32.to_bits() + 1),
        100.0,
    ];
    assert!(workspace.validate(workspace.document.graph_id).is_err());
}

#[test]
fn world_coordinate_rounding_cannot_collapse_projected_cells() {
    let (graph, [a, b]) = graph();
    let mut workspace = FormeWorkspace::<NoPreferences>::new(Uuid::from_u128(10));
    workspace.open(a, &graph).unwrap();
    workspace.open(b, &graph).unwrap();
    workspace.locked = false;
    let mut layout = workspace.layout();
    layout.split_beside_axis(a, b, SplitAxis::Row, true);
    workspace.keep_layout(&layout);
    let left = 1.0e20_f32;
    workspace.bounds = [left, 0.0, f32::from_bits(left.to_bits() + 1), 1000.0];
    assert!(workspace.validate(workspace.document.graph_id).is_err());
}

#[test]
fn split_fraction_extremes_and_divider_arity_fail_without_mutation() {
    let (graph, [a, b]) = graph();
    let mut workspace = FormeWorkspace::<NoPreferences>::new(Uuid::from_u128(10));
    workspace.open(a, &graph).unwrap();
    workspace.open(b, &graph).unwrap();
    workspace.locked = false;
    let original = serde_json::to_vec(&workspace).unwrap();
    for fractions in [vec![], vec![0.2], vec![0.2, 0.3, 0.5], vec![0.0, 1.0]] {
        assert!(
            workspace
                .event(
                    TileEvent::DividerMoved {
                        split: workbench::TilePath(vec![]),
                        fractions
                    },
                    &graph
                )
                .is_err()
        );
        assert_eq!(serde_json::to_vec(&workspace).unwrap(), original);
    }
    let Some(TreeGeometry::Split { children, .. }) = &mut workspace.geometry else {
        panic!("split expected")
    };
    children[0].fraction = f32::MIN_POSITIVE;
    children[1].fraction = f32::MAX / 2.;
    assert!(workspace.validate(workspace.document.graph_id).is_err());
}

#[test]
fn locked_arrangements_still_allow_focus_and_close_and_reconcile_missing_members() {
    let (mut graph, [a, b]) = graph();
    let mut workspace = FormeWorkspace::<NoPreferences>::new(Uuid::from_u128(10));
    workspace.open(a, &graph).unwrap();
    workspace.open(b, &graph).unwrap();
    let before = workspace.geometry.clone();
    assert!(
        workspace
            .event(
                TileEvent::DividerMoved {
                    split: workbench::TilePath(vec![]),
                    fractions: vec![0.3, 0.7]
                },
                &graph
            )
            .is_err()
    );
    assert_eq!(workspace.geometry, before);
    workspace
        .event(
            TileEvent::Activated(workspace.tile_for_member(b).unwrap()),
            &graph,
        )
        .unwrap();
    workspace
        .event(
            TileEvent::Closed(workspace.tile_for_member(a).unwrap()),
            &graph,
        )
        .unwrap();
    assert_eq!(workspace.members(), vec![b]);

    let key = graph.get_node_key_by_id(b).unwrap();
    apply_graph_delta(&mut graph, GraphDelta::RemoveNode { key });
    workspace.reconcile(&graph);
    assert!(workspace.members().is_empty());
    assert_eq!(graph.node_count(), 1);
    assert!(workspace.validate(Uuid::from_u128(99)).is_err());
}

#[test]
fn stack_drop_order_keeps_active_member_and_opaque_tile_ids() {
    let (mut graph, ids) = graph();
    let c = Uuid::from_u128(3);
    add_node(
        &mut graph,
        Some(c),
        "https://example.org/third".into(),
        Default::default(),
    );
    let mut workspace = FormeWorkspace::<NoPreferences>::new(Uuid::from_u128(10));
    for id in [ids[0], ids[1], c] {
        workspace.open(id, &graph).unwrap();
    }
    let mut layout = workspace.layout();
    layout.stack_all();
    workspace.keep_layout(&layout);
    workspace.locked = false;
    workspace
        .event(
            TileEvent::Dragged {
                tile: workspace.tile_for_member(c).unwrap(),
                to: DropTarget::Stack {
                    stack: workbench::TilePath(vec![]),
                    index: 0,
                },
            },
            &graph,
        )
        .unwrap();
    assert_eq!(
        workspace.geometry,
        Some(TreeGeometry::Stack {
            members: vec![c, ids[0], ids[1]],
            active: 0
        })
    );
    workspace
        .event(
            TileEvent::Activated(workspace.tile_for_member(ids[1]).unwrap()),
            &graph,
        )
        .unwrap();
    let bytes = serde_json::to_vec(&workspace).unwrap();
    let restored: FormeWorkspace = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(restored.geometry, workspace.geometry);
    assert_eq!(restored.region().cells[0].member, ids[1]);
}
