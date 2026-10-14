//! Transactional preparation and delivery eligibility for Transform callbacks.

use crate::{Runtime, RuntimeError};
use engine_object::ObjectId;

pub(crate) const CHILDREN_CHANGED: i32 = 9;
pub(crate) const PARENT_CHANGED: i32 = 10;

pub(crate) type Notification = (ObjectId, i32);

impl Runtime {
    pub(crate) fn prepare_reparent_notifications(
        &mut self,
        old_parent: Option<ObjectId>,
        new_parent: Option<ObjectId>,
        moved_root: ObjectId,
    ) -> Result<Vec<Notification>, RuntimeError> {
        let mut notifications = Vec::new();

        if let Some(parent) = old_parent {
            self.append_transform_notifications(parent, CHILDREN_CHANGED, &mut notifications)?;
        }
        if new_parent != old_parent {
            if let Some(parent) = new_parent {
                self.append_transform_notifications(parent, CHILDREN_CHANGED, &mut notifications)?;
            }
        }

        let parent_active = match new_parent {
            Some(parent) => {
                let owner = self.transform(parent)?.owner;
                self.gameobject_active(owner, true)?
            }
            None => true,
        };
        self.append_subtree_notifications(moved_root, parent_active, &mut notifications)?;
        self.reserve_hierarchy_notifications(notifications.len())?;
        Ok(notifications)
    }

    pub(crate) fn prepare_sibling_notifications(
        &mut self,
        parent: ObjectId,
    ) -> Result<Vec<Notification>, RuntimeError> {
        let mut notifications = Vec::new();
        self.append_transform_notifications(parent, CHILDREN_CHANGED, &mut notifications)?;
        self.reserve_hierarchy_notifications(notifications.len())?;
        Ok(notifications)
    }

    pub(crate) fn prepare_destroyed_child_notifications(
        &mut self,
        parents: &[ObjectId],
    ) -> Result<Vec<Notification>, RuntimeError> {
        let mut notifications = Vec::new();

        for &parent in parents {
            self.append_transform_notifications(parent, CHILDREN_CHANGED, &mut notifications)?;
        }

        self.reserve_hierarchy_notifications(notifications.len())?;
        Ok(notifications)
    }

    pub(crate) fn commit_hierarchy_notifications(&mut self, notifications: Vec<Notification>) {
        for (component, event) in notifications {
            self.lifecycle
                .hierarchy_notifications
                .push_back((component, event));
        }
    }

    pub(in crate::lifecycle) fn next_hierarchy_notification(&mut self) -> Option<Notification> {
        while let Some(notification) = self.lifecycle.hierarchy_notifications.pop_front() {
            if self.notification_eligible(notification.0) {
                return Some(notification);
            }
        }
        None
    }

    fn append_owner_notifications(
        &self,
        owner: ObjectId,
        owner_active: bool,
        event: i32,
        notifications: &mut Vec<Notification>,
    ) -> Result<(), RuntimeError> {
        let scripts = self.owner_scripts(owner)?;
        notifications
            .try_reserve(scripts.len())
            .map_err(|_| RuntimeError::CountOverflow)?;

        for &component in scripts {
            let Some(script) = self.lifecycle.scripts.get(&component.to_raw()) else {
                continue;
            };
            if owner_active
                && script.committed
                && script.awakened
                && script.enabled
                && !script.pending_destroy
            {
                notifications.push((component, event));
            }
        }

        Ok(())
    }

    fn append_transform_notifications(
        &self,
        transform: ObjectId,
        event: i32,
        notifications: &mut Vec<Notification>,
    ) -> Result<(), RuntimeError> {
        let owner = self.transform(transform)?.owner;
        let active = self.gameobject_active(owner, true)?;
        self.append_owner_notifications(owner, active, event, notifications)
    }

    fn append_subtree_notifications(
        &self,
        root: ObjectId,
        parent_active: bool,
        notifications: &mut Vec<Notification>,
    ) -> Result<(), RuntimeError> {
        let mut pending = Vec::new();
        pending
            .try_reserve(1)
            .map_err(|_| RuntimeError::CountOverflow)?;
        pending.push((root, parent_active));

        while let Some((transform, ancestor_active)) = pending.pop() {
            let owner = self.transform(transform)?.owner;
            let active = ancestor_active && self.objects.active_self(owner)?;
            self.append_owner_notifications(owner, active, PARENT_CHANGED, notifications)?;

            let children = self.transform_children(transform)?;
            pending
                .try_reserve(children.len())
                .map_err(|_| RuntimeError::CountOverflow)?;
            pending.extend(children.iter().rev().copied().map(|child| (child, active)));
        }

        Ok(())
    }

    fn reserve_hierarchy_notifications(&mut self, count: usize) -> Result<(), RuntimeError> {
        self.lifecycle
            .hierarchy_notifications
            .try_reserve(count)
            .map_err(|_| RuntimeError::CountOverflow)
    }

    fn notification_eligible(&self, component: ObjectId) -> bool {
        let Some(script) = self.lifecycle.scripts.get(&component.to_raw()) else {
            return false;
        };
        script.committed
            && script.awakened
            && script.enabled
            && !script.pending_destroy
            && self.is_alive(component)
            && self.gameobject_active(script.owner, true).unwrap_or(false)
    }
}
