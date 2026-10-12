//! Ownership-safe opaque identities. Implements INFRA-OBJECT-001, not Unity object semantics.
#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::fmt;
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// A process-local identity, never an address or a serialized Unity file ID.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ObjectId(NonZeroU64);

impl ObjectId {
    /// Decode a positive ABI identity. Liveness still requires a registry lookup.
    pub fn from_raw(raw: i64) -> Option<Self> {
        NonZeroU64::new(u64::try_from(raw).ok()?).map(Self)
    }

    /// Return the fixed-width ABI representation.
    pub fn to_raw(self) -> i64 {
        // Construction is restricted to the positive signed 63-bit range.
        self.0.get() as i64
    }
}

/// No further identities can be allocated without risking identity reuse.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IdentityExhausted;

impl fmt::Display for IdentityExhausted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("process identity space exhausted")
    }
}

impl std::error::Error for IdentityExhausted {}

fn allocate(counter: &AtomicU64) -> Result<ObjectId, IdentityExhausted> {
    // The value after i64::MAX is a permanent exhaustion sentinel. Relaxed
    // ordering suffices for uniqueness; registry synchronization owns values.
    let raw = counter
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |next| {
            (next <= i64::MAX as u64).then_some(next + 1)
        })
        .map_err(|_| IdentityExhausted)?;
    NonZeroU64::new(raw).map(ObjectId).ok_or(IdentityExhausted)
}

/// Owns values independently of their stable process-local identities.
#[derive(Debug)]
pub struct Registry<T> {
    values: HashMap<ObjectId, T>,
}

impl<T> Default for Registry<T> {
    fn default() -> Self {
        Self {
            values: HashMap::new(),
        }
    }
}

impl<T> Registry<T> {
    /// Insert a value with a new identity, distinct from every prior allocation.
    ///
    /// # Errors
    /// Returns [`IdentityExhausted`] when the 63-bit identity space is exhausted.
    pub fn insert(&mut self, value: T) -> Result<ObjectId, IdentityExhausted> {
        let id = allocate(&NEXT_ID)?;
        self.values.insert(id, value);
        Ok(id)
    }

    /// Reserve enough space to transfer all values from another private registry.
    /// # Errors
    /// Returns a capacity/allocation error without changing either registry's values.
    pub fn prepare_append(
        &mut self,
        other: &Self,
    ) -> Result<(), std::collections::TryReserveError> {
        self.values.try_reserve(other.values.len())
    }

    /// Move process-unique identities after successful capacity preparation.
    /// The caller must call `prepare_append` without inserting intervening values.
    pub fn append_reserved(&mut self, other: Self) {
        self.values.extend(other.values);
    }

    /// Borrow a live value belonging to this registry.
    pub fn get(&self, id: ObjectId) -> Option<&T> {
        self.values.get(&id)
    }

    /// Mutably borrow a live value belonging to this registry.
    pub fn get_mut(&mut self, id: ObjectId) -> Option<&mut T> {
        self.values.get_mut(&id)
    }

    /// Remove a live value, transferring ownership to the caller.
    pub fn remove(&mut self, id: ObjectId) -> Option<T> {
        self.values.remove(&id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::AtomicUsize;

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
    fn identity_exhaustion_never_wraps_or_recycles() {
        let counter = AtomicU64::new(i64::MAX as u64);
        assert_eq!(allocate(&counter).unwrap().to_raw(), i64::MAX);
        assert_eq!(allocate(&counter), Err(IdentityExhausted));
        assert_eq!(allocate(&counter), Err(IdentityExhausted));
    }
}
