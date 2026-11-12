use super::*;

#[test]
fn write_then_read_round_trips() {
    let mut store = NodeFacetStore::new();
    let a = Uuid::from_u128(1);
    let b = Uuid::from_u128(2);
    write_arrangement_positions(&mut store, [(a, (10.0, 20.0)), (b, (-30.5, 0.0))]);
    let mut read = read_arrangement_positions(&store);
    read.sort_by_key(|(id, _)| *id);
    assert_eq!(read, vec![(a, (10.0, 20.0)), (b, (-30.5, 0.0))]);
}

#[test]
fn a_rewrite_clears_positions_absent_from_the_new_set() {
    let mut store = NodeFacetStore::new();
    let a = Uuid::from_u128(1);
    let b = Uuid::from_u128(2);
    write_arrangement_positions(&mut store, [(a, (1.0, 1.0)), (b, (2.0, 2.0))]);
    // The next save has no position for b (deleted / unplaced).
    write_arrangement_positions(&mut store, [(a, (5.0, 5.0))]);
    assert_eq!(read_arrangement_positions(&store), vec![(a, (5.0, 5.0))]);
}

#[test]
fn a_rewrite_leaves_other_namespaces_untouched() {
    let mut store = NodeFacetStore::new();
    let a = Uuid::from_u128(1);
    store
        .set(
            a,
            FacetId::new("web.viewer"),
            json!({ "mode": "reader" }),
            &AcceptAll,
        )
        .unwrap();
    write_arrangement_positions(&mut store, [(a, (3.0, 4.0))]);
    write_arrangement_positions(&mut store, []);
    assert_eq!(
        store.get(&a, &FacetId::new("web.viewer")),
        Some(&json!({ "mode": "reader" })),
        "the position rewrite must not disturb web.* facets"
    );
    assert!(read_arrangement_positions(&store).is_empty());
}

#[test]
fn malformed_and_non_finite_payloads_are_skipped() {
    let mut store = NodeFacetStore::new();
    let good = Uuid::from_u128(1);
    let bad_shape = Uuid::from_u128(2);
    let bad_nan = Uuid::from_u128(3);
    let facet = arrangement_position_facet();
    store
        .set(
            good,
            facet.clone(),
            json!({ "x": 7.0, "y": 8.0 }),
            &AcceptAll,
        )
        .unwrap();
    store
        .set(bad_shape, facet.clone(), json!([7.0, 8.0]), &AcceptAll)
        .unwrap();
    // JSON has no NaN literal; a null coordinate stands in for the
    // unparsable case (as_f64 fails the same way).
    store
        .set(
            bad_nan,
            facet.clone(),
            json!({ "x": null, "y": 1.0 }),
            &AcceptAll,
        )
        .unwrap();
    assert_eq!(read_arrangement_positions(&store), vec![(good, (7.0, 8.0))]);
}

#[test]
fn every_family_member_round_trips() {
    let mut store = NodeFacetStore::new();
    let a = Uuid::from_u128(1);
    write_arrangement_sizes(&mut store, [(a, 48.0)]);
    write_arrangement_sprites(&mut store, [(a, "data:image/png;base64,AAAA")]);
    write_arrangement_sprite_hulls(
        &mut store,
        [(a, vec![(-0.5, -0.5), (0.5, 0.0), (0.0, 0.5)])],
    );
    write_arrangement_materials(&mut store, [(a, (0.8, 0.2, 3.0))]);
    write_arrangement_faces(&mut store, [(a, "sprite")]);

    assert_eq!(read_arrangement_sizes(&store), vec![(a, 48.0)]);
    assert_eq!(
        read_arrangement_sprites(&store),
        vec![(a, "data:image/png;base64,AAAA".to_string())]
    );
    assert_eq!(
        read_arrangement_sprite_hulls(&store),
        vec![(a, vec![(-0.5, -0.5), (0.5, 0.0), (0.0, 0.5)])]
    );
    assert_eq!(
        read_arrangement_materials(&store),
        vec![(a, (0.8, 0.2, 3.0))]
    );
    assert_eq!(
        read_arrangement_faces(&store),
        vec![(a, "sprite".to_string())]
    );
}

