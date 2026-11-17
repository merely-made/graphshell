// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

use super::*;
use sceno::{Frame, FrameKind, Rect, Transform2, Vec2};

fn bounds() -> Rect {
    Rect::new(Vec2::new(50.0, 80.0), Size2::new(400.0, 300.0))
}

#[test]
fn partition_and_sets_read_the_recipe_rectangle_and_mosaic_reads_its_origin() {
    let compiler = ProjectionCompiler::new(sizes(40.0, 20.0));
    for kind in ["partition", "sets"] {
        let mut definition = definition(kind, "lane");
        definition.frame = Some(Frame::Bounded { bounds: bounds() });
        let compiled = compiler.compile(&definition, &dataset()).unwrap();
        let mut expected = compiled.score.clone();
        expected.frame = None;
        match &mut expected.arrangement {
            ScenoArrangement::Partition(config) => config.bounds = bounds(),
            ScenoArrangement::Setscape(config) => config.bounds = bounds(),
            _ => unreachable!(),
        }
        let mut actual = compiled.scene.clone();
        assert_eq!(
            actual.frame.take().unwrap().spec,
            definition.frame.clone().unwrap()
        );
        assert_eq!(actual, crate::solve(&expected));
        assert_eq!(compiled.scene.bounds, bounds());
    }
    let mut definition = definition("mosaic", "lane");
    let before = compiler.compile(&definition, &dataset()).unwrap();
    let origin = Vec2::new(500.0, -250.0);
    definition.frame = Some(Frame::Unbounded { origin });
    let after = compiler.compile(&definition, &dataset()).unwrap();
    for (old, new) in before.scene.items.iter().zip(&after.scene.items) {
        assert!((new.transform.translate.x - old.transform.translate.x - origin.x).abs() < 0.001);
        assert!((new.transform.translate.y - old.transform.translate.y - origin.y).abs() < 0.001);
        assert_eq!(old.footprint, new.footprint);
        assert_eq!(old.source, new.source);
    }
}

#[test]
fn explicit_legacy_frames_preserve_partition_sets_and_mosaic_scenes_exactly() {
    let compiler = ProjectionCompiler::new(sizes(40.0, 20.0));
    for kind in ["partition", "sets", "mosaic"] {
        let mut definition = definition(kind, "lane");
        let before = compiler.compile(&definition, &dataset()).unwrap();
        definition.frame = Some(match &before.score.arrangement {
            ScenoArrangement::Partition(c) => Frame::Bounded { bounds: c.bounds },
            ScenoArrangement::Setscape(c) => Frame::Bounded { bounds: c.bounds },
            ScenoArrangement::Mosaic(c) => Frame::Unbounded { origin: c.origin },
            _ => unreachable!(),
        });
        let mut explicit = compiler.compile(&definition, &dataset()).unwrap().scene;
        explicit.frame = None;
        assert_eq!(explicit, before.scene, "{kind}");
    }
}

#[test]
fn capabilities_and_legacy_size_conflicts_are_checked_before_solving() {
    let compiler = ProjectionCompiler::new(sizes(40.0, 20.0));
    for (kind, frame) in [
        ("partition", Frame::Unbounded { origin: Vec2::ZERO }),
        ("mosaic", Frame::Bounded { bounds: bounds() }),
        (
            "sets",
            Frame::Flow {
                origin: Vec2::ZERO,
                width: 300.0,
            },
        ),
    ] {
        let mut definition = definition(kind, "lane");
        definition.frame = Some(frame);
        assert!(
            compiler
                .compile(&definition, &dataset())
                .unwrap_err()
                .iter()
                .any(|i| i.field == "frame")
        );
    }
    let mut definition = definition("partition", "lane");
    definition.frame = Some(Frame::Bounded { bounds: bounds() });
    definition
        .arrangement
        .options
        .insert("width".into(), "500".into());
    assert!(
        compiler
            .compile(&definition, &dataset())
            .unwrap_err()
            .iter()
            .any(|i| i.field == "arrangement.options.width")
    );
    assert_eq!(
        Family::Partition.capability().frames,
        vec![FrameKind::Bounded]
    );
    assert_eq!(
        Family::Mosaic.capability().frames,
        vec![FrameKind::Unbounded]
    );
}

#[test]
fn changing_frame_invalidates_placement_but_paint_refresh_keeps_it() {
    let compiler = ProjectionCompiler::new(sizes(40.0, 20.0));
    let mut definition = definition("partition", "lane");
    definition.frame = Some(Frame::Bounded { bounds: bounds() });
    let before = compiler.compile(&definition, &dataset()).unwrap();
    assert!(
        compiler
            .refresh(&before, &definition, &dataset())
            .unwrap()
            .placement_reused
    );
    let Frame::Bounded { bounds } = definition.frame.as_mut().unwrap() else {
        unreachable!()
    };
    bounds.origin.x += 100.0;
    let after = compiler.refresh(&before, &definition, &dataset()).unwrap();
    assert!(!after.placement_reused);
    assert_eq!(after.instance_by_occurrence, before.instance_by_occurrence);
    assert_ne!(
        after.scene.items[0].transform,
        before.scene.items[0].transform
    );
}

