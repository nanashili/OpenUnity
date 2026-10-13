//! Input validation for the bounded 2D collision world.

use engine_physics_api::CollisionError;

use crate::{
    MAX_ABSOLUTE_GRAVITY_COMPONENT, MAX_ABSOLUTE_POSITION_COMPONENT, MAX_CUBOID_HALF_EXTENT,
    MIN_CUBOID_HALF_EXTENT,
};

pub(crate) fn validate_gravity(gravity: [f32; 2]) -> Result<(), CollisionError> {
    if gravity
        .iter()
        .any(|component| !component.is_finite() || component.abs() > MAX_ABSOLUTE_GRAVITY_COMPONENT)
    {
        return Err(CollisionError::InvalidNumber);
    }

    Ok(())
}

pub(crate) fn validate_position(position: [f32; 2]) -> Result<(), CollisionError> {
    if position.iter().any(|component| {
        !component.is_finite() || component.abs() > MAX_ABSOLUTE_POSITION_COMPONENT
    }) {
        return Err(CollisionError::InvalidNumber);
    }

    Ok(())
}

pub(crate) fn validate_box(
    position: [f32; 2],
    rotation_radians: f32,
    half_extents: [f32; 2],
) -> Result<(), CollisionError> {
    validate_position(position)?;

    if !rotation_radians.is_finite() {
        return Err(CollisionError::InvalidNumber);
    }

    if half_extents.iter().any(|extent| {
        !extent.is_finite() || *extent < MIN_CUBOID_HALF_EXTENT || *extent > MAX_CUBOID_HALF_EXTENT
    }) {
        return Err(CollisionError::InvalidDimensions);
    }

    Ok(())
}
