//! Candidate camera and affine conversion contract for `engine-render-api`.

use engine_math::{Affine3x4, MathError, Vector};

use crate::{RenderError, TargetSize};

/// Immutable, validated orthographic camera state.
#[derive(Clone, Copy, Debug)]
pub struct OrthographicCamera {
    world_from_camera: Affine3x4,
    camera_from_world: Affine3x4,
    half_height: f32,
    near: f32,
    far: f32,
    clear_rgba8: [u8; 4],
}

impl OrthographicCamera {
    /// Validate a camera using the caller's unit-scale ancestor-chain result.
    ///
    /// The caller must establish this flag from the scene snapshot or a complete
    /// static hierarchy. A world affine alone cannot establish ancestor scales.
    pub fn try_new_with_scale_chain(
        world_from_camera: Affine3x4,
        unit_scale_chain: bool,
        half_height: f32,
        near: f32,
        far: f32,
        clear_rgba8: [u8; 4],
    ) -> Result<Self, RenderError> {
        if !unit_scale_chain {
            return Err(RenderError::InvalidCamera);
        }
        if clear_rgba8[3] != u8::MAX {
            return Err(RenderError::InvalidCamera);
        }
        validate_clip_range(half_height, near, far)?;

        let camera_from_world = world_from_camera.inverse().map_err(map_math_error)?;
        Ok(Self {
            world_from_camera,
            camera_from_world,
            half_height,
            near,
            far,
            clear_rgba8,
        })
    }

    /// Return the clear color in the target's RGBA8 channel order.
    pub const fn clear_rgba8(&self) -> [u8; 4] {
        self.clear_rgba8
    }

    /// Borrow the validated world-space camera affine.
    pub fn world_from_camera(&self) -> &Affine3x4 {
        &self.world_from_camera
    }

    /// Convert a world-space point into homogeneous orthographic clip space.
    pub fn project_world_point(
        &self,
        world_point: Vector,
        target: TargetSize,
    ) -> Result<[f32; 4], RenderError> {
        let camera_point = self
            .camera_from_world
            .transform_point(world_point)
            .map_err(map_math_error)?;
        self.project_camera_point(camera_point, target)
    }

    /// Project a local point through the complete model affine.
    pub fn project_model_point(
        &self,
        model_from_local: &Affine3x4,
        local_point: Vector,
        target: TargetSize,
    ) -> Result<[f32; 4], RenderError> {
        let world_point = model_from_local
            .transform_point(local_point)
            .map_err(map_math_error)?;
        self.project_world_point(world_point, target)
    }

    fn project_camera_point(
        &self,
        camera_point: Vector,
        target: TargetSize,
    ) -> Result<[f32; 4], RenderError> {
        let aspect = target.width() as f32 / target.height() as f32;
        finite(aspect)?;

        let horizontal_half_extent = self.half_height * aspect;
        finite(horizontal_half_extent)?;

        let depth_range = self.far - self.near;
        finite(depth_range)?;

        let clip = [
            camera_point[0] / horizontal_half_extent,
            camera_point[1] / self.half_height,
            (camera_point[2] - self.near) / depth_range,
            1.0,
        ];
        finite_slice(&clip)?;
        Ok(clip)
    }
}

/// Decode row-major snapshot coefficients without dropping shear or translation.
pub fn affine_from_snapshot_rows(rows: [[f32; 4]; 3]) -> Result<Affine3x4, RenderError> {
    Affine3x4::from_rows(rows).map_err(map_math_error)
}

/// Apply a complete model affine without requesting a model inverse.
pub fn transform_model_point(
    model_from_local: &Affine3x4,
    local_point: Vector,
) -> Result<Vector, RenderError> {
    model_from_local
        .transform_point(local_point)
        .map_err(map_math_error)
}

fn validate_clip_range(half_height: f32, near: f32, far: f32) -> Result<(), RenderError> {
    finite_slice(&[half_height, near, far])?;
    if half_height <= 0.0 || near <= 0.0 || near >= far {
        return Err(RenderError::InvalidCamera);
    }
    Ok(())
}

fn finite(value: f32) -> Result<f32, RenderError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(RenderError::Numeric)
    }
}

fn finite_slice(values: &[f32]) -> Result<(), RenderError> {
    for value in values {
        finite(*value)?;
    }

    Ok(())
}

fn map_math_error(error: MathError) -> RenderError {
    match error {
        MathError::Numeric => RenderError::Numeric,
        MathError::Unsupported => RenderError::InvalidCamera,
    }
}
