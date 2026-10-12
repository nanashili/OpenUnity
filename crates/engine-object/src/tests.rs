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
