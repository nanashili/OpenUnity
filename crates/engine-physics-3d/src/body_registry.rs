//! Exact-ID storage and prepared mutations for bounded motion-world bodies.

use engine_identifiers::ObjectId;
use engine_physics_api::{BodyId, BodyState, PhysicsError};

/// Maximum number of reserved and committed bodies in one motion world.
pub const MAX_MOTION_BODIES: usize = 256;

#[derive(Clone, Copy, Debug)]
pub(crate) struct BodyRecord {
    pub(crate) id: BodyId,
    pub(crate) state: BodyState,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum BodySlot {
    Reserved(BodyRecord),
    Committed(BodyRecord),
}

impl BodySlot {
    fn record(self) -> BodyRecord {
        match self {
            Self::Reserved(record) | Self::Committed(record) => record,
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct BodyRegistry {
    slots: Vec<BodySlot>,
}

pub(crate) struct PreparedReplacements {
    records: [BodyRecord; MAX_MOTION_BODIES],
    indices: [usize; MAX_MOTION_BODIES],
    len: usize,
}

pub(crate) struct PreparedRetirement {
    indices: [usize; MAX_MOTION_BODIES],
    len: usize,
}

impl BodyRegistry {
    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    pub(crate) fn ensure_slot_available(&self) -> Result<(), PhysicsError> {
        if self.slots.len() >= MAX_MOTION_BODIES {
            return Err(PhysicsError::Capacity);
        }
        Ok(())
    }

    pub(crate) fn ensure_reservation_capacity(&mut self) -> Result<(), PhysicsError> {
        self.ensure_slot_available()?;
        self.slots
            .try_reserve(1)
            .map_err(|_| PhysicsError::Capacity)
    }

    pub(crate) fn insertion_index(&self, id: BodyId) -> Result<usize, PhysicsError> {
        let key = raw_id(id);
        match self
            .slots
            .binary_search_by_key(&key, |slot| raw_id(slot.record().id))
        {
            Ok(_) => Err(PhysicsError::InvalidState),
            Err(index) => Ok(index),
        }
    }

    pub(crate) fn insert_reserved(&mut self, index: usize, record: BodyRecord) {
        debug_assert!(self.slots.len() < MAX_MOTION_BODIES);
        debug_assert!(self.slots.capacity() > self.slots.len());
        self.slots.insert(index, BodySlot::Reserved(record));
    }

    pub(crate) fn exact_slot(&self, id: BodyId) -> Result<(usize, BodySlot), PhysicsError> {
        let key = raw_id(id);
        let index = self
            .slots
            .binary_search_by_key(&key, |slot| raw_id(slot.record().id))
            .map_err(|_| PhysicsError::InvalidBody)?;

        Ok((index, self.slots[index]))
    }

    pub(crate) fn reserved_body(&self, id: BodyId) -> Result<(usize, BodyRecord), PhysicsError> {
        let (index, slot) = self.exact_slot(id)?;
        match slot {
            BodySlot::Reserved(record) => Ok((index, record)),
            BodySlot::Committed(_) => Err(PhysicsError::InvalidState),
        }
    }

    pub(crate) fn committed_body(&self, id: BodyId) -> Result<(usize, BodyRecord), PhysicsError> {
        let (index, slot) = self.exact_slot(id)?;
        match slot {
            BodySlot::Committed(record) => Ok((index, record)),
            BodySlot::Reserved(_) => Err(PhysicsError::InvalidState),
        }
    }

    pub(crate) fn publish_committed(&mut self, index: usize, record: BodyRecord) {
        debug_assert!(matches!(self.slots[index], BodySlot::Reserved(_)));
        self.slots[index] = BodySlot::Committed(record);
    }

    pub(crate) fn replace_committed(&mut self, index: usize, record: BodyRecord) {
        debug_assert!(matches!(self.slots[index], BodySlot::Committed(_)));
        self.slots[index] = BodySlot::Committed(record);
    }

    pub(crate) fn remove_at(&mut self, index: usize) {
        self.slots.remove(index);
    }

    pub(crate) fn committed_count(&self) -> usize {
        self.slots
            .iter()
            .filter(|slot| matches!(slot, BodySlot::Committed(_)))
            .count()
    }

    pub(crate) fn committed_records(&self) -> impl Iterator<Item = (usize, BodyRecord)> + '_ {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(index, slot)| match slot {
                BodySlot::Committed(record) => Some((index, *record)),
                BodySlot::Reserved(_) => None,
            })
    }

    pub(crate) fn prepare_replacements(
        &self,
        bodies: &[(BodyId, BodyState)],
    ) -> Result<PreparedReplacements, PhysicsError> {
        if bodies.len() > MAX_MOTION_BODIES {
            return Err(PhysicsError::Capacity);
        }

        let Some((first_id, first_state)) = bodies.first().copied() else {
            return Err(PhysicsError::InvalidState);
        };
        let mut records = [BodyRecord {
            id: first_id,
            state: first_state,
        }; MAX_MOTION_BODIES];
        for (index, (id, state)) in bodies.iter().copied().enumerate().skip(1) {
            records[index] = BodyRecord { id, state };
        }
        records[..bodies.len()].sort_unstable_by_key(|record| raw_id(record.id));

        let mut indices = [0; MAX_MOTION_BODIES];
        for index in 0..bodies.len() {
            let record = records[index];
            if index > 0 && records[index - 1].id == record.id {
                return Err(PhysicsError::InvalidState);
            }

            let (slot_index, slot) = self.exact_slot(record.id)?;
            if !matches!(slot, BodySlot::Committed(_)) {
                return Err(PhysicsError::InvalidState);
            }
            indices[index] = slot_index;
        }

        Ok(PreparedReplacements {
            records,
            indices,
            len: bodies.len(),
        })
    }

    pub(crate) fn apply_replacements(&mut self, prepared: PreparedReplacements) {
        for index in 0..prepared.len {
            let slot_index = prepared.indices[index];
            self.replace_committed(slot_index, prepared.records[index]);
        }
    }

    pub(crate) fn prepare_retirement(
        &self,
        body_ids: &[BodyId],
    ) -> Result<PreparedRetirement, PhysicsError> {
        if body_ids.len() > MAX_MOTION_BODIES {
            return Err(PhysicsError::Capacity);
        }

        if body_ids.is_empty() {
            return Ok(PreparedRetirement {
                indices: [0; MAX_MOTION_BODIES],
                len: 0,
            });
        }

        let mut sorted_ids = [body_ids[0]; MAX_MOTION_BODIES];
        sorted_ids[..body_ids.len()].copy_from_slice(body_ids);
        sorted_ids[..body_ids.len()].sort_unstable_by_key(|id| raw_id(*id));

        let mut indices = [0; MAX_MOTION_BODIES];
        for index in 0..body_ids.len() {
            let id = sorted_ids[index];
            if index > 0 && sorted_ids[index - 1] == id {
                return Err(PhysicsError::InvalidState);
            }
            indices[index] = self.exact_slot(id)?.0;
        }

        Ok(PreparedRetirement {
            indices,
            len: body_ids.len(),
        })
    }

    pub(crate) fn apply_retirement(&mut self, prepared: PreparedRetirement) {
        for index in prepared.indices[..prepared.len].iter().rev() {
            self.remove_at(*index);
        }
    }
}

fn raw_id(id: BodyId) -> i64 {
    ObjectId::from(id).to_raw()
}