fn grid_score() -> sceno::Score {
    let mut score = sceno::Score::new(ScenoArrangement::Grid(sceno::Grid {
        cell: Vec2::new(40.0, 20.0),
        gap: 10.0,
        columns: 8,
        ..Default::default()
    }));
    for ordinal in 0..5 {
        score.items.push(sceno::ScoreItem {
            source: SourceRef::new("fixture", ordinal.to_string()),
            ordinal,
            footprint: sceno::Footprint::Rect {
                size: Size2::new(40.0, 20.0),
            },
            representation: sceno::Representation::Glyph,
            placement: sceno::Placement::Ordinal,
            layer: 0,
            visible: true,
            axis: None,
            embedding: None,
            weight: None,
        });
    }
    score
}

#[test]
fn flow_wraps_ordinal_items_to_page_width_and_grows_height() {
    let mut score = grid_score();
    score.frame = Some(Frame::Flow {
        origin: Vec2::new(100.0, 200.0),
        width: 90.0,
    });
    let scene = crate::solve_via(&score, &crate::SolverRegistry::new()).unwrap();
    assert_eq!(scene.items[0].transform.translate, Vec2::new(120.0, 210.0));
    assert_eq!(scene.items[1].transform.translate, Vec2::new(170.0, 210.0));
    assert_eq!(scene.items[2].transform.translate, Vec2::new(120.0, 240.0));
    assert_eq!(
        scene.bounds,
        Rect::new(Vec2::new(100.0, 200.0), Size2::new(90.0, 80.0))
    );
    let pinned = Vec2::new(-1000.0, -1000.0);
    score.holds.push(sceno::HeldPlacement::pinned(
        score.items[0].source.clone(),
        pinned,
    ));
    let scene = crate::solve_via(&score, &crate::SolverRegistry::new()).unwrap();
    assert_eq!(scene.items[0].transform.translate, pinned);
    assert_eq!(scene.honored_holds.len(), 1);
}

#[test]
fn nested_grid_uses_a_space_and_keeps_local_pin_coordinates() {
    let mut score = grid_score();
    let transform = Transform2 {
        translate: Vec2::new(1000.0, 500.0),
        scale: 2.0,
        rotate: 0.2,
    };
    let local = Rect::new(Vec2::new(10.0, 20.0), Size2::new(90.0, 100.0));
    score.frame = Some(Frame::Nested {
        bounds: local,
        transform,
    });
    let pin = Vec2::new(35.0, 35.0);
    score.holds.push(sceno::HeldPlacement::pinned(
        score.items[0].source.clone(),
        pin,
    ));
    let scene = crate::solve_via(&score, &crate::SolverRegistry::new()).unwrap();
    assert_eq!(scene.spaces.len(), 2);
    assert_eq!(scene.spaces[1].parent, Some(sceno::Scene::WORLD));
    assert_eq!(scene.spaces[1].transform, transform);
    for item in &scene.items {
        assert_eq!(item.space, sceno::SpaceId(1));
    }
    assert_eq!(scene.items[0].transform.translate, pin);
    assert_eq!(
        scene.to_world(scene.items[0].space).unwrap().apply(pin),
        transform.apply(pin)
    );
    assert_eq!(scene.honored_holds.len(), 1);
    let decoded: sceno::Scene =
        serde_json::from_slice(&serde_json::to_vec(&scene).unwrap()).unwrap();
    assert_eq!(decoded, scene);
}

#[test]
fn impossible_flow_width_or_authored_cells_are_refused_instead_of_dropped() {
    let mut score = grid_score();
    score.frame = Some(Frame::Flow {
        origin: Vec2::ZERO,
        width: 30.0,
    });
    assert!(
        crate::solve_via(&score, &crate::SolverRegistry::new())
            .unwrap_err()
            .to_string()
            .contains("page width")
    );
    score.frame = Some(Frame::Flow {
        origin: Vec2::ZERO,
        width: 90.0,
    });
    score.items[0].placement = sceno::Placement::Cell { column: 4, row: 0 };
    assert!(
        crate::solve_via(&score, &crate::SolverRegistry::new())
            .unwrap_err()
            .to_string()
            .contains("do not fit")
    );
}

#[test]
fn empty_explicit_frames_do_not_include_an_unrequested_world_origin() {
    let origin = Vec2::new(1000.0, 2000.0);
    let mut score = grid_score();
    score.items.clear();
    score.frame = Some(Frame::Flow {
        origin,
        width: 300.0,
    });
    assert_eq!(
        crate::solve_via(&score, &crate::SolverRegistry::new())
            .unwrap()
            .bounds,
        Rect::new(origin, Size2::new(300.0, 0.0))
    );
    score.frame = Some(Frame::Nested {
        bounds: bounds(),
        transform: Transform2::translation(1000.0, 2000.0),
    });
    let scene = crate::solve_via(&score, &crate::SolverRegistry::new()).unwrap();
    assert_eq!(scene.bounds.origin, Vec2::new(1050.0, 2080.0));
}

