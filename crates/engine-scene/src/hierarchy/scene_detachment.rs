use super::unique::ensure_unique;
use super::{HierarchyError, HierarchyTarget, PreparedHierarchyChange, RootOwner, SceneHierarchy};
use engine_identifiers::ObjectId;

impl SceneHierarchy {
    /// Prepare detaching scene-owned Transforms into the same scene root group.
    /// Newly promoted roots follow the supplied traversal order.
    pub fn prepare_scene_detachment(
        &self,
        scene: ObjectId,
        transforms_in_promotion_order: &[ObjectId],
    ) -> Result<PreparedHierarchyChange, HierarchyError> {
        self.validate_all()?;
        if !self.scene_roots.contains_key(&scene) {
            return Err(HierarchyError::MissingNode);
        }
        ensure_unique(transforms_in_promotion_order)?;

        let mut candidate = self.try_clone_graph()?;
        for &transform in transforms_in_promotion_order {
            if candidate.root_owner_of_node(transform)? != RootOwner::Scene(scene) {
                return Err(HierarchyError::MalformedTopology);
            }
            candidate.reparent(transform, HierarchyTarget::Root(RootOwner::Scene(scene)))?;
        }
        candidate.validate_all()?;
        Ok(PreparedHierarchyChange {
            base_revision: self.revision,
            candidate,
        })
    }
}
