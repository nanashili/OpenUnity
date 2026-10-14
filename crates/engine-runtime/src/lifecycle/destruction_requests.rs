//! Deferred destruction requests and live-hierarchy reconciliation.

use super::state::DestroyTarget;
use crate::{ObjectKind, Runtime, RuntimeError};
use engine_object::ObjectId;
use std::collections::HashSet;

pub(crate) struct PreparedSceneDestroyRequests {
    targets: Vec<ObjectId>,
    scripts: Vec<ObjectId>,
}

impl Runtime {
    pub(crate) fn prepare_scene_destroy_requests(
        &mut self,
        owners: &[ObjectId],
    ) -> Result<PreparedSceneDestroyRequests, RuntimeError> {
        let mut targets = Vec::new();
        let mut scripts = HashSet::new();

        for &owner in owners {
            if !self.is_alive(owner) {
                continue;
            }
            if self.objects.kind(owner)? != ObjectKind::GameObject {
                return Err(RuntimeError::InvalidKind);
            }

            if !self
                .lifecycle
                .pending_destroy
                .iter()
                .any(|target| target.id == owner)
            {
                targets.push(owner);
            }

            scripts.extend(self.owner_scripts(owner)?.iter().copied());
        }

        for &script in &scripts {
            if !self.lifecycle.scripts.contains_key(&script.to_raw()) {
                return Err(RuntimeError::InvalidObject);
            }
        }

        self.lifecycle
            .pending_destroy
            .try_reserve(targets.len())
            .map_err(|_| RuntimeError::CountOverflow)?;

        Ok(PreparedSceneDestroyRequests {
            targets,
            scripts: scripts.into_iter().collect(),
        })
    }

    pub(crate) fn commit_scene_destroy_requests(&mut self, prepared: PreparedSceneDestroyRequests) {
        for script in prepared.scripts {
            let state = self
                .lifecycle
                .scripts
                .get_mut(&script.to_raw())
                .expect("scene destruction scripts remain live through commit");
            state.pending_destroy = true;
            state.enabled = false;
        }

        self.lifecycle
            .pending_destroy
            .extend(prepared.targets.into_iter().map(|id| DestroyTarget {
                id,
                prepared: false,
            }));
    }

    pub(crate) fn physics_destruction_requested(&self, id: ObjectId) -> Result<bool, RuntimeError> {
        if self.lifecycle_pending_owner(id) {
            return Ok(true);
        }

        let owner = match self.objects.kind(id)? {
            ObjectKind::GameObject => id,
            ObjectKind::Script | ObjectKind::Rigidbody => self.objects.component_gameobject(id)?,
            ObjectKind::Plain => return Ok(false),
            ObjectKind::Transform => return Err(RuntimeError::InvalidKind),
        };

        Ok(self.lifecycle_pending_owner(owner))
    }

    /// Queue owned destruction while preserving identities for callback queries.
    /// # Errors
    /// Rejects absent/wrong targets, unpublished reservations, and faulted dispatch.
    pub fn object_destroy_deferred(&mut self, id: ObjectId) -> Result<(), RuntimeError> {
        let mut scripts = Vec::new();
        match self.objects.kind(id)? {
            ObjectKind::Transform => return Err(RuntimeError::InvalidKind),
            ObjectKind::Script => {
                self.committed_script(id)?;
                scripts.push(id);
            }
            ObjectKind::Rigidbody => {
                if !self.objects.is_component_published(id)? {
                    return Err(RuntimeError::Lifecycle);
                }
            }
            ObjectKind::GameObject => {
                let mut pending = vec![self.gameobject_transform(id)?];
                while let Some(transform) = pending.pop() {
                    let data = self.transform(transform)?;
                    scripts.extend(self.owner_scripts(data.owner)?.iter().copied());
                    pending.extend(self.transform_children(transform)?.iter().copied());
                }
            }
            ObjectKind::Plain => {}
        }
        self.lifecycle_ready()?;
        for script in scripts {
            let script = self
                .lifecycle
                .scripts
                .get_mut(&script.to_raw())
                .ok_or(RuntimeError::InvalidObject)?;
            script.pending_destroy = true;
            script.enabled = false;
        }
        if !self
            .lifecycle
            .pending_destroy
            .iter()
            .any(|target| target.id == id)
        {
            self.lifecycle.pending_destroy.push(DestroyTarget {
                id,
                prepared: false,
            });
        }
        Ok(())
    }

    pub(in crate::lifecycle) fn prepare_destruction(&mut self) -> Result<(), RuntimeError> {
        let roots: Vec<_> = self
            .lifecycle
            .pending_destroy
            .iter()
            .filter(|target| !target.prepared)
            .map(|target| target.id)
            .collect();
        if roots.is_empty() {
            return Ok(());
        }

        // Requests suppress the scripts present at request time. Final ownership
        // follows the live hierarchy: escaped scripts survive, incoming scripts
        // are destroyed, and an explicitly requested component stays queued.
        // Resolve every membership before changing flags or owner activity.
        let membership: Vec<_> = self
            .lifecycle
            .scripts
            .iter()
            .map(|(&raw, script)| {
                let explicitly_requested = self
                    .lifecycle
                    .pending_destroy
                    .iter()
                    .any(|target| target.id.to_raw() == raw);
                let destroying =
                    explicitly_requested || self.owner_pending_destroy(script.owner)?;
                Ok((raw, destroying))
            })
            .collect::<Result<_, RuntimeError>>()?;

        for (raw, destroying) in membership {
            let script = self
                .lifecycle
                .scripts
                .get_mut(&raw)
                .ok_or(RuntimeError::InvalidObject)?;
            script.pending_destroy = destroying;
            // Do not change enabled here. Escaped scripts retain the request's
            // disabled value; newly included scripts retain their own value.
        }
        for root in roots {
            if self.objects.kind(root) == Ok(ObjectKind::GameObject) {
                self.objects.set_active_self(root, false)?;
            }
        }
        for target in &mut self.lifecycle.pending_destroy {
            target.prepared = true;
        }
        Ok(())
    }
}
