//! Ordered callback eligibility and phase snapshot consumption.

use crate::{Runtime, RuntimeError};
use engine_object::ObjectId;

impl Runtime {
    pub(in crate::lifecycle) fn ordered_eligible(&self, started: bool) -> Vec<ObjectId> {
        let mut scripts: Vec<_> = self
            .lifecycle
            .scripts
            .iter()
            .filter_map(|(&raw, script)| {
                (script.committed
                    && script.awakened
                    && script.started == started
                    && script.notified_enabled
                    && script.enabled
                    && !script.pending_destroy
                    && script.in_flight == 0
                    && self.gameobject_active(script.owner, true).unwrap_or(false))
                .then_some((script.order, raw))
            })
            .collect();
        scripts.sort_unstable();
        scripts
            .into_iter()
            .filter_map(|(_, raw)| ObjectId::from_raw(raw))
            .collect()
    }

    pub(in crate::lifecycle) fn mutation_event(&self) -> Option<(ObjectId, i32)> {
        self.lifecycle
            .scripts
            .iter()
            .filter_map(|(&raw, script)| {
                if !script.committed || script.in_flight > 0 {
                    return None;
                }
                let owner_active = self.gameobject_active(script.owner, true).unwrap_or(false);
                let active = owner_active && script.enabled && !script.pending_destroy;
                let event = if script.notified_enabled && !active {
                    6
                } else if !script.pending_destroy && !script.awakened && owner_active {
                    1
                } else if script.awakened && !script.notified_enabled && active {
                    2
                } else {
                    return None;
                };
                Some((script.order, raw, event))
            })
            .min_by_key(|&(order, raw, _)| (order, raw))
            .and_then(|(_, raw, event)| Some((ObjectId::from_raw(raw)?, event)))
    }

    pub(in crate::lifecycle) fn phase_event(
        &mut self,
    ) -> Result<Option<(ObjectId, i32)>, RuntimeError> {
        match self.lifecycle.phase {
            1 | 4 => Ok(self
                .ordered_eligible(false)
                .first()
                .copied()
                .map(|id| (id, 3))),
            2 | 3 => Ok(self.snapshot_event(self.lifecycle.phase + 2)),
            5 => Ok(self.lifecycle.scripts.iter().find_map(|(&raw, script)| {
                (script.committed
                    && script.pending_destroy
                    && script.awakened
                    && !script.notified_destroy
                    && script.in_flight == 0)
                    .then(|| ObjectId::from_raw(raw).map(|id| (id, 7)))
                    .flatten()
            })),
            _ => Err(RuntimeError::Lifecycle),
        }
    }

    pub(in crate::lifecycle) fn snapshot_event(&mut self, event: i32) -> Option<(ObjectId, i32)> {
        while let Some(&id) = self.lifecycle.snapshot.get(self.lifecycle.cursor) {
            self.lifecycle.cursor += 1;
            if self.committed_script(id).is_ok_and(|script| {
                script.started && script.notified_enabled && script.in_flight == 0
            }) && self.behaviour_active(id).unwrap_or(false)
            {
                return Some((id, event));
            }
        }
        None
    }
}
