// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The vectors' keyframe and diff: every table, optional field and scene
//! operation the schema names, in a scene that validates.

use std::collections::BTreeMap;

use chirograph::{
    ActionFormChoiceV1, ActionFormFieldV1, ActionFormV1, ActionIcon, AdvertisedAction,
    BoundsRelationship, CachePolicy, CacheRetention, ContentHash, EngineRevision, IntentEffect,
    IntentReference, PresentationBinding, PresentationCapability, PresentationChange,
    PresentationCodec, PresentationKey, PresentationManifest, PresentationOffer,
    PresentationSemantics, ProjectionDiff, ProjectionSession, ProjectionSnapshot, ProtocolVersion,
    Revision, SceneDiff, SceneEpoch, SceneSnapshot, SemanticRole, SessionStatus,
};
use sceno::fold::{BoundaryBundle, FoldBoundary, FoldDirection, FoldRule, StandIn};
use sceno::{
    Backdrop, Fold, Footprint, Frame, HeldPlacement, HonoredHold, InstanceId, LinkKey, LinkMarks,
    LinkStratum, ProjectedItem, Rect, Region, RelationConstraint, RelationConstraintReport,
    RelationForm, Representation, RoutedRelation, Scene, SceneFrame, Size2, SourceIx, SourceRef,
    Space, SpaceId, Transform2, Vec2,
};
use scenotime::{BackdropId, FoldId, RegionId, RelationId, SceneOp};

pub fn engine(cursor: u64) -> EngineRevision {
    EngineRevision {
        session: "0190f3a2-7c41-7b9e-9d1a-5e2f4c6b8a01".into(),
        cursor,
        digest: format!("v2:{}", blake3::hash(&cursor.to_be_bytes()).to_hex()),
    }
}

fn item(source: u32, x: f32, y: f32, representation: Representation) -> ProjectedItem {
    ProjectedItem {
        source: SourceIx(source),
        space: Scene::WORLD,
        transform: Transform2::translation(x, y),
        footprint: Footprint::Rect {
            size: Size2::new(48.0, 32.0),
        },
        representation,
        layer: 1,
        visible: true,
        hit: Some(Footprint::Circle { radius: 20.5 }),
        channels: vec![("salience".into(), 0.75)],
    }
}

fn scene() -> Scene {
    let mut scene = Scene::new();
    for id in ["node:a", "node:b", "node:c"] {
        scene.intern_source(SourceRef::new("mere.graph", id));
    }
    let lane = scene.push_space(
        Scene::WORLD,
        Transform2::translation(0.0, 120.0),
        Some("lane".into()),
    );
    scene.frame = Some(SceneFrame {
        spec: Frame::Bounded {
            bounds: Rect::new(Vec2::new(-512.0, -384.0), Size2::new(1024.0, 768.0)),
        },
        space: Scene::WORLD,
    });
    scene.backdrops.push(Backdrop {
        source: SourceIx(0),
        space: lane,
        transform: Transform2::IDENTITY,
        footprint: Footprint::Polygon {
            points: vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(64.0, 0.0),
                Vec2::new(32.0, 48.0),
            ],
        },
        kind: "mere:grid-paper".into(),
        visible: true,
        collidable: false,
    });
    scene.items = vec![
        item(0, 12.5, -8.0, Representation::Card),
        item(1, 96.0, 40.0, Representation::Glyph),
        item(
            2,
            160.0,
            40.0,
            Representation::Open {
                kind: "sprite".into(),
            },
        ),
    ];
    scene.items[2].hit = None;
    scene.relations = vec![
        RoutedRelation {
            form: RelationForm::Stroke,
            from: InstanceId(0),
            to: InstanceId(1),
            space: Scene::WORLD,
            points: vec![Vec2::new(12.5, -8.0), Vec2::new(96.0, 40.0)],
            kind: Some("cites".into()),
            weight: Some(0.5),
            link_key: Some(LinkKey::new(LinkStratum::Resource, "node:a", "node:b")),
            marks: LinkMarks {
                forward: true,
                reverse: false,
                opposed: false,
            },
            strength: Some(2.0),
        },
        RoutedRelation {
            form: RelationForm::Order,
            from: InstanceId(1),
            to: InstanceId(2),
            space: Scene::WORLD,
            points: Vec::new(),
            kind: None,
            weight: None,
            link_key: None,
            marks: LinkMarks::default(),
            strength: None,
        },
    ];
    arrange(&mut scene);
    scene
}