#[test]
fn each_family_rewrite_clears_only_its_own_facet() {
    let mut store = NodeFacetStore::new();
    let a = Uuid::from_u128(1);
    write_arrangement_sizes(&mut store, [(a, 48.0)]);
    write_arrangement_faces(&mut store, [(a, "bare")]);
    // Rewriting sizes to empty must not disturb the face facet.
    write_arrangement_sizes(&mut store, []);
    assert!(read_arrangement_sizes(&store).is_empty());
    assert_eq!(
        read_arrangement_faces(&store),
        vec![(a, "bare".to_string())]
    );
}

#[test]
fn a_hull_with_a_malformed_point_is_skipped_whole() {
    let mut store = NodeFacetStore::new();
    let a = Uuid::from_u128(1);
    store
        .set(
            a,
            FacetId::new(ARRANGEMENT_SPRITE_HULL),
            json!([[0.1, 0.2], [null, 0.3]]),
            &AcceptAll,
        )
        .unwrap();
    assert!(
        read_arrangement_sprite_hulls(&store).is_empty(),
        "a partial polygon is a wrong collider; drop the hull"
    );
}

#[test]
fn seed_round_trips_and_none_is_the_recipe_seed() {
    let mut store = NodeFacetStore::new();
    let a = Uuid::from_u128(1);
    let b = Uuid::from_u128(2);
    assert_eq!(read_arrangement_seed(&store), None);
    write_arrangement_seed(&mut store, Some([(a, (1.0, 2.0)), (b, (-3.5, 0.0))]));
    let mut seed = read_arrangement_seed(&store).unwrap();
    seed.sort_by_key(|(id, _)| *id);
    assert_eq!(seed, vec![(a, (1.0, 2.0)), (b, (-3.5, 0.0))]);
    // Resetting to the recipe seed clears the stale places.
    write_arrangement_seed(&mut store, None::<[(Uuid, (f32, f32)); 0]>);
    assert_eq!(read_arrangement_seed(&store), None);
}

#[test]
fn a_malformed_seed_place_is_skipped_and_all_malformed_reads_none() {
    let mut store = NodeFacetStore::new();
    let good = Uuid::from_u128(1);
    let bad = Uuid::from_u128(2);
    let facet = FacetId::new(ARRANGEMENT_SEED);
    store
        .set(
            bad,
            facet.clone(),
            json!({ "x": null, "y": 1.0 }),
            &AcceptAll,
        )
        .unwrap();
    assert_eq!(read_arrangement_seed(&store), None);
    store
        .set(good, facet, json!({ "x": 4.0, "y": 5.0 }), &AcceptAll)
        .unwrap();
    assert_eq!(
        read_arrangement_seed(&store),
        Some(vec![(good, (4.0, 5.0))])
    );
}

fn sample_regions() -> Vec<SavedRegion> {
    vec![
        SavedRegion {
            contour: vec![(0.0, 0.0), (10.0, 0.0), (5.0, 8.0)],
            label: Some("Reading".into()),
            members: vec![Uuid::from_u128(1), Uuid::from_u128(2)],
        },
        SavedRegion {
            contour: vec![(20.0, 20.0), (30.0, 25.0)],
            label: None,
            members: vec![Uuid::from_u128(3)],
        },
    ]
}

#[test]
fn regions_round_trip_in_order_and_a_rewrite_clears_them() {
    let mut store = NodeFacetStore::new();
    let scene = Uuid::from_u128(100);
    write_arrangement_regions(&mut store, scene, &sample_regions());
    assert_eq!(read_arrangement_regions(&store, scene), sample_regions());
    write_arrangement_regions(&mut store, scene, &[]);
    assert!(read_arrangement_regions(&store, scene).is_empty());
    assert!(
        store
            .facets_of(&scene)
            .is_none_or(|f| !f.has(&FacetId::new(ARRANGEMENT_REGIONS)))
    );
}

#[test]
fn malformed_regions_fall_back_to_none_without_touching_other_groups() {
    let mut store = NodeFacetStore::new();
    let scene = Uuid::from_u128(100);
    let a = Uuid::from_u128(1);
    write_arrangement_sizes(&mut store, [(a, 48.0)]);
    write_arrangement_mosaic(&mut store, scene, Some(&sample_mosaic()));
    store
        .set(
            scene,
            FacetId::new(ARRANGEMENT_REGIONS),
            json!([{ "contour": [[0.0, null]], "label": null, "members": [] }]),
            &AcceptAll,
        )
        .unwrap();
    assert!(read_arrangement_regions(&store, scene).is_empty());
    assert_eq!(read_arrangement_sizes(&store), vec![(a, 48.0)]);
    assert_eq!(
        read_arrangement_mosaic(&store, scene),
        Some(sample_mosaic())
    );
}

