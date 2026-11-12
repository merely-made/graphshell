use super::*;
use crate::{ContentSource, SplitAxis, TabStack, TileBranch};

fn pane(id: u64, kind: &str) -> Tile {
    Tile {
        id: TileId(id),
        title: kind.to_string(),
        content: ContentSource::Open {
            kind: format!("turnstone.{kind}"),
            id: String::new(),
        },
        accent: None,
    }
}

fn rect() -> RelativeRect {
    RelativeRect {
        x: 0.25,
        y: 0.25,
        width: 0.5,
        height: 0.5,
    }
}

fn two_up() -> Workspace {
    Workspace::new(TileTree::split(
        SplitAxis::Row,
        vec![
            TileBranch::new(0.5, TileTree::single(pane(1, "graph"))),
            TileBranch::new(0.5, TileTree::single(pane(2, "roster"))),
        ],
    ))
}

fn stations(ws: &Workspace, id: TileId) -> (bool, bool) {
    (ws.tiled().find(id).is_some(), ws.is_floating(id))
}

/// One tile id walks tiled → float → nested split → float → another
/// workspace → back → tab, and is at exactly one station throughout.
#[test]
fn host_title_updates_preserve_tiles_at_both_stations() {
    let mut workspace=two_up();
    workspace.apply(&FloatEvent::Float{tile:TileId(2),rect:rect()}.into());
    let geometry=workspace.floating()[0].rect;
    workspace.tile_mut(TileId(1)).unwrap().title="Map".into();
    workspace.tile_mut(TileId(2)).unwrap().title="Inspect".into();
    assert_eq!(workspace.find(TileId(1)).unwrap().title,"Map");
    assert_eq!(workspace.find(TileId(2)).unwrap().title,"Inspect");
    assert_eq!(workspace.floating()[0].rect,geometry);
    assert!(workspace.tile_mut(TileId(99)).is_none());
}

#[test]
fn a_tile_moves_through_every_station_and_is_at_one_station_throughout() {
    let mut primary = two_up();
    let roster = TileId(2);
    assert_eq!(stations(&primary, roster), (true, false));

    assert!(
        primary
            .apply(
                &FloatEvent::Float {
                    tile: roster,
                    rect: rect()
                }
                .into()
            )
            .changed()
    );
    assert_eq!(stations(&primary, roster), (false, true));
    assert_eq!(
        primary.tiled().tiles().len(),
        1,
        "the split collapsed to the graph"
    );

    let beside = FloatDockTarget::Beside {
        target: TileId(1),
        edge: Edge::Bottom,
    };
    assert!(
        primary
            .apply(
                &FloatEvent::Dock {
                    tile: roster,
                    target: beside
                }
                .into()
            )
            .changed()
    );
    assert_eq!(stations(&primary, roster), (true, false));
    assert!(matches!(
        primary.tiled(),
        TileTree::Split {
            axis: SplitAxis::Column,
            ..
        }
    ));

    assert!(
        primary
            .apply(
                &FloatEvent::Float {
                    tile: roster,
                    rect: rect()
                }
                .into()
            )
            .changed()
    );
    let outside = TileEvent::Dragged {
        tile: roster,
        to: DropTarget::Outside,
    };
    assert_eq!(
        primary.apply(&outside.into()),
        WorkbenchOutcome::Effect(WorkbenchEffect::TearOut { tile: roster }),
        "an outside drop is the host's effect, and the tile stays put"
    );
    assert_eq!(stations(&primary, roster), (false, true));

    let float = primary.take_floating(roster).unwrap();
    assert!(!primary.contains(roster));
    let mut lens = Workspace::default();
    assert!(lens.insert_floating(float.clone()));
    assert!(
        !lens.insert_floating(float),
        "the station rule refuses a second copy"
    );
    assert_eq!(
        lens.find(roster),
        Some(&pane(2, "roster")),
        "identity travelled"
    );
    assert_eq!(lens.floating()[0].rect, rect(), "so did its rect");

    let back = lens.take_floating(roster).unwrap();
    assert!(primary.insert_floating(back));
    let tab = FloatDockTarget::Tab { target: TileId(1) };
    assert!(
        primary
            .apply(
                &FloatEvent::Dock {
                    tile: roster,
                    target: tab
                }
                .into()
            )
            .changed()
    );
    assert_eq!(stations(&primary, roster), (true, false));
    match primary.tiled() {
        TileTree::Stack(TabStack { tabs, active }) => {
            assert_eq!(tabs.iter().map(|t| t.id.0).collect::<Vec<_>>(), vec![1, 2]);
            assert_eq!(*active, 1, "the docked tab is the active one");
        },
        other => panic!("expected a two-tab stack, got {other:?}"),
    }
}

