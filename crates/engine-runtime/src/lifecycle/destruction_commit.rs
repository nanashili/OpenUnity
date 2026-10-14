//! Validated destruction planning and idempotent ownership removal.

use super::hierarchy_notifications::Notification;
use crate::{ObjectKind, Runtime, RuntimeError};
use engine_object::ObjectId;
use std::collections::HashSet;

/// Fully validated removal work. No callback or fallible hierarchy lookup occurs
/// between planning this batch and committing it.
#[derive(Debug)]
struct DestructionPlan {
    targets: Vec<PlannedDestruction>,
    retirement_ids: Vec<ObjectId>,
    hierarchy: Option<engine_scene::PreparedHierarchyChange>,
    hierarchy_notifications: Vec<Notification>,
}

#[derive(Debug)]
enum PlannedDestruction {
    Script(ObjectId),
    Plain(ObjectId),
    Rigidbody(ObjectId),
    Subtree { nodes: Vec<DestructionNode> },
}

#[derive(Debug)]
struct DestructionNode {
    transform: ObjectId,
    owner: ObjectId,
    scripts: Vec<ObjectId>,
}

impl Runtime {
    fn remove_script(&mut self, id: ObjectId) {
        self.lifecycle.scripts.remove(&id.to_raw());
        // An overlapping queued target may have already retired this identity.
        if self.objects.is_alive(id) {
            self.objects
                .remove_object(id)
                .expect("validated script retirement");
        }
    }

    fn plan_subtree_destruction(&self, root: ObjectId) -> Result<PlannedDestruction, RuntimeError> {
        let parent = self.transform_parent_id(root)?;
        if let Some(parent) = parent {
            self.transform(parent)?;
        }
        let mut nodes = Vec::new();
        let mut pending = vec![root];
        while let Some(transform) = pending.pop() {
            let data = self.transform(transform)?;
            self.owner_scripts(data.owner)?;
            pending.extend(self.transform_children(transform)?.iter().copied());
            // Include unpublished reservations without exposing their identities.
            let scripts = self
                .lifecycle
                .scripts
                .iter()
                .filter_map(|(&raw, script)| {
                    (script.owner == data.owner)
                        .then(|| ObjectId::from_raw(raw))
                        .flatten()
                })
                .collect();
            nodes.push(DestructionNode {
                transform,
                owner: data.owner,
                scripts,
            });
        }
        Ok(PlannedDestruction::Subtree { nodes })
    }

    fn plan_destruction(&mut self) -> Result<DestructionPlan, RuntimeError> {
        let mut targets = Vec::new();
        let mut retirement_ids = Vec::new();
        let mut transforms = HashSet::new();
        for target in &self.lifecycle.pending_destroy {
            if !self.objects.is_alive(target.id) {
                continue;
            }
            let planned = match self.objects.kind(target.id)? {
                ObjectKind::Script => PlannedDestruction::Script(target.id),
                ObjectKind::Plain => PlannedDestruction::Plain(target.id),
                ObjectKind::Rigidbody => PlannedDestruction::Rigidbody(target.id),
                ObjectKind::GameObject => {
                    let root = self.gameobject_transform(target.id)?;
                    self.plan_subtree_destruction(root)?
                }
                ObjectKind::Transform => return Err(RuntimeError::InvalidKind),
            };
            match &planned {
                PlannedDestruction::Script(id)
                | PlannedDestruction::Plain(id)
                | PlannedDestruction::Rigidbody(id) => {
                    retirement_ids.push(*id);
                }
                PlannedDestruction::Subtree { nodes } => {
                    retirement_ids.extend(nodes.iter().map(|node| node.owner));
                    transforms.extend(nodes.iter().map(|node| node.transform));
                }
            }
            targets.push(planned);
        }
        let parents = self.surviving_destruction_parents(&targets, &transforms)?;
        let hierarchy_notifications = self.prepare_destroyed_child_notifications(&parents)?;
        let transforms: Vec<_> = transforms.into_iter().collect();
        let hierarchy = if transforms.is_empty() {
            None
        } else {
            Some(
                self.hierarchy
                    .prepare_removal(&transforms)
                    .map_err(crate::hierarchy::map_hierarchy_error)?,
            )
        };

        Ok(DestructionPlan {
            targets,
            retirement_ids,
            hierarchy,
            hierarchy_notifications,
        })
    }

    fn surviving_destruction_parents(
        &self,
        targets: &[PlannedDestruction],
        removed_transforms: &HashSet<ObjectId>,
    ) -> Result<Vec<ObjectId>, RuntimeError> {
        let mut parents = Vec::new();
        let mut seen = HashSet::new();
        parents
            .try_reserve(targets.len())
            .map_err(|_| RuntimeError::CountOverflow)?;
        seen.try_reserve(targets.len())
            .map_err(|_| RuntimeError::CountOverflow)?;

        for target in targets {
            let PlannedDestruction::Subtree { nodes } = target else {
                continue;
            };
            let Some(root) = nodes.first() else {
                continue;
            };
            let Some(parent) = self.transform_parent_id(root.transform)? else {
                continue;
            };

            if removed_transforms.contains(&parent) || !seen.insert(parent) {
                continue;
            }
            parents.push(parent);
        }

        Ok(parents)
    }

    fn commit_subtree_destruction(&mut self, nodes: Vec<DestructionNode>) {
        for node in nodes {
            if self.transforms.remove(&node.transform).is_none() {
                // An overlapping queued subtree may already have retired this node.
                continue;
            }

            for script in node.scripts {
                self.remove_script(script);
            }
        }
    }

    fn commit_destruction(&mut self, targets: Vec<PlannedDestruction>) {
        for target in targets {
            match target {
                PlannedDestruction::Script(id) => self.remove_script(id),
                PlannedDestruction::Plain(id) | PlannedDestruction::Rigidbody(id) => {
                    if self.objects.is_alive(id) {
                        self.objects
                            .remove_object(id)
                            .expect("validated plain-object retirement");
                    }
                }
                PlannedDestruction::Subtree { nodes } => self.commit_subtree_destruction(nodes),
            }
        }
        self.lifecycle.pending_destroy.clear();
    }

    pub(in crate::lifecycle) fn finalize_destruction(&mut self) -> Result<(), RuntimeError> {
        // Retain every pending request if any validation or traversal fails.
        // No callback runs between this complete plan and its infallible commit.
        let plan = self.plan_destruction()?;
        let DestructionPlan {
            targets,
            retirement_ids,
            hierarchy,
            hierarchy_notifications,
        } = plan;
        self.physics_retire_objects(&retirement_ids)?;
        if let Some(hierarchy) = hierarchy {
            self.hierarchy
                .commit_change(hierarchy)
                .expect("prepared hierarchy removal remains current during exclusive retirement");
        }
        self.commit_destruction(targets);
        self.commit_hierarchy_notifications(hierarchy_notifications);
        Ok(())
    }
}