fn sample_mosaic() -> SavedMosaic {
    SavedMosaic {
        group_of: vec![(Uuid::from_u128(1), "docs".into())],
        group_order: vec!["docs".into(), "news".into()],
        member_order: vec![Uuid::from_u128(2), Uuid::from_u128(1)],
        scale: vec![(Uuid::from_u128(2), 1.5)],
    }
}

#[test]
fn mosaic_round_trips_and_none_clears_it() {
    let mut store = NodeFacetStore::new();
    let scene = Uuid::from_u128(100);
    assert_eq!(read_arrangement_mosaic(&store, scene), None);
    write_arrangement_mosaic(&mut store, scene, Some(&sample_mosaic()));
    assert_eq!(
        read_arrangement_mosaic(&store, scene),
        Some(sample_mosaic())
    );
    write_arrangement_mosaic(&mut store, scene, None);
    assert_eq!(read_arrangement_mosaic(&store, scene), None);
}

#[test]
fn a_malformed_mosaic_reads_none() {
    let mut store = NodeFacetStore::new();
    let scene = Uuid::from_u128(100);
    let facet = FacetId::new(ARRANGEMENT_MOSAIC);
    store
        .set(scene, facet.clone(), json!("not a mosaic"), &AcceptAll)
        .unwrap();
    assert_eq!(read_arrangement_mosaic(&store, scene), None);
    store
        .set(
            scene,
            facet,
            json!({ "scale": [[Uuid::nil(), null]] }),
            &AcceptAll,
        )
        .unwrap();
    assert_eq!(read_arrangement_mosaic(&store, scene), None);
}

#[test]
fn a_facet_set_from_before_seed_regions_and_mosaic_still_loads() {
    let mut store = NodeFacetStore::new();
    let scene = Uuid::from_u128(100);
    let a = Uuid::from_u128(1);
    write_arrangement_positions(&mut store, [(a, (1.0, 2.0))]);
    write_arrangement_sizes(&mut store, [(a, 48.0)]);
    write_arrangement_faces(&mut store, [(a, "bare")]);
    assert_eq!(read_arrangement_positions(&store), vec![(a, (1.0, 2.0))]);
    assert_eq!(read_arrangement_sizes(&store), vec![(a, 48.0)]);
    assert_eq!(read_arrangement_seed(&store), None);
    assert!(read_arrangement_regions(&store, scene).is_empty());
    assert_eq!(read_arrangement_mosaic(&store, scene), None);
}

#[test]
fn the_new_groups_rewrite_independently_of_the_rest_of_the_family() {
    let mut store = NodeFacetStore::new();
    let scene = Uuid::from_u128(100);
    let a = Uuid::from_u128(1);
    write_arrangement_seed(&mut store, Some([(a, (1.0, 1.0))]));
    write_arrangement_regions(&mut store, scene, &sample_regions());
    write_arrangement_mosaic(&mut store, scene, Some(&sample_mosaic()));
    write_arrangement_regions(&mut store, scene, &[]);
    assert_eq!(read_arrangement_seed(&store), Some(vec![(a, (1.0, 1.0))]));
    assert_eq!(
        read_arrangement_mosaic(&store, scene),
        Some(sample_mosaic())
    );
}

#[test]
fn retain_present_drops_departed_nodes_entirely() {
    let mut store = NodeFacetStore::new();
    let live = Uuid::from_u128(1);
    let departed = Uuid::from_u128(2);
    write_arrangement_positions(&mut store, [(live, (1.0, 2.0)), (departed, (3.0, 4.0))]);
    store
        .set(
            departed,
            FacetId::new("some-mod.exotic"),
            json!(true),
            &AcceptAll,
        )
        .unwrap();
    let present: BTreeSet<Uuid> = [live].into_iter().collect();
    retain_present_nodes(&mut store, &present);
    assert_eq!(read_arrangement_positions(&store), vec![(live, (1.0, 2.0))]);
    assert!(
        store.facets_of(&departed).is_none(),
        "a departed node's whole facet record goes with it"
    );
}