#[test]
fn docking_to_the_root_needs_an_empty_tree() {
    let mut ws = two_up();
    ws.apply(
        &FloatEvent::Float {
            tile: TileId(2),
            rect: rect(),
        }
        .into(),
    );
    let root = FloatDockTarget::TiledRoot;
    assert!(
        !ws.apply(
            &FloatEvent::Dock {
                tile: TileId(2),
                target: root
            }
            .into()
        )
        .changed()
    );
    ws.apply(
        &FloatEvent::Float {
            tile: TileId(1),
            rect: rect(),
        }
        .into(),
    );
    assert!(ws.tiled().is_empty());
    assert!(
        ws.apply(
            &FloatEvent::Dock {
                tile: TileId(2),
                target: root
            }
            .into()
        )
        .changed()
    );
    assert_eq!(ws.tiled().tiles().len(), 1);
}

#[test]
fn raise_reorders_z_and_pinned_floats_outlive_a_hidden_layer() {
    let mut ws = two_up();
    ws.apply(
        &FloatEvent::Float {
            tile: TileId(1),
            rect: rect(),
        }
        .into(),
    );
    ws.apply(
        &FloatEvent::Float {
            tile: TileId(2),
            rect: rect(),
        }
        .into(),
    );
    let order = |ws: &Workspace| {
        ws.visible_floating(true)
            .iter()
            .map(|f| f.tile.id.0)
            .collect::<Vec<_>>()
    };
    assert_eq!(order(&ws), vec![1, 2]);
    assert!(
        ws.apply(&TileEvent::Activated(TileId(1)).into()).changed(),
        "activating a float raises it"
    );
    assert_eq!(order(&ws), vec![2, 1]);
    assert_eq!(
        ws.floating().iter().map(|f| f.z).max(),
        Some(2),
        "z stays normalized"
    );

    ws.apply(
        &FloatEvent::SetPinned {
            tile: TileId(2),
            pinned: true,
        }
        .into(),
    );
    assert_eq!(
        ws.visible_floating(false)
            .iter()
            .map(|f| f.tile.id.0)
            .collect::<Vec<_>>(),
        vec![2]
    );
    assert!(ws.apply(&TileEvent::Closed(TileId(2)).into()).changed());
    assert!(!ws.contains(TileId(2)));
}

#[test]
fn a_float_resolves_inside_its_area_within_its_constraints() {
    let mut ws = two_up();
    ws.apply(
        &FloatEvent::Float {
            tile: TileId(2),
            rect: rect(),
        }
        .into(),
    );
    ws.apply(
        &FloatEvent::SetConstraints {
            tile: TileId(2),
            constraints: FloatSizeConstraints {
                min_width: 300.0,
                min_height: 0.0,
                max_width: None,
                max_height: Some(100.0),
            },
        }
        .into(),
    );
    let float = &ws.floating()[0];
    let (x, y, w, h) = float.resolve((0.0, 0.0, 400.0, 400.0));
    assert_eq!((w, h), (300.0, 100.0), "min width and max height applied");
    assert_eq!(
        (x, y),
        (100.0, 100.0),
        "x clamped so the float stays inside"
    );
    let nan = FloatingTile {
        rect: RelativeRect {
            x: f32::NAN,
            y: 0.0,
            width: f32::INFINITY,
            height: 0.5,
        },
        ..float.clone()
    };
    let (x, _, w, _) = nan.resolve((0.0, 0.0, 400.0, 400.0));
    assert_eq!(
        (x, w),
        (0.0, 300.0),
        "non-finite reads as zero, then the minimum holds"
    );
}

#[cfg(feature = "serde")]
#[test]
fn a_workspace_round_trips_through_serde_with_every_station_intact() {
    let mut ws = two_up();
    ws.apply(
        &FloatEvent::Float {
            tile: TileId(2),
            rect: rect(),
        }
        .into(),
    );
    ws.apply(
        &FloatEvent::SetPinned {
            tile: TileId(2),
            pinned: true,
        }
        .into(),
    );
    let json = serde_json::to_string(&ws).unwrap();
    let restored: Workspace = serde_json::from_str(&json).unwrap();
    assert_eq!(restored, ws);
}
