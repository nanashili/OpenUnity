use super::{HierarchyError, RootOwner, SceneHierarchy};
use engine_identifiers::ObjectId;

impl SceneHierarchy {
    /// Return a Transform's index in its parent or scene-root sequence.
    /// Unowned roots have the stable compatibility index zero.
    pub fn sibling_index(&self, transform: ObjectId) -> Result<usize, HierarchyError> {
        match self.validated_parent(transform)? {
            Some(parent) => self
                .children
                .get(&parent)
                .and_then(|children| children.iter().position(|&child| child == transform))
                .ok_or(HierarchyError::MalformedTopology),
            None => match self.root_owners[&transform] {
                RootOwner::Unowned => Ok(0),
                RootOwner::Scene(scene) => self
                    .scene_roots
                    .get(&scene)
                    .and_then(|roots| roots.iter().position(|&root| root == transform))
                    .ok_or(HierarchyError::MalformedTopology),
            },
        }
    }

    /// Reorder a Transform within its existing direct parent's child sequence.
    pub fn set_sibling_index(
        &mut self,
        transform: ObjectId,
        index: usize,
    ) -> Result<(), HierarchyError> {
        let parent = self.validate_parented_group(transform)?;
        let children = self
            .children
            .get(&parent)
            .ok_or(HierarchyError::MalformedTopology)?;
        if index >= children.len() {
            return Err(HierarchyError::InvalidSiblingIndex {
                index,
                count: children.len(),
            });
        }
        let current_index = children
            .iter()
            .position(|&child| child == transform)
            .ok_or(HierarchyError::MalformedTopology)?;
        if current_index == index {
            return Ok(());
        }

        let committed_revision = self.next_revision()?;
        let children = self
            .children
            .get_mut(&parent)
            .ok_or(HierarchyError::MalformedTopology)?;
        let moved = children.remove(current_index);
        children.insert(index, moved);
        self.revision = committed_revision;
        Ok(())
    }
}
