//! Per-frame validation and per-draw indexed geometry preparation.

use wgpu::util::DeviceExt;

use engine_assets::{MeshAsset, UnlitMaterial};
use engine_render_api::{DrawInstance, OrthographicCamera, RenderError, TargetSize};

use crate::backend_mapping::map_device_error;
use crate::device::DeviceOwner;
use crate::pipeline::{PreparedVertex, validate_prepared_vertices};

const MAX_VERTICES_PER_MESH: usize = 65_536;
const MAX_INDICES_PER_MESH: usize = 196_608;

/// Stateless geometry validator and uploader.
#[derive(Default)]
pub(crate) struct DrawPreparer;

/// Aggregate facts established by the allocation-free validation pass.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ValidatedDraws {
    enabled_count: usize,
    triangle_count: usize,
}

impl ValidatedDraws {
    /// Return the number of enabled draws that will be submitted.
    pub(crate) const fn enabled_count(self) -> usize {
        self.enabled_count
    }

    /// Return the aggregate indexed triangle count for the frame.
    pub(crate) const fn triangle_count(self) -> usize {
        self.triangle_count
    }
}

/// GPU buffers for exactly one ordered draw.
pub(crate) struct PreparedDraw {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
}

impl DrawPreparer {
    /// Create a preparer with no cross-frame GPU state.
    pub(crate) const fn new() -> Self {
        Self
    }

    /// Validate every enabled draw and transformed vertex before allocation.
    pub(crate) fn prevalidate<'draws>(
        &self,
        camera: &OrthographicCamera,
        target: TargetSize,
        meshes: &[MeshAsset],
        materials: &[UnlitMaterial],
        ordered_draws: impl ExactSizeIterator<Item = &'draws DrawInstance>,
    ) -> Result<ValidatedDraws, RenderError> {
        let mut enabled_count = 0usize;
        let mut triangle_count = 0usize;

        for draw in ordered_draws {
            if !draw.enabled() {
                continue;
            }

            let (mesh, material) = resolve_assets(draw, meshes, materials)?;
            validate_mesh_bounds(mesh)?;
            enabled_count = enabled_count
                .checked_add(1)
                .ok_or(RenderError::Allocation)?;
            triangle_count = triangle_count
                .checked_add(mesh.indices().len() / 3)
                .ok_or(RenderError::Allocation)?;
            let material_rgba8 = material.rgba8();
            for position in mesh.positions() {
                let clip = camera
                    .project_model_point(&draw.world_from_local(), *position, target)
                    .map_err(|_| RenderError::Numeric)?;
                let vertex = PreparedVertex {
                    clip_position: clip,
                    rgba8: material_rgba8,
                };
                validate_prepared_vertices(std::slice::from_ref(&vertex))
                    .map_err(|_| RenderError::Numeric)?;
            }
        }

        Ok(ValidatedDraws {
            enabled_count,
            triangle_count,
        })
    }

    /// Project and upload one already prevalidated enabled draw.
    pub(crate) fn prepare(
        &self,
        device: &DeviceOwner,
        camera: &OrthographicCamera,
        target: TargetSize,
        meshes: &[MeshAsset],
        materials: &[UnlitMaterial],
        draw: &DrawInstance,
    ) -> Result<PreparedDraw, RenderError> {
        let (mesh, material) = resolve_assets(draw, meshes, materials)?;
        validate_mesh_bounds(mesh)?;

        let vertices = mesh
            .positions()
            .iter()
            .map(|position| {
                let clip = camera
                    .project_model_point(&draw.world_from_local(), *position, target)
                    .map_err(|_| RenderError::Numeric)?;
                Ok(PreparedVertex {
                    clip_position: clip,
                    rgba8: material.rgba8(),
                })
            })
            .collect::<Result<Vec<_>, RenderError>>()?;
        validate_prepared_vertices(&vertices).map_err(|_| RenderError::Numeric)?;

        let index_count =
            u32::try_from(mesh.indices().len()).map_err(|_| RenderError::Allocation)?;
        let (vertex_buffer, index_buffer) = device
            .with_error_scopes(|gpu| {
                let vertex_buffer = gpu.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("openunity-draw-vertices"),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                let index_buffer = gpu.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("openunity-draw-indices"),
                    contents: bytemuck::cast_slice(mesh.indices()),
                    usage: wgpu::BufferUsages::INDEX,
                });
                Ok((vertex_buffer, index_buffer))
            })
            .map_err(map_device_error)?;

        Ok(PreparedDraw {
            vertex_buffer,
            index_buffer,
            index_count,
        })
    }
}

impl PreparedDraw {
    /// Borrow this draw for the submission owner while retaining its buffers.
    pub(crate) fn as_submission_draw(&self) -> crate::submission::PreparedDraw<'_> {
        crate::submission::PreparedDraw {
            vertex_buffer: &self.vertex_buffer,
            index_buffer: &self.index_buffer,
            index_count: self.index_count,
        }
    }
}

fn resolve_assets<'assets>(
    draw: &DrawInstance,
    meshes: &'assets [MeshAsset],
    materials: &'assets [UnlitMaterial],
) -> Result<(&'assets MeshAsset, &'assets UnlitMaterial), RenderError> {
    let mesh = meshes
        .get(
            usize::try_from(draw.mesh_index()).map_err(|_| RenderError::InvalidMeshReference {
                index: draw.mesh_index(),
            })?,
        )
        .ok_or(RenderError::InvalidMeshReference {
            index: draw.mesh_index(),
        })?;
    let material = materials
        .get(usize::try_from(draw.material_index()).map_err(|_| {
            RenderError::InvalidMaterialReference {
                index: draw.material_index(),
            }
        })?)
        .ok_or(RenderError::InvalidMaterialReference {
            index: draw.material_index(),
        })?;
    Ok((mesh, material))
}

fn validate_mesh_bounds(mesh: &MeshAsset) -> Result<(), RenderError> {
    if mesh.positions().len() > MAX_VERTICES_PER_MESH
        || mesh.indices().len() > MAX_INDICES_PER_MESH
        || mesh.indices().is_empty()
        || !mesh.indices().len().is_multiple_of(3)
    {
        return Err(RenderError::Allocation);
    }
    Ok(())
}