/// The placed-relation, region, fold and hold facts beside the tables.
fn arrange(scene: &mut Scene) {
    scene.relation_constraints = vec![RelationConstraintReport {
        constraint: RelationConstraint {
            id: "order:b-c".into(),
            from: InstanceId(1),
            to: InstanceId(2),
            form: RelationForm::Order,
        },
        honored: true,
        reason: "placed in order".into(),
    }];
    scene.arrangement_order = vec![InstanceId(2), InstanceId(0), InstanceId(1)];
    scene.regions = vec![Region {
        members: vec![InstanceId(1), InstanceId(2)],
        space: Scene::WORLD,
        contour: Some(Footprint::Path {
            points: vec![Vec2::new(90.0, 30.0), Vec2::new(170.0, 30.0)],
            width: 4.0,
        }),
        label: Some("pair".into()),
        confidence: 0.875,
    }];
    scene.folds = vec![Fold {
        members: vec![InstanceId(1), InstanceId(2)],
        stand_in: StandIn::Member(InstanceId(1)),
        rule: Some(FoldRule::Descendants {
            root: InstanceId(1),
            family: "cites".into(),
            direction: FoldDirection::Outgoing,
        }),
        boundary: Some(FoldBoundary {
            internal_relations: 1,
            bundles: vec![BoundaryBundle {
                outside: InstanceId(0),
                direction: FoldDirection::Incoming,
                family: Some("cites".into()),
                count: 1,
            }],
        }),
        label: Some("b and what it cites".into()),
    }];
    scene.bounds = Rect::new(Vec2::new(-24.0, -24.0), Size2::new(208.0, 88.0));
    scene.generation = 9;
    scene.unmet_holds = vec![HeldPlacement::pinned(
        SourceRef::new("mere.graph", "node:z"),
        Vec2::new(300.0, 300.0),
    )];
    scene.honored_holds = vec![HonoredHold {
        instance: InstanceId(0),
        placement: HeldPlacement::pinned(
            SourceRef::new("mere.graph", "node:a"),
            Vec2::new(12.5, -8.0),
        ),
    }];
}

fn offer(
    codec: PresentationCodec,
    byte: u8,
    requires: PresentationCapability,
) -> PresentationOffer {
    PresentationOffer {
        codec,
        resource: ContentHash([byte; 32]),
        byte_size: 1536,
        requires,
        semantics: PresentationSemantics {
            label: "Node A".into(),
            role: SemanticRole::Article,
            bounds: BoundsRelationship::FitWithinFootprint,
            actions: vec![AdvertisedAction {
                intent: IntentReference("open".into()),
                label: "Open".into(),
                explanation: "Open the node".into(),
                payload_schema: "mere.open-address/v1".into(),
                input_form: Some(ActionFormV1 {
                    schema: "mere.open-address/v1".into(),
                    fields: vec![ActionFormFieldV1 {
                        name: "pane".into(),
                        label: "Pane".into(),
                        description: String::new(),
                        required: true,
                        choices: vec![ActionFormChoiceV1 {
                            value: "split".into(),
                            label: "Split".into(),
                            description: Some("Beside the graph".into()),
                        }],
                    }],
                }),
                effect: IntentEffect::Curation,
                icon: Some(ActionIcon::Named("open".into())),
            }],
        },
    }
}

fn manifest() -> PresentationManifest {
    let card = PresentationKey("card:a".into());
    let glyph = PresentationKey("glyph:b".into());
    PresentationManifest {
        bindings: vec![
            PresentationBinding {
                instance: InstanceId(0),
                key: card.clone(),
            },
            PresentationBinding {
                instance: InstanceId(1),
                key: glyph.clone(),
            },
        ],
        offers: BTreeMap::from([
            (
                card,
                vec![
                    offer(
                        PresentationCodec::EditableTextV1,
                        1,
                        PresentationCapability::EditableText,
                    ),
                    offer(
                        PresentationCodec::PortableCardV1,
                        2,
                        PresentationCapability::PortableCard,
                    ),
                ],
            ),
            (
                glyph,
                vec![
                    offer(
                        PresentationCodec::ImageV1 {
                            mime_type: "image/png".into(),
                        },
                        3,
                        PresentationCapability::Image,
                    ),
                    offer(
                        PresentationCodec::NativeGlyphV1,
                        4,
                        PresentationCapability::NativeGlyph,
                    ),
                ],
            ),
        ]),
    }
}

pub fn keyframe() -> ProjectionSnapshot {
    ProjectionSnapshot {
        version: ProtocolVersion::V1,
        session: ProjectionSession("projection:7f3a".into()),
        scene: SceneSnapshot::from_dense(SceneEpoch(2), Revision(14), scene())
            .expect("the vector scene validates")
            .with_engine_revision(engine(108)),
        presentation: manifest(),
        cache_policy: CachePolicy {
            retention: CacheRetention::EncryptedPersistent,
            expires_at_ms: Some(1_790_000_000_000),
            purge_on_revocation: true,
        },
    }
}

