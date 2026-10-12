use crate::{SceneError, ScenePlan};
use std::collections::BTreeSet;

/// Validate the GameObject owners used by an `INFRA-RENDER-001` camera and draw list.
///
/// Camera and draw owners must be positive IDs of GameObjects in `plan`. Draw owners
/// must be unique; the camera may share its GameObject with a draw.
pub fn validate_render_binding_owners(
    plan: &ScenePlan,
    camera_owner: i64,
    draw_owners: &[i64],
) -> std::result::Result<(), SceneError> {
    plan.validate()?;

    let game_object_ids: BTreeSet<_> = plan
        .game_objects
        .iter()
        .map(|object| object.file_id)
        .collect();
    validate_game_object_owner(camera_owner, "camera", &game_object_ids)?;

    let mut bound_draw_owners = BTreeSet::new();
    for owner in draw_owners {
        validate_game_object_owner(*owner, "draw", &game_object_ids)?;

        if !bound_draw_owners.insert(*owner) {
            return Err(SceneError::new("multiple render draws for one GameObject"));
        }
    }

    Ok(())
}

fn validate_game_object_owner(
    owner: i64,
    binding_kind: &str,
    game_object_ids: &BTreeSet<i64>,
) -> std::result::Result<(), SceneError> {
    if owner <= 0 {
        return Err(SceneError::new(format!(
            "render {binding_kind} owner must be a positive GameObject ID"
        )));
    }

    if !game_object_ids.contains(&owner) {
        return Err(SceneError::new(format!(
            "render {binding_kind} owner is not a GameObject"
        )));
    }

    Ok(())
}
