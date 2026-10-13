use engine_assets::{MeshAsset, RenderAssets, UnlitMaterial};

use super::arguments::Dimension;
use super::error::PhysicsSceneError;

const DYNAMIC_RGBA8: [u8; 4] = [224, 72, 48, 255];
const FLOOR_RGBA8: [u8; 4] = [88, 112, 136, 255];

/// Build a unit mesh and opaque materials for the selected physics profile.
pub(crate) fn create_assets(dimension: Dimension) -> Result<RenderAssets, PhysicsSceneError> {
    let mesh = match dimension {
        Dimension::TwoD => unit_quad()?,
        Dimension::ThreeD => unit_cube()?,
    };
    let materials = vec![
        UnlitMaterial::try_new(1, DYNAMIC_RGBA8)
            .map_err(|error| PhysicsSceneError::display("create box material", error))?,
        UnlitMaterial::try_new(2, FLOOR_RGBA8)
            .map_err(|error| PhysicsSceneError::display("create floor material", error))?,
    ];

    RenderAssets::try_new(vec![mesh], materials)
        .map_err(|error| PhysicsSceneError::display("validate physics scene assets", error))
}

fn unit_quad() -> Result<MeshAsset, PhysicsSceneError> {
    MeshAsset::try_new(
        1,
        vec![
            [-1.0, -1.0, 0.0],
            [1.0, -1.0, 0.0],
            [1.0, 1.0, 0.0],
            [-1.0, 1.0, 0.0],
        ],
        vec![0, 1, 2, 0, 2, 3],
    )
    .map_err(|error| PhysicsSceneError::display("create unit 2D mesh", error))
}

fn unit_cube() -> Result<MeshAsset, PhysicsSceneError> {
    MeshAsset::try_new(
        1,
        vec![
            [-1.0, -1.0, -1.0],
            [1.0, -1.0, -1.0],
            [1.0, 1.0, -1.0],
            [-1.0, 1.0, -1.0],
            [-1.0, -1.0, 1.0],
            [1.0, -1.0, 1.0],
            [1.0, 1.0, 1.0],
            [-1.0, 1.0, 1.0],
        ],
        vec![
            0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 3, 7, 6, 3, 6, 2, 0, 4, 7, 0, 7,
            3, 1, 2, 6, 1, 6, 5,
        ],
    )
    .map_err(|error| PhysicsSceneError::display("create unit 3D mesh", error))
}
