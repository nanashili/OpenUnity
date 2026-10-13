//! Bounded offscreen color and depth targets for one WGPU frame.

const RGBA8_BYTES_PER_PIXEL: u64 = 4;
const COPY_BYTES_PER_ROW_ALIGNMENT: u64 = 256;

pub(crate) use engine_render_api::TargetSize;

pub(crate) fn copy_bytes_per_row(size: TargetSize) -> Result<u32, TargetError> {
    let unpadded = u64::from(size.width())
        .checked_mul(RGBA8_BYTES_PER_PIXEL)
        .ok_or(TargetError::SizeOverflow)?;
    let padded = unpadded
        .checked_add(COPY_BYTES_PER_ROW_ALIGNMENT - 1)
        .ok_or(TargetError::SizeOverflow)?
        / COPY_BYTES_PER_ROW_ALIGNMENT
        * COPY_BYTES_PER_ROW_ALIGNMENT;
    u32::try_from(padded).map_err(|_| TargetError::SizeOverflow)
}

pub(crate) fn packed_byte_len(size: TargetSize) -> Result<usize, TargetError> {
    let bytes = u64::from(size.width())
        .checked_mul(u64::from(size.height()))
        .and_then(|pixels| pixels.checked_mul(RGBA8_BYTES_PER_PIXEL))
        .ok_or(TargetError::SizeOverflow)?;
    usize::try_from(bytes).map_err(|_| TargetError::SizeOverflow)
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum TargetError {
    SizeOverflow,
}

pub(crate) struct FrameTargets {
    pub(crate) size: TargetSize,
    pub(crate) color: wgpu::Texture,
    pub(crate) color_view: wgpu::TextureView,
    pub(crate) _depth: wgpu::Texture,
    pub(crate) depth_view: wgpu::TextureView,
}

impl FrameTargets {
    pub(crate) fn create(device: &wgpu::Device, size: TargetSize) -> Result<Self, TargetError> {
        copy_bytes_per_row(size)?;
        packed_byte_len(size)?;

        let color = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("openunity-frame-color"),
            size: wgpu::Extent3d {
                width: size.width(),
                height: size.height(),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let color_view = color.create_view(&wgpu::TextureViewDescriptor::default());

        let depth = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("openunity-frame-depth"),
            size: wgpu::Extent3d {
                width: size.width(),
                height: size.height(),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());

        Ok(Self {
            size,
            color,
            color_view,
            _depth: depth,
            depth_view,
        })
    }
}
