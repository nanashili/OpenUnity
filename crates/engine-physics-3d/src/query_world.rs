//! Deterministic owner-bound AABB queries independent from dynamic motion.

use std::collections::{BTreeMap, BTreeSet};

use engine_identifiers::ObjectId;
use engine_math::{Aabb, MathError, Vector, finite};
use engine_physics_api::{QueryError, RaySegmentHit};

/// Maximum number of registered query boxes in one world.
pub const MAX_QUERY_BOXES: usize = 256;

/// A bounded world of explicit, world-space query boxes.
#[derive(Debug, Default)]
pub struct AabbQueryWorld {
    boxes: BTreeMap<i64, (ObjectId, Aabb)>,
    revision: u64,
}

#[derive(Clone, Copy, Debug)]
struct SegmentHitCandidate {
    owner: ObjectId,
    entry_parameter: f64,
    point: Vector,
}

impl SegmentHitCandidate {
    fn precedes(self, other: Self) -> bool {
        self.entry_parameter < other.entry_parameter
            || (self.entry_parameter == other.entry_parameter
                && self.owner.to_raw() < other.owner.to_raw())
    }

    fn into_public_hit(self) -> Result<RaySegmentHit, QueryError> {
        RaySegmentHit::try_new(self.owner, self.entry_parameter as f32, self.point)
    }
}

impl AabbQueryWorld {
    /// Register one box for an owner without partially changing this world.
    pub fn add_aabb(&mut self, owner: ObjectId, bounds: Aabb) -> Result<(), QueryError> {
        let key = owner.to_raw();

        if self.boxes.contains_key(&key) {
            return Err(QueryError::DuplicateOwner);
        }

        if self.boxes.len() >= MAX_QUERY_BOXES {
            return Err(QueryError::Capacity);
        }

        let next_revision = self.next_revision()?;
        self.boxes.insert(key, (owner, bounds));
        self.revision = next_revision;
        Ok(())
    }

    /// Register one finite, ordered box from its endpoint vectors.
    pub fn add_aabb_from_vectors(
        &mut self,
        owner: ObjectId,
        min: Vector,
        max: Vector,
    ) -> Result<(), QueryError> {
        let bounds = Aabb::try_new(min, max).map_err(map_math_error)?;
        self.add_aabb(owner, bounds)
    }

    /// Return the nearest closed-segment hit, retaining the lower owner on ties.
    pub fn raycast_segment(
        &self,
        origin: Vector,
        displacement: Vector,
    ) -> Result<Option<RaySegmentHit>, QueryError> {
        finite(&origin).map_err(map_math_error)?;
        finite(&displacement).map_err(map_math_error)?;
        if displacement == [0.0; 3] {
            return Err(QueryError::InvalidQuery);
        }

        let mut nearest: Option<SegmentHitCandidate> = None;

        for (owner, bounds) in self.boxes.values() {
            let Some((entry_parameter, point)) = bounds
                .intersect_segment(origin, displacement)
                .map_err(map_math_error)?
            else {
                continue;
            };

            let candidate = SegmentHitCandidate {
                owner: *owner,
                entry_parameter,
                point,
            };
            let is_nearer = nearest.is_none_or(|hit| candidate.precedes(hit));
            if is_nearer {
                nearest = Some(candidate);
            }
        }

        nearest
            .map(SegmentHitCandidate::into_public_hit)
            .transpose()
    }

    /// Return all closed-volume overlaps in ascending raw owner order.
    pub fn overlap_aabb(&self, bounds: &Aabb) -> Result<Vec<ObjectId>, QueryError> {
        Ok(self
            .boxes
            .values()
            .filter_map(|(owner, candidate)| candidate.intersects(bounds).then_some(*owner))
            .collect())
    }

    /// Validate a borrow-bound owner retirement before any record is removed.
    pub fn prepare_owner_retirement(
        &mut self,
        owners: &[ObjectId],
    ) -> Result<PreparedOwnerRetirement<'_>, QueryError> {
        let owner_keys = owners
            .iter()
            .map(|owner| owner.to_raw())
            .filter(|key| self.boxes.contains_key(key))
            .collect::<BTreeSet<_>>();
        let next_revision = if owner_keys.is_empty() {
            self.revision
        } else {
            self.next_revision()?
        };

        Ok(PreparedOwnerRetirement {
            world: self,
            owner_keys: owner_keys.into_iter().collect(),
            next_revision,
        })
    }

    fn next_revision(&self) -> Result<u64, QueryError> {
        self.revision
            .checked_add(1)
            .ok_or(QueryError::RevisionExhausted)
    }
}

/// A prepared query-record retirement that is inert unless explicitly committed.
pub struct PreparedOwnerRetirement<'world> {
    world: &'world mut AabbQueryWorld,
    owner_keys: Vec<i64>,
    next_revision: u64,
}

impl PreparedOwnerRetirement<'_> {
    /// Remove the prepared records and advance the world revision once.
    pub fn commit(self) {
        let Self {
            world,
            owner_keys,
            next_revision,
        } = self;

        for owner_key in owner_keys {
            world.boxes.remove(&owner_key);
        }
        world.revision = next_revision;
    }
}

fn map_math_error(error: MathError) -> QueryError {
    match error {
        MathError::Numeric => QueryError::Numeric,
        MathError::Unsupported => QueryError::InvalidQuery,
    }
}
