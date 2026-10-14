//! Published scene ownership and root identity snapshots.

use super::Runtime;
use crate::RuntimeError;
use engine_object::ObjectId;
use engine_scene::{HierarchyError, RootOwner};

impl Runtime {
    /// Return the published scene owning a live GameObject, if any.
    ///
    /// Live unowned GameObjects have no scene and return `Ok(None)`.
    /// # Errors
    /// Rejects stale or wrong-kind objects and unavailable owner scenes.
    pub fn gameobject_scene(
        &self,
        game_object: ObjectId,
    ) -> Result<Option<ObjectId>, RuntimeError> {
        self.objects.gameobject_transform(game_object)?;

        let Some(scene) = self.scene_hierarchy_owner(game_object) else {
            return Ok(None);
        };
        self.scene_snapshot_scene_ready(scene)?;
        Ok(Some(scene))
    }

    /// Capture the complete ordered roots of one published scene.
    ///
    /// The returned pairs contain each root's GameObject and mandatory
    /// Transform identities. The vector owns its storage and observes one
    /// runtime state while the caller holds the runtime-store lock.
    /// # Errors
    /// Rejects unavailable scenes, malformed root groups, and allocation failure.
    pub fn scene_root_identities(
        &self,
        scene: ObjectId,
    ) -> Result<Vec<(ObjectId, ObjectId)>, RuntimeError> {
        self.scene_snapshot_scene_ready(scene)?;

        let roots = self
            .hierarchy
            .validated_scene_roots(scene)
            .map_err(map_root_validation_error)?;
        let mut identities = Vec::new();
        identities
            .try_reserve_exact(roots.len())
            .map_err(|_| RuntimeError::AllocationFailure)?;

        for &transform in roots {
            let data = self
                .transform(transform)
                .map_err(|_| RuntimeError::InvalidKind)?;
            let game_object = data.owner;
            self.validate_scene_root_identity(scene, game_object, transform)?;
            identities.push((game_object, transform));
        }

        Ok(identities)
    }

    fn validate_scene_root_identity(
        &self,
        scene: ObjectId,
        game_object: ObjectId,
        transform: ObjectId,
    ) -> Result<(), RuntimeError> {
        if !self.is_alive(game_object)
            || self.gameobject_transform(game_object)? != transform
            || self.hierarchy.parent_of(transform).is_some()
            || self.hierarchy.root_owner(transform) != Some(RootOwner::Scene(scene))
            || self.scene_hierarchy_owner(game_object) != Some(scene)
            || self.scene_hierarchy_owner(transform) != Some(scene)
        {
            return Err(RuntimeError::InvalidKind);
        }

        Ok(())
    }
}

fn map_root_validation_error(error: HierarchyError) -> RuntimeError {
    match error {
        HierarchyError::Allocation => RuntimeError::AllocationFailure,
        other => crate::hierarchy::map_hierarchy_error(other),
    }
}
