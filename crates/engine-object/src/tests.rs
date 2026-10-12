use crate::{ObjectId, Registry};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn owns_values_and_never_resolves_stale_or_foreign_identities() {
    let mut first = Registry::default();
    let mut second = Registry::default();
    let old = first.insert(String::from("before")).unwrap();
    first.get_mut(old).unwrap().push_str(" after");
    assert_eq!(first.get(old).map(String::as_str), Some("before after"));
    assert_eq!(second.get(old), None);
    assert_eq!(first.remove(old).as_deref(), Some("before after"));
    let replacement = first.insert(String::from("replacement")).unwrap();
    let foreign = second.insert(String::from("foreign")).unwrap();
    assert_ne!(old, replacement);
    assert_ne!(replacement, foreign);
    assert_eq!(first.get(old), None);
    assert_eq!(first.remove(old), None);
    assert_eq!(first.remove(foreign), None);
    for invalid in [0, -1, i64::MIN] {
        assert_eq!(ObjectId::from_raw(invalid), None);
    }
}

#[test]
fn dropping_registry_releases_every_owned_value_once() {
    struct Tracked(Arc<AtomicUsize>);
    impl Drop for Tracked {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }
    let dropped = Arc::new(AtomicUsize::new(0));
    {
        let mut registry = Registry::default();
        for _ in 0..3 {
            registry.insert(Tracked(Arc::clone(&dropped))).unwrap();
        }
        assert_eq!(dropped.load(Ordering::Relaxed), 0);
    }
    assert_eq!(dropped.load(Ordering::Relaxed), 3);
}

#[test]
fn body_metadata_keeps_hidden_reservations_and_ordered_catalogs() {
    let mut store = crate::ObjectStore::default();
    let (owner, transform) = store.create_gameobject("body").unwrap();
    let script = store.reserve_script(owner).unwrap();
    let body = store.reserve_rigidbody(owner).unwrap();

    assert_eq!(store.kind(body).unwrap(), crate::ObjectKind::Rigidbody);
    assert_eq!(store.component_gameobject(body).unwrap(), owner);
    assert_eq!(store.owner_scripts(owner).unwrap(), &[]);
    assert_eq!(store.owner_component_count(owner).unwrap(), 1);
    assert_eq!(store.owner_component_at(owner, 0).unwrap(), transform);

    store.publish_rigidbody(body).unwrap();
    store.publish_script(script).unwrap();
    assert_eq!(store.owner_scripts(owner).unwrap(), &[script]);
    assert_eq!(store.owner_component_count(owner).unwrap(), 3);
    assert_eq!(store.owner_component_at(owner, 1).unwrap(), body);
    assert_eq!(store.owner_component_at(owner, 2).unwrap(), script);
}

#[test]
fn dropping_rigidbody_reservation_rolls_back_hidden_identity() {
    let mut store = crate::ObjectStore::default();
    let (owner, _) = store.create_gameobject("body").unwrap();

    let reservation = store.prepare_rigidbody_reservation(owner).unwrap();
    let abandoned = reservation.body_id();
    drop(reservation);

    assert!(!store.is_alive(abandoned));
    let replacement = store.reserve_rigidbody(owner).unwrap();
    assert!(store.is_alive(replacement));
}

#[test]
fn committed_rigidbody_reservation_stays_hidden_until_publication() {
    let mut store = crate::ObjectStore::default();
    let (owner, transform) = store.create_gameobject("body").unwrap();

    let body = store.prepare_rigidbody_reservation(owner).unwrap().commit();

    assert!(store.is_alive(body));
    assert_eq!(store.owner_component_count(owner).unwrap(), 1);
    assert_eq!(store.owner_component_at(owner, 0).unwrap(), transform);

    store.publish_rigidbody(body).unwrap();
    assert_eq!(store.owner_component_count(owner).unwrap(), 2);
    assert_eq!(store.owner_component_at(owner, 1).unwrap(), body);
}

#[test]
fn prepared_publication_and_retirement_hold_visibility_until_commit() {
    let mut store = crate::ObjectStore::default();
    let (owner, transform) = store.create_gameobject("body").unwrap();
    let body = store.reserve_rigidbody(owner).unwrap();

    {
        let _publication = store.prepare_rigidbody_publication(body).unwrap();
    }
    assert_eq!(store.owner_component_count(owner).unwrap(), 1);

    store.prepare_rigidbody_publication(body).unwrap().commit();
    assert_eq!(store.owner_component_at(owner, 1).unwrap(), body);

    {
        let _retirement = store.prepare_retirement(&[body]).unwrap();
    }
    assert!(store.is_alive(body));
    store.prepare_retirement(&[body]).unwrap().commit();
    assert_eq!(store.owner_component_count(owner).unwrap(), 1);
    assert_eq!(store.owner_component_at(owner, 0).unwrap(), transform);
}