/// Every scene operation once, in an order that leaves the keyframe valid.
fn operations() -> Vec<SceneOp> {
    let lane = Space {
        parent: Some(SpaceId(0)),
        transform: Transform2::translation(0.0, 120.0),
        name: Some("lane 2".into()),
    };
    let mut paper = scene().backdrops[0].clone();
    paper.visible = false;
    let spare = Backdrop {
        source: SourceIx(3),
        space: SpaceId(2),
        ..paper.clone()
    };
    let mut cited = scene().relations[0].clone();
    cited.weight = Some(0.25);
    let mut pair = scene().regions[0].clone();
    pair.label = None;
    let summary = Fold {
        members: vec![InstanceId(2), InstanceId(3)],
        stand_in: StandIn::Summary {
            label: Some("two".into()),
        },
        rule: Some(FoldRule::Selection),
        boundary: None,
        label: None,
    };
    vec![
        SceneOp::AddSource {
            index: SourceIx(3),
            value: SourceRef::new("mere.graph", "node:d"),
        },
        SceneOp::AddSource {
            index: SourceIx(4),
            value: SourceRef::new("mere.graph", "node:e"),
        },
        SceneOp::UpdateSource {
            index: SourceIx(2),
            value: SourceRef::new("mere.graph", "node:c2"),
        },
        SceneOp::TombstoneSource { index: SourceIx(4) },
        SceneOp::AddSpace {
            index: SpaceId(2),
            value: Space {
                parent: Some(SpaceId(1)),
                transform: Transform2::IDENTITY,
                name: None,
            },
        },
        SceneOp::UpdateSpace {
            index: SpaceId(1),
            value: lane,
        },
        SceneOp::AddBackdrop {
            index: BackdropId(1),
            value: spare,
        },
        SceneOp::UpdateBackdrop {
            index: BackdropId(0),
            value: paper,
        },
        SceneOp::TombstoneBackdrop {
            index: BackdropId(1),
        },
        SceneOp::TombstoneSpace { index: SpaceId(2) },
        SceneOp::AddItem {
            index: InstanceId(3),
            value: item(3, 220.0, 40.0, Representation::Snapshot),
            order: 3,
        },
        SceneOp::AddItem {
            index: InstanceId(4),
            value: item(3, 260.0, 40.0, Representation::LivePane),
            order: 5,
        },
        SceneOp::UpdateItem {
            index: InstanceId(1),
            value: item(1, 104.0, 40.0, Representation::Glyph),
        },
        SceneOp::TombstoneItem {
            index: InstanceId(4),
        },
        SceneOp::SetItemLayer {
            index: InstanceId(2),
            layer: -1,
        },
        SceneOp::SetItemOrder {
            index: InstanceId(0),
            order: 4,
        },
        SceneOp::AddRelation {
            index: RelationId(2),
            value: RoutedRelation {
                form: RelationForm::Containment,
                from: InstanceId(0),
                to: InstanceId(3),
                ..scene().relations[1].clone()
            },
        },
        SceneOp::UpdateRelation {
            index: RelationId(0),
            value: cited,
        },
        SceneOp::TombstoneRelation {
            index: RelationId(1),
        },
        SceneOp::AddRegion {
            index: RegionId(1),
            value: Region {
                members: vec![InstanceId(0), InstanceId(3)],
                ..scene().regions[0].clone()
            },
        },
        SceneOp::UpdateRegion {
            index: RegionId(0),
            value: pair,
        },
        SceneOp::TombstoneRegion { index: RegionId(1) },
        SceneOp::TombstoneFold { index: FoldId(0) },
        SceneOp::AddFold {
            index: FoldId(1),
            value: summary.clone(),
        },
        SceneOp::UpdateFold {
            index: FoldId(1),
            value: Fold {
                label: Some("c and d".into()),
                ..summary
            },
        },
        SceneOp::SetFoldStandIn {
            index: FoldId(1),
            stand_in: StandIn::Member(InstanceId(2)),
        },
        SceneOp::SetRelationConstraints {
            reports: scene().relation_constraints,
            order: vec![InstanceId(2), InstanceId(0), InstanceId(1), InstanceId(3)],
        },
        SceneOp::SetFrame {
            frame: Some(SceneFrame {
                spec: Frame::Flow {
                    origin: Vec2::new(-512.0, -384.0),
                    width: 1024.0,
                },
                space: Scene::WORLD,
            }),
        },
        SceneOp::SetBounds {
            bounds: Rect::new(Vec2::new(-24.0, -24.0), Size2::new(308.0, 88.0)),
        },
        SceneOp::SetGeneration { generation: 10 },
    ]
}

pub fn diff() -> ProjectionDiff {
    ProjectionDiff {
        version: ProtocolVersion::V1,
        session: ProjectionSession("projection:7f3a".into()),
        scene: SceneDiff::new(SceneEpoch(2), Revision(14), Revision(15), operations())
            .with_engine(engine(108), engine(109)),
        presentation: vec![
            PresentationChange::Bind(PresentationBinding {
                instance: InstanceId(3),
                key: PresentationKey("glyph:b".into()),
            }),
            PresentationChange::Unbind {
                instance: InstanceId(1),
            },
            PresentationChange::ReplaceOffers {
                key: PresentationKey("glyph:b".into()),
                offers: vec![offer(
                    PresentationCodec::NativeGlyphV1,
                    5,
                    PresentationCapability::NativeGlyph,
                )],
            },
            PresentationChange::RemoveOffers {
                key: PresentationKey("card:a".into()),
            },
            PresentationChange::InvalidateResource {
                resource: ContentHash([2; 32]),
            },
        ],
        status: Some(SessionStatus::Live),
    }
}