#[test]
fn custom_solvers_declare_and_receive_the_complete_frame() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct Framed {
        declared: bool,
        calls: Arc<AtomicUsize>,
    }
    impl Solver for Framed {
        fn capability(&self) -> SolverCapability {
            let mut capability = SolverCapability::new("framed", "Framed");
            if self.declared {
                capability.frames = vec![FrameKind::Flow, FrameKind::Nested];
            }
            capability
        }
        fn place(
            &self,
            _: &serde_json::Value,
            items: &[&sceno::ScoreItem],
        ) -> Result<Vec<Vec2>, SolveError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(vec![Vec2::ZERO; items.len()])
        }
        fn place_in_frame(
            &self,
            _: &serde_json::Value,
            items: &[&sceno::ScoreItem],
            frame: &Frame,
        ) -> Result<Vec<Vec2>, SolveError> {
            self.calls.fetch_add(10, Ordering::SeqCst);
            let origin = match frame {
                Frame::Flow { origin, .. } => *origin,
                Frame::Nested { bounds, .. } => bounds.origin,
                _ => panic!("capability"),
            };
            Ok(vec![origin; items.len()])
        }
    }
    for declared in [false, true] {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut registry = crate::SolverRegistry::new();
        registry
            .register(Arc::new(Framed {
                declared,
                calls: calls.clone(),
            }))
            .unwrap();
        let compiler = ProjectionCompiler::with_registry(sizes(40.0, 20.0), registry.clone());
        let mut definition = definition("framed", "x");
        definition.frame = Some(Frame::Flow {
            origin: Vec2::new(400.0, 600.0),
            width: 300.0,
        });
        let result = compiler.compile(&definition, &dataset());
        if declared {
            let compiled = result.unwrap();
            assert_eq!(calls.load(Ordering::SeqCst), 10);
            assert_eq!(
                compiled.scene.items[0].transform.translate,
                Vec2::new(400.0, 600.0)
            );
        } else {
            assert!(
                result
                    .unwrap_err()
                    .iter()
                    .any(|issue| issue.field == "frame")
            );
            assert_eq!(calls.load(Ordering::SeqCst), 0, "refused before placement");
        }
        let mut legacy_definition = definition.clone();
        legacy_definition.frame = None;
        let legacy = compiler.compile(&legacy_definition, &dataset());
        assert!(legacy.is_ok());
        assert_eq!(
            calls.load(Ordering::SeqCst) % 10,
            1,
            "omitted frames keep the existing solver entry"
        );
    }
}

#[test]
fn frame_extents_survive_motion_refresh_and_a_moved_nested_space() {
    let mut score = grid_score();
    score.frame = Some(Frame::Flow {
        origin: Vec2::new(100.0, 200.0),
        width: 400.0,
    });
    let mut scene = crate::solve_via(&score, &crate::SolverRegistry::new()).unwrap();
    let width = scene.bounds.size.w;
    for item in &mut scene.items {
        item.transform.translate.x = 150.0;
    }
    crate::relation_forms::refresh_scene(&mut scene);
    assert_eq!(scene.bounds.size.w, width);
    score.frame = Some(Frame::Nested {
        bounds: bounds(),
        transform: Transform2::translation(1000.0, 2000.0),
    });
    let mut scene = crate::solve_via(&score, &crate::SolverRegistry::new()).unwrap();
    scene.spaces[1].transform.translate = Vec2::new(3000.0, 4000.0);
    crate::relation_forms::refresh_scene(&mut scene);
    assert_eq!(
        scene
            .frame
            .as_ref()
            .unwrap()
            .world_bounds(&scene)
            .unwrap()
            .origin,
        Vec2::new(3050.0, 4080.0)
    );
    assert_eq!(scene.bounds.origin, Vec2::new(3050.0, 4080.0));
    let before = scene.clone();
    crate::relation_forms::refresh_scene(&mut scene);
    assert_eq!(scene, before);
}

#[test]
fn declaring_a_frame_without_implementing_its_reader_never_calls_legacy_placement() {
    struct DeclarationOnly;
    impl Solver for DeclarationOnly {
        fn capability(&self) -> SolverCapability {
            let mut capability = SolverCapability::new("declared", "Declared");
            capability.frames.push(FrameKind::Flow);
            capability
        }
        fn place(
            &self,
            _: &serde_json::Value,
            _: &[&sceno::ScoreItem],
        ) -> Result<Vec<Vec2>, SolveError> {
            panic!("explicit frames cannot fall through to legacy placement")
        }
    }
    let mut registry = crate::SolverRegistry::new();
    registry.register(Arc::new(DeclarationOnly)).unwrap();
    let compiler = ProjectionCompiler::with_registry(sizes(40.0, 20.0), registry);
    let mut definition = definition("declared", "x");
    definition.frame = Some(Frame::Flow {
        origin: Vec2::ZERO,
        width: 400.0,
    });
    let errors = compiler.compile(&definition, &dataset()).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|issue| issue.field == "frame" && issue.message.contains("frame reader"))
    );
}
