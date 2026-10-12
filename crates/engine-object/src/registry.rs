use crate::{IdentityExhausted, ObjectId};
use engine_identifiers::allocate_next;
use std::collections::HashMap;

/// Owns values independently of their stable process-local identities.
#[derive(Debug)]
pub struct Registry<T> {
    values: HashMap<ObjectId, T>,
}

/// A registry value detached by a transaction that still owns restoration.
#[derive(Debug)]
pub struct DetachedEntry<T> {
    id: ObjectId,
    value: T,
}

impl<T> DetachedEntry<T> {
    /// Restore the exact identity/value pair after the peer operation fails.
    pub fn restore(self, registry: &mut Registry<T>) {
        registry.values.insert(self.id, self.value);
    }

    /// Discard the detached value after every peer commit succeeds.
    pub fn discard(self) {}
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
        let id = allocate_next()?;
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

    /// Detach an identity so a transaction can restore it after a peer operation
    /// fails before commit.
    pub fn detach(&mut self, id: ObjectId) -> Option<DetachedEntry<T>> {
        self.values
            .remove(&id)
            .map(|value| DetachedEntry { id, value })
    }
}
