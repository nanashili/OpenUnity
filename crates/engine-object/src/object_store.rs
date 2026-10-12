//! Private object metadata, independent of hierarchy poses and callback scheduling.

use crate::{IdentityExhausted, ObjectId};
use engine_identifiers::allocate_next;
use std::collections::HashMap;

/// Supported identity categories, independent of ABI numeric encodings.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObjectKind {
    /// An independent object without components.
    Plain,
    /// An owner with one mandatory Transform and optional attached scripts.
    GameObject,
    /// The mandatory component identity belonging to one GameObject.
    Transform,
    /// A reserved or published custom script component.
    Script,
}

#[derive(Debug)]
pub(crate) enum ObjectRecord {
    Plain {
        name: String,
    },
    GameObject {
        name: String,
        transform: ObjectId,
        active_self: bool,
        scripts: Vec<ObjectId>,
        reserved_scripts: Vec<ObjectId>,
    },
    Transform {
        owner: ObjectId,
    },
    Script {
        owner: ObjectId,
    },
}

/// Owns stable identities, names, and mandatory component relationships.
/// Hierarchy data and callback execution belong to the composing runtime.
#[derive(Debug, Default)]
pub struct ObjectStore {
    pub(crate) records: HashMap<ObjectId, ObjectRecord>,
}

impl ObjectStore {
    /// Allocate a plain object with an empty owned name.
    /// # Errors
    /// Returns [`IdentityExhausted`] without publishing an object.
    pub fn create_object(&mut self) -> Result<ObjectId, IdentityExhausted> {
        let id = allocate_next()?;
        self.records.insert(
            id,
            ObjectRecord::Plain {
                name: String::new(),
            },
        );
        Ok(id)
    }

    /// Allocate a GameObject and its mandatory Transform as one owned pair.
    /// # Errors
    /// Returns [`IdentityExhausted`] without publishing either identity.
    pub fn create_gameobject(
        &mut self,
        name: &str,
    ) -> Result<(ObjectId, ObjectId), IdentityExhausted> {
        let owner = allocate_next()?;
        let transform = allocate_next()?;

        self.records.insert(
            owner,
            ObjectRecord::GameObject {
                name: name.to_owned(),
                transform,
                active_self: true,
                scripts: Vec::new(),
                reserved_scripts: Vec::new(),
            },
        );
        self.records
            .insert(transform, ObjectRecord::Transform { owner });
        Ok((owner, transform))
    }

    /// Whether this store owns the live identity.
    pub fn is_alive(&self, id: ObjectId) -> bool {
        self.records.contains_key(&id)
    }

    /// Prepare capacity before moving a staged store into this store.
    /// # Errors
    /// Returns a capacity error without changing either store's values.
    pub fn prepare_append(
        &mut self,
        other: &Self,
    ) -> Result<(), std::collections::TryReserveError> {
        self.records.try_reserve(other.records.len())
    }

    /// Move globally unique identities after successful capacity preparation.
    /// No insertion may intervene after `prepare_append`.
    pub fn append_reserved(&mut self, other: Self) {
        self.records.extend(other.records);
    }
}
