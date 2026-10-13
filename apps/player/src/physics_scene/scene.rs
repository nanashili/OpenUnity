use engine_math::Vector;
use engine_object::ObjectId;
use engine_runtime::{PhysicsSceneDimension, Runtime};
use serde::Serialize;

use super::arguments::Dimension;
use super::error::PhysicsSceneError;

const GRAVITY: f32 = -9.81;
const DEPTH: f32 = 3.0;

/// A body label used by the receipt and ordered renderer bindings.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum BodyRole {
    FixedFloor,
    DynamicBox,
}

/// Runtime object and render facts shared across simulation captures.
#[derive(Clone, Copy)]
pub(super) struct BodyDefinition {
    pub(super) role: BodyRole,
    pub(super) owner: ObjectId,
    pub(super) transform: ObjectId,
    pub(super) half_extents: Vector,
    pub(super) mesh_index: u32,
    pub(super) material_index: u32,
}

/// The live native runtime and its two admitted collision bodies.
pub(super) struct RuntimeScene {
    pub(super) runtime: Runtime,
    pub(super) dimension: Dimension,
    pub(super) floor: BodyDefinition,
    pub(super) dynamic: BodyDefinition,
}

impl RuntimeScene {
    pub(super) fn create(dimension: Dimension) -> Result<Self, PhysicsSceneError> {
        let mut runtime = Runtime::default();

        select_dimension(&mut runtime, dimension)?;
        let floor = create_body(&mut runtime, BodyRole::FixedFloor)?;
        let dynamic = create_body(&mut runtime, BodyRole::DynamicBox)?;

        admit_bodies(&mut runtime, dimension, floor, dynamic)?;
        validate_scene_selection(&runtime, dimension)?;

        Ok(Self {
            runtime,
            dimension,
            floor,
            dynamic,
        })
    }
}

fn select_dimension(runtime: &mut Runtime, dimension: Dimension) -> Result<(), PhysicsSceneError> {
    match dimension {
        Dimension::TwoD => runtime
            .physics_scene_create_2d([0.0, GRAVITY])
            .map_err(|error| PhysicsSceneError::debug("create native 2D collision scene", error)),
        Dimension::ThreeD => runtime
            .physics_scene_create_3d([0.0, GRAVITY, 0.0])
            .map_err(|error| PhysicsSceneError::debug("create native 3D collision scene", error)),
    }
}

fn create_body(runtime: &mut Runtime, role: BodyRole) -> Result<BodyDefinition, PhysicsSceneError> {
    let (name, position, half_extents, material_index) = body_specification(role);
    let owner = runtime
        .create_gameobject(name)
        .map_err(|error| PhysicsSceneError::debug("create physics scene GameObject", error))?;
    let transform = runtime
        .gameobject_transform(owner)
        .map_err(|error| PhysicsSceneError::debug("resolve physics scene Transform", error))?;

    write_transform(runtime, transform, 1, &position)?;
    write_transform(runtime, transform, 2, &[0.0, 0.0, 0.0, 1.0])?;
    write_transform(runtime, transform, 3, &[1.0, 1.0, 1.0])?;

    Ok(BodyDefinition {
        role,
        owner,
        transform,
        half_extents,
        mesh_index: 0,
        material_index,
    })
}

fn body_specification(role: BodyRole) -> (&'static str, Vector, Vector, u32) {
    match role {
        BodyRole::FixedFloor => (
            "Native Physics Floor",
            [0.0, -1.0, DEPTH],
            [4.0, 0.25, 0.5],
            1,
        ),
        BodyRole::DynamicBox => ("Native Dynamic Box", [0.0, 1.0, DEPTH], [0.5, 0.5, 0.5], 0),
    }
}

fn write_transform(
    runtime: &mut Runtime,
    transform: ObjectId,
    property: i32,
    values: &[f32],
) -> Result<(), PhysicsSceneError> {
    runtime
        .transform_write(transform, property, values)
        .map_err(|error| PhysicsSceneError::debug("author initial physics pose", error))
}

fn admit_bodies(
    runtime: &mut Runtime,
    dimension: Dimension,
    floor: BodyDefinition,
    dynamic: BodyDefinition,
) -> Result<(), PhysicsSceneError> {
    match dimension {
        Dimension::TwoD => {
            runtime
                .physics_scene_add_fixed_box_2d(
                    floor.owner,
                    [floor.half_extents[0], floor.half_extents[1]],
                )
                .map_err(|error| PhysicsSceneError::debug("admit fixed 2D floor", error))?;
            runtime
                .physics_scene_add_dynamic_box_2d(
                    dynamic.owner,
                    [dynamic.half_extents[0], dynamic.half_extents[1]],
                )
                .map_err(|error| PhysicsSceneError::debug("admit dynamic 2D box", error))?;
        }
        Dimension::ThreeD => {
            runtime
                .physics_scene_add_fixed_cuboid_3d(floor.owner, floor.half_extents)
                .map_err(|error| PhysicsSceneError::debug("admit fixed 3D floor", error))?;
            runtime
                .physics_scene_add_dynamic_cuboid_3d(dynamic.owner, dynamic.half_extents)
                .map_err(|error| PhysicsSceneError::debug("admit dynamic 3D box", error))?;
        }
    }

    if runtime.physics_scene_body_count() != 2 {
        return Err(PhysicsSceneError::new(
            "native collision scene did not retain its two admitted bodies",
        ));
    }

    Ok(())
}

fn validate_scene_selection(
    runtime: &Runtime,
    dimension: Dimension,
) -> Result<(), PhysicsSceneError> {
    let expected = match dimension {
        Dimension::TwoD => PhysicsSceneDimension::TwoD,
        Dimension::ThreeD => PhysicsSceneDimension::ThreeD,
    };
    if runtime.physics_scene_dimension() != Some(expected) {
        return Err(PhysicsSceneError::new(
            "native collision scene selected an unexpected dimension",
        ));
    }

    Ok(())
}
