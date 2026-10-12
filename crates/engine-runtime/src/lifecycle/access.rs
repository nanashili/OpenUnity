//! Validated script/owner lookup and scene lifecycle ownership.

use super::state::Script;
use crate::{ObjectKind, Runtime, RuntimeError};
use engine_object::ObjectId;

impl Runtime {
    pub(crate) fn lifecycle_pending_owner(&self, owner: ObjectId) -> bool {
        self.lifecycle
            .pending_destroy
            .iter()
            .any(|target| target.id == owner)
    }

    pub(crate) fn lifecycle_ready(&self) -> Result<(), RuntimeError> {
        if self.lifecycle.faulted {
            Err(RuntimeError::DispatchFaulted)
        } else {
            Ok(())
        }
    }

    pub(crate) fn input_binding_ready(&self) -> Result<(), RuntimeError> {
        self.lifecycle_ready()?;
        if !self.frame_idle() || !self.lifecycle.deliveries.is_empty() {
            return Err(RuntimeError::Lifecycle);
        }
        Ok(())
    }

    pub(crate) fn scene_lifecycle_ready(&self, healthy: bool) -> Result<(), RuntimeError> {
        if healthy {
            self.lifecycle_ready()?;
        }
        if (healthy && !self.frame_idle()) || !self.lifecycle.deliveries.is_empty() {
            return Err(RuntimeError::StagingBusy);
        }
        Ok(())
    }

    pub(crate) fn scene_release_lifecycle_ready(
        &self,
        scene_ids: &std::collections::BTreeSet<i64>,
    ) -> Result<(), RuntimeError> {
        self.scene_lifecycle_ready(false)?;
        if self
            .lifecycle
            .pending_destroy
            .iter()
            .any(|target| scene_ids.contains(&target.id.to_raw()))
        {
            return Err(RuntimeError::StagingBusy);
        }
        Ok(())
    }

    pub(crate) fn scene_snapshot_lifecycle_ready(&self) -> Result<(), RuntimeError> {
        self.lifecycle_ready()?;
        if !self.frame_idle()
            || !self.lifecycle.deliveries.is_empty()
            || !self.lifecycle.pending_destroy.is_empty()
        {
            return Err(RuntimeError::Lifecycle);
        }
        Ok(())
    }

    pub(crate) fn scene_owned_scripts(
        &self,
        owners: &std::collections::BTreeSet<i64>,
    ) -> Vec<ObjectId> {
        self.lifecycle
            .scripts
            .iter()
            .filter_map(|(&raw, script)| {
                owners
                    .contains(&script.owner.to_raw())
                    .then(|| ObjectId::from_raw(raw))
                    .flatten()
            })
            .collect()
    }

    pub(crate) fn scene_release_lifecycle(&mut self, ids: &std::collections::BTreeSet<i64>) {
        self.lifecycle
            .scripts
            .retain(|id, script| !ids.contains(id) && !ids.contains(&script.owner.to_raw()));
        let cursor = self.lifecycle.snapshot
            [..self.lifecycle.cursor.min(self.lifecycle.snapshot.len())]
            .iter()
            .filter(|id| !ids.contains(&id.to_raw()))
            .count();
        self.lifecycle
            .snapshot
            .retain(|id| !ids.contains(&id.to_raw()));
        self.lifecycle.cursor = cursor;
        self.lifecycle
            .pending_destroy
            .retain(|target| !ids.contains(&target.id.to_raw()));
    }

    pub(in crate::lifecycle) fn script(&self, id: ObjectId) -> Result<&Script, RuntimeError> {
        match self.objects.kind(id)? {
            ObjectKind::Script => self
                .lifecycle
                .scripts
                .get(&id.to_raw())
                .ok_or(RuntimeError::InvalidObject),
            _ => Err(RuntimeError::InvalidKind),
        }
    }

    pub(in crate::lifecycle) fn committed_script(
        &self,
        id: ObjectId,
    ) -> Result<&Script, RuntimeError> {
        let script = self.script(id)?;
        if script.committed {
            Ok(script)
        } else {
            Err(RuntimeError::Lifecycle)
        }
    }

    pub(in crate::lifecycle) fn owner_scripts(
        &self,
        owner: ObjectId,
    ) -> Result<&[ObjectId], RuntimeError> {
        self.objects.owner_scripts(owner).map_err(Into::into)
    }

    pub(crate) fn subtree_has_scripts(&self, root: ObjectId) -> Result<bool, RuntimeError> {
        let mut pending = vec![root];
        while let Some(transform) = pending.pop() {
            let data = self.transform(transform)?;
            if !self.owner_scripts(data.owner)?.is_empty()
                || self
                    .lifecycle
                    .scripts
                    .values()
                    .any(|script| script.owner == data.owner)
            {
                return Ok(true);
            }
            pending.extend(data.children.iter().copied());
        }
        Ok(false)
    }

    pub(crate) fn owner_pending_destroy(&self, owner: ObjectId) -> Result<bool, RuntimeError> {
        let mut current = Some(self.gameobject_transform(owner)?);
        while let Some(transform) = current {
            let data = self.transform(transform)?;
            if self
                .lifecycle
                .pending_destroy
                .iter()
                .any(|target| target.id == data.owner)
            {
                return Ok(true);
            }
            current = data.parent;
        }
        Ok(false)
    }
}
