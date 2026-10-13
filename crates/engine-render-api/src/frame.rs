//! Renderer-neutral frame values and checked asset references.
//!
//! The production owner is `crates/engine-render-api/src/frame.rs`.  This
//! API deliberately depends on validated values from the math and
//! asset owners; it does not parse assets or perform projection work.

use engine_assets::{MeshAsset, RenderAssets, UnlitMaterial};
use engine_math::Affine3x4;

use crate::{camera::OrthographicCamera, error::RenderError};

/// The largest target dimension admitted by the first offscreen slice.
pub const MAX_TARGET_DIMENSION: u32 = 2_048;
/// The largest ordered draw list admitted by the first offscreen slice.
pub const MAX_DRAW_INSTANCES: usize = 4_096;

/// A bounded, nonzero output target size.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TargetSize {
    width: u32,
    height: u32,
}

impl TargetSize {
    /// Validate a target whose dimensions fit the first offscreen slice.
    pub fn try_new(width: u32, height: u32) -> Result<Self, RenderError> {
        if !(1..=MAX_TARGET_DIMENSION).contains(&width)
            || !(1..=MAX_TARGET_DIMENSION).contains(&height)
        {
            return Err(RenderError::InvalidTargetSize { width, height });
        }

        let pixels = usize::try_from(width)
            .ok()
            .and_then(|width| {
                usize::try_from(height)
                    .ok()
                    .and_then(|height| width.checked_mul(height))
            })
            .ok_or(RenderError::TargetAllocationOverflow)?;
        pixels
            .checked_mul(4)
            .ok_or(RenderError::TargetAllocationOverflow)?;

        Ok(Self { width, height })
    }

    /// Return the width in pixels.
    pub const fn width(self) -> u32 {
        self.width
    }

    /// Return the height in pixels.
    pub const fn height(self) -> u32 {
        self.height
    }

    /// Return the exact tightly packed RGBA byte count.
    pub fn rgba8_byte_len(self) -> usize {
        let width = usize::try_from(self.width).expect("validated target width fits usize");
        let height = usize::try_from(self.height).expect("validated target height fits usize");
        width
            .checked_mul(height)
            .and_then(|pixels| pixels.checked_mul(4))
            .expect("validated target byte count fits usize")
    }
}

/// One ordered indexed draw against validated asset arrays.
#[derive(Clone, Copy, Debug)]
pub struct DrawInstance {
    mesh_index: u32,
    material_index: u32,
    world_from_local: Affine3x4,
    enabled: bool,
}

impl DrawInstance {
    /// Validate asset-array indices and retain the complete affine transform.
    ///
    /// `Affine3x4` is a validated engine-math value, so constructing it through
    /// its owner is the finite-matrix admission point.  This API preserves
    /// shear, negative scale, and singular forward transforms.
    pub fn try_new(
        mesh_index: u32,
        material_index: u32,
        world_from_local: Affine3x4,
        enabled: bool,
        assets: &RenderAssets,
    ) -> Result<Self, RenderError> {
        let meshes = assets.meshes();
        let materials = assets.materials();
        if usize::try_from(mesh_index)
            .ok()
            .is_none_or(|index| index >= meshes.len())
        {
            return Err(RenderError::InvalidMeshReference { index: mesh_index });
        }
        if usize::try_from(material_index)
            .ok()
            .is_none_or(|index| index >= materials.len())
        {
            return Err(RenderError::InvalidMaterialReference {
                index: material_index,
            });
        }

        Ok(Self {
            mesh_index,
            material_index,
            world_from_local,
            enabled,
        })
    }

    /// Return the zero-based validated mesh-array index.
    pub const fn mesh_index(self) -> u32 {
        self.mesh_index
    }

    /// Return the zero-based validated material-array index.
    pub const fn material_index(self) -> u32 {
        self.material_index
    }

    /// Return the complete model affine transform.
    pub const fn world_from_local(self) -> Affine3x4 {
        self.world_from_local
    }

    /// Return whether the binding contributes an indexed draw.
    pub const fn enabled(self) -> bool {
        self.enabled
    }
}

/// A synchronous render input borrowing immutable validated assets.
pub struct RenderFrame<'assets> {
    frame_number: u64,
    target_size: TargetSize,
    camera: OrthographicCamera,
    assets: &'assets RenderAssets,
    draws: Box<[DrawInstance]>,
}

impl<'assets> RenderFrame<'assets> {
    /// Validate frame bounds and every draw reference before backend submission.
    pub fn try_new(
        frame_number: u64,
        target_size: TargetSize,
        camera: OrthographicCamera,
        assets: &'assets RenderAssets,
        draws: impl IntoIterator<Item = DrawInstance>,
    ) -> Result<Self, RenderError> {
        let mut checked_draws = Vec::new();
        for draw in draws {
            if checked_draws.len() == MAX_DRAW_INSTANCES {
                return Err(RenderError::DrawLimitExceeded {
                    count: MAX_DRAW_INSTANCES + 1,
                });
            }
            checked_draws.push(draw);
        }
        let draws: Box<[DrawInstance]> = checked_draws.into_boxed_slice();
        for draw in &draws {
            validate_draw_reference(draw, assets)?;
        }

        Ok(Self {
            frame_number,
            target_size,
            camera,
            assets,
            draws,
        })
    }

    /// Return the native lifecycle frame number, including zero after init.
    pub const fn frame_number(&self) -> u64 {
        self.frame_number
    }

    /// Return the checked output target.
    pub const fn target_size(&self) -> TargetSize {
        self.target_size
    }

    /// Return the camera's validated clear color in RGBA8 order.
    pub const fn clear_rgba8(&self) -> [u8; 4] {
        self.camera.clear_rgba8()
    }

    /// Borrow the validated orthographic camera.
    pub const fn camera(&self) -> &OrthographicCamera {
        &self.camera
    }

    /// Borrow mesh assets without transferring ownership across the backend seam.
    pub fn meshes(&self) -> &'assets [MeshAsset] {
        self.assets.meshes()
    }

    /// Borrow material assets without transferring ownership across the backend seam.
    pub fn materials(&self) -> &'assets [UnlitMaterial] {
        self.assets.materials()
    }

    /// Iterate draws in manifest order, preserving equal-depth last-wins order.
    pub fn draws(&self) -> impl ExactSizeIterator<Item = &DrawInstance> {
        self.draws.iter()
    }
}

fn validate_draw_reference(draw: &DrawInstance, assets: &RenderAssets) -> Result<(), RenderError> {
    let meshes = assets.meshes();
    let materials = assets.materials();
    if usize::try_from(draw.mesh_index)
        .ok()
        .is_none_or(|index| index >= meshes.len())
    {
        return Err(RenderError::InvalidMeshReference {
            index: draw.mesh_index,
        });
    }
    if usize::try_from(draw.material_index)
        .ok()
        .is_none_or(|index| index >= materials.len())
    {
        return Err(RenderError::InvalidMaterialReference {
            index: draw.material_index,
        });
    }
    Ok(())
}