#[test]
fn failed_rigidbody_publication_preserves_the_outstanding_reservation() {
    let mut store = crate::ObjectStore::default();
    let (owner, transform) = store.create_gameobject("body").unwrap();
    let first = store.reserve_rigidbody(owner).unwrap();
    store.publish_rigidbody(first).unwrap();

    let second = store.reserve_rigidbody(owner).unwrap();
    assert_eq!(
        store.publish_rigidbody(first),
        Err(crate::ObjectError::ComponentState)
    );
    assert_eq!(store.owner_component_count(owner).unwrap(), 2);
    assert_eq!(store.owner_component_at(owner, 0).unwrap(), transform);
    assert_eq!(store.owner_component_at(owner, 1).unwrap(), first);

    store.publish_rigidbody(second).unwrap();
    assert_eq!(store.owner_component_count(owner).unwrap(), 3);
    assert_eq!(store.owner_component_at(owner, 2).unwrap(), second);
}

#[test]
fn complete_release_rejects_partial_owner_without_mutation() {
    let mut store = crate::ObjectStore::default();
    let (owner, transform) = store.create_gameobject("body").unwrap();
    let body = store.reserve_rigidbody(owner).unwrap();
    store.publish_rigidbody(body).unwrap();

    assert_eq!(
        store.prepare_release(&[owner, transform]).err(),
        Some(crate::ObjectError::ComponentState)
    );
    assert!(store.is_alive(owner));
    assert!(store.is_alive(transform));
    assert!(store.is_alive(body));

    store
        .prepare_release(&[owner, transform, body])
        .unwrap()
        .commit();
    assert!(!store.is_alive(owner));
    assert!(!store.is_alive(transform));
    assert!(!store.is_alive(body));
}

#[test]
fn owner_retirement_removes_committed_body_and_script() {
    let mut store = crate::ObjectStore::default();
    let (owner, transform) = store.create_gameobject("body").unwrap();
    let script = store.reserve_script(owner).unwrap();
    let body = store.reserve_rigidbody(owner).unwrap();
    store.publish_script(script).unwrap();
    store.prepare_rigidbody_publication(body).unwrap().commit();

    store.prepare_retirement(&[owner]).unwrap().commit();

    for id in [owner, transform, script, body] {
        assert!(!store.is_alive(id));
    }
}

#[test]
fn direct_owner_removal_rejects_published_rigidbody_without_orphaning_it() {
    let mut store = crate::ObjectStore::default();
    let (owner, transform) = store.create_gameobject("body").unwrap();
    let body = store.reserve_rigidbody(owner).unwrap();
    store.publish_rigidbody(body).unwrap();

    let removal = store.remove_object(owner);
    let post_removal = (
        store.is_alive(owner),
        store.is_alive(transform),
        store.is_alive(body),
        store.component_gameobject(body).ok(),
    );

    assert_eq!(
        removal,
        Err(crate::ObjectError::ComponentState),
        "a published Rigidbody must keep direct owner removal from retiring its GameObject; \
         post-removal owner/Transform/body/owner-link state: {post_removal:?}"
    );
    assert!(store.is_alive(owner));
    assert!(store.is_alive(transform));
    assert!(store.is_alive(body));
    assert_eq!(store.component_gameobject(body), Ok(owner));
    assert_eq!(store.owner_component_count(owner), Ok(2));
    assert_eq!(store.owner_component_at(owner, 0), Ok(transform));
    assert_eq!(store.owner_component_at(owner, 1), Ok(body));

    store.remove_object(body).unwrap();
    store.remove_object(owner).unwrap();

    assert!(!store.is_alive(owner));
    assert!(!store.is_alive(transform));
}

#[test]
fn direct_retirement_prunes_hidden_reservations_and_keeps_owner() {
    let mut store = crate::ObjectStore::default();
    let (owner, transform) = store.create_gameobject("body").unwrap();
    let script = store.reserve_script(owner).unwrap();
    let body = store.reserve_rigidbody(owner).unwrap();

    store.prepare_retirement(&[script, body]).unwrap().commit();

    assert!(store.is_alive(owner));
    assert!(store.is_alive(transform));
    assert!(!store.is_alive(script));
    assert!(!store.is_alive(body));
    assert_eq!(store.owner_scripts(owner).unwrap(), &[]);
    assert_eq!(store.owner_component_count(owner).unwrap(), 1);

    let replacement = store.reserve_rigidbody(owner).unwrap();
    assert!(store.is_alive(replacement));
}
