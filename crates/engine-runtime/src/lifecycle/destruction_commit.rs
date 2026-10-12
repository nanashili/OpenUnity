//! Validated destruction planning and idempotent ownership removal.

use crate::{ObjectKind, Runtime, RuntimeError};
use engine_object::ObjectId;

/// Fully validated removal work. No callback or fallible hierarchy lookup occurs
/// between planning this batch and committing it.
#[derive(Debug)]
struct DestructionPlan {
    targets: Vec<PlannedDestruction>,
}

#[derive(Debug)]
enum PlannedDestruction {
    Script(ObjectId),
    Plain(ObjectId),
    Subtree {
        root_owner: ObjectId,
        root_transform: ObjectId,
        parent: Option<ObjectId>,
        nodes: Vec<DestructionNode>,
    },
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

    fn plan_subtree_destruction(
        &self,
        owner: ObjectId,
        root: ObjectId,
    ) -> Result<PlannedDestruction, RuntimeError> {
        let parent = self.transform(root)?.parent;
        if let Some(parent) = parent {
            self.transform(parent)?;
        }
        let mut nodes = Vec::new();
        let mut pending = vec![root];
        while let Some(transform) = pending.pop() {
            let data = self.transform(transform)?;
            self.owner_scripts(data.owner)?;
            pending.extend(data.children.iter().copied());
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
        Ok(PlannedDestruction::Subtree {
            root_owner: owner,
            root_transform: root,
            parent,
            nodes,
        })
    }

    fn plan_destruction(&self) -> Result<DestructionPlan, RuntimeError> {
        let mut targets = Vec::new();
        for target in &self.lifecycle.pending_destroy {
            if !self.objects.is_alive(target.id) {
                continue;
            }
            let planned = match self.objects.kind(target.id)? {
                ObjectKind::Script => PlannedDestruction::Script(target.id),
                ObjectKind::Plain => PlannedDestruction::Plain(target.id),
                ObjectKind::GameObject => {
                    let root = self.gameobject_transform(target.id)?;
                    self.plan_subtree_destruction(target.id, root)?
                }
                ObjectKind::Transform => return Err(RuntimeError::InvalidKind),
            };
            targets.push(planned);
        }
        Ok(DestructionPlan { targets })
    }

    fn commit_subtree_destruction(
        &mut self,
        root_owner: ObjectId,
        root_transform: ObjectId,
        parent: Option<ObjectId>,
        nodes: Vec<DestructionNode>,
    ) {
        // An earlier overlapping request may already have removed this subtree.
        if !self.objects.is_alive(root_owner) {
            return;
        }
        if let Some(parent) = parent {
            if let Some(data) = self.transforms.get_mut(&parent) {
                data.children.retain(|&child| child != root_transform);
            }
        }
        for node in nodes {
            for script in node.scripts {
                self.remove_script(script);
            }
            // Removal is idempotent for overlapping targets. All hierarchy
            // lookups were validated before this infallible commit begins.
            if self.objects.is_alive(node.owner) {
                self.objects
                    .remove_object(node.owner)
                    .expect("validated owner retirement");
            }
            drop(self.transforms.remove(&node.transform));
        }
    }

    fn commit_destruction(&mut self, plan: DestructionPlan) {
        for target in plan.targets {
            match target {
                PlannedDestruction::Script(id) => self.remove_script(id),
                PlannedDestruction::Plain(id) => {
                    if self.objects.is_alive(id) {
                        self.objects
                            .remove_object(id)
                            .expect("validated plain-object retirement");
                    }
                }
                PlannedDestruction::Subtree {
                    root_owner,
                    root_transform,
                    parent,
                    nodes,
                } => self.commit_subtree_destruction(root_owner, root_transform, parent, nodes),
            }
        }
        self.lifecycle.pending_destroy.clear();
    }

    pub(in crate::lifecycle) fn finalize_destruction(&mut self) -> Result<(), RuntimeError> {
        // Retain every pending request if any validation or traversal fails.
        // No callback runs between this complete plan and its infallible commit.
        let plan = self.plan_destruction()?;
        self.commit_destruction(plan);
        Ok(())
    }
}
