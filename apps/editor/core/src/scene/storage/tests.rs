use crate::scene::{Primitive, Transform};

use super::*;

fn fixture(id: u64, parent: Option<u64>) -> Entity {
    Entity {
        id: EntityId(id),
        name: format!("Entity {id}"),
        parent: parent.map(EntityId),
        active: true,
        transform: Transform::default(),
        primitive: Primitive::Empty,
    }
}

#[test]
fn subtree_removal_and_history_restore_never_reuse_allocated_ids() {
    let mut scene = Scene::new("Scene").unwrap();
    let root = scene.create("Root", Primitive::Cube, None).unwrap();
    let child = scene.create("Child", Primitive::Empty, Some(root)).unwrap();
    let snapshot = scene.clone();
    let unrelated = scene.create("Camera", Primitive::Camera, None).unwrap();
    assert_eq!(scene.remove_subtree(root).unwrap(), vec![root, child]);
    assert!(scene.get(unrelated).is_some());
    assert!(scene.get(child).is_none());
    let high_water = scene.next_id();
    scene = snapshot;
    scene.preserve_high_water(high_water).unwrap();
    assert!(scene.get(child).is_some());
    assert_eq!(
        scene.create("New", Primitive::Empty, None).unwrap(),
        EntityId(4)
    );
    assert_eq!(
        scene.entities().map(|entity| entity.id).collect::<Vec<_>>(),
        vec![EntityId(1), EntityId(2), EntityId(4)]
    );
}

#[test]
fn invalid_entity_edits_are_atomic() {
    let mut scene = Scene::new("Scene").unwrap();
    let root = scene.create("Root", Primitive::Empty, None).unwrap();
    let child = scene.create("Child", Primitive::Empty, Some(root)).unwrap();
    let before = scene.clone();
    let invalid: [fn(&mut Entity); 7] = [
        |entity| entity.name.clear(),
        |entity| entity.name = "é".repeat(513),
        |entity| entity.transform.translation[0] = f32::NAN,
        |entity| entity.transform.rotation_deg[2] = f32::INFINITY,
        |entity| entity.transform.scale[1] = f32::NEG_INFINITY,
        |entity| entity.parent = Some(EntityId(999)),
        |entity| entity.id = EntityId(999),
    ];
    for edit in invalid {
        assert!(scene.edit(root, edit).is_err());
        assert_eq!(scene, before);
    }
    assert!(
        scene
            .edit(root, |entity| entity.parent = Some(child))
            .is_err()
    );
    assert!(
        scene
            .edit(root, |entity| entity.parent = Some(root))
            .is_err()
    );
    assert!(scene.edit(EntityId(999), |_| {}).is_err());
    assert_eq!(scene, before);
    scene
        .edit(child, |entity| {
            entity.active = false;
            entity.transform.scale = [-1.0, 0.0, 2.0];
            entity.name = "é".repeat(512);
        })
        .unwrap();
    assert!(!scene.get(child).unwrap().active);
    assert_eq!(scene.get(child).unwrap().transform.scale, [-1.0, 0.0, 2.0]);
}

#[test]
fn reconstruction_rejects_invalid_ids_parents_cycles_and_counters() {
    let cases = [
        (2, vec![fixture(0, None)]),
        (2, vec![fixture(1, None), fixture(1, None)]),
        (1, vec![fixture(1, None)]),
        (0, vec![]),
        (2, vec![fixture(1, Some(2))]),
        (2, vec![fixture(1, Some(1))]),
        (3, vec![fixture(1, Some(2)), fixture(2, Some(1))]),
    ];
    for (counter, entities) in cases {
        assert!(Scene::from_entities("Scene", counter, entities).is_err());
    }
    let scene =
        Scene::from_entities("Scene", 3, vec![fixture(2, None), fixture(1, Some(2))]).unwrap();
    assert_eq!(
        scene.entities().map(|entity| entity.id).collect::<Vec<_>>(),
        vec![EntityId(1), EntityId(2)]
    );
}

#[test]
fn hierarchy_depth_limit_applies_to_creation_loading_and_reparenting() {
    let chain: Vec<_> = (1..=256)
        .map(|id| fixture(id, (id > 1).then_some(id - 1)))
        .collect();
    let mut scene = Scene::from_entities("Scene", 257, chain.clone()).unwrap();
    let before = scene.clone();
    assert!(
        scene
            .create("Too deep", Primitive::Empty, Some(EntityId(256)))
            .is_err()
    );
    assert_eq!(scene, before);
    let extra = scene.create("Root", Primitive::Empty, None).unwrap();
    let before = scene.clone();
    assert!(
        scene
            .edit(EntityId(1), |entity| entity.parent = Some(extra))
            .is_err()
    );
    assert_eq!(scene, before);
    let mut too_deep = chain;
    too_deep.push(fixture(257, Some(256)));
    assert!(Scene::from_entities("Scene", 258, too_deep).is_err());
}

#[test]
fn names_entity_limit_and_identity_exhaustion_preserve_state() {
    assert!(Scene::new("").is_err());
    assert!(Scene::new("é".repeat(513)).is_err());
    let mut scene = Scene::new("é".repeat(512)).unwrap();
    let before = scene.clone();
    assert!(scene.set_name("").is_err());
    assert!(scene.create("", Primitive::Empty, None).is_err());
    assert!(
        scene
            .create("Missing", Primitive::Empty, Some(EntityId(1)))
            .is_err()
    );
    assert!(scene.remove_subtree(EntityId(1)).is_err());
    assert!(scene.preserve_high_water(0).is_err());
    assert_eq!(scene, before);
    scene.preserve_high_water(u64::MAX - 1).unwrap();
    assert_eq!(
        scene.create("Last", Primitive::Empty, None).unwrap(),
        EntityId(u64::MAX - 1)
    );
    let before = scene.clone();
    assert!(scene.create("Exhausted", Primitive::Empty, None).is_err());
    assert_eq!(scene, before);
    let entities: Vec<_> = (1..=100_000).map(|id| fixture(id, None)).collect();
    let mut full = Scene::from_entities("Full", 100_001, entities.clone()).unwrap();
    assert!(full.create("Extra", Primitive::Empty, None).is_err());
    assert_eq!(full.entities().len(), 100_000);
    assert_eq!(full.next_id(), 100_001);
    let mut overfull = entities;
    overfull.push(fixture(100_001, None));
    assert!(Scene::from_entities("Overfull", 100_002, overfull).is_err());
}

#[test]
fn entity_wire_types_reject_unknown_and_duplicate_fields() {
    for json in [
        r#"{"translation":[0,0,0],"rotation_deg":[0,0,0],"scale":[1,1,1],"extra":true}"#,
        r#"{"translation":[0,0,0],"translation":[1,1,1],"rotation_deg":[0,0,0],"scale":[1,1,1]}"#,
    ] {
        assert!(serde_json::from_str::<Transform>(json).is_err());
    }
    let mut value = serde_json::to_value(fixture(1, None)).unwrap();
    value["unexpected"] = serde_json::Value::Bool(true);
    assert!(serde_json::from_value::<Entity>(value).is_err());
}
