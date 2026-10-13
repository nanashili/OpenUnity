//! Native presentation surface configuration, resize, and frame acquisition.

use engine_render_api::{RenderError, TargetSize};

use crate::backend_mapping::map_device_error;
use crate::device::DeviceOwner;

pub(crate) struct SurfaceState {
    surface: wgpu::Surface<'static>,
    configuration: wgpu::SurfaceConfiguration,
    size: TargetSize,
    reconfigure_before_next_frame: bool,
}

impl SurfaceState {
    pub(crate) fn create(
        surface: wgpu::Surface<'static>,
        device: &DeviceOwner,
        size: TargetSize,
    ) -> Result<Self, RenderError> {
        let configuration = surface_configuration(&surface, device.adapter(), size)?;
        configure_surface(&surface, device, &configuration)?;

        Ok(Self {
            surface,
            configuration,
            size,
            reconfigure_before_next_frame: false,
        })
    }

    pub(crate) fn size(&self) -> TargetSize {
        self.size
    }

    pub(crate) fn format(&self) -> wgpu::TextureFormat {
        self.configuration.format
    }

    pub(crate) fn resize(
        &mut self,
        device: &DeviceOwner,
        size: TargetSize,
    ) -> Result<(), RenderError> {
        if self.size == size {
            return Ok(());
        }

        let mut configuration = self.configuration.clone();
        configuration.width = size.width();
        configuration.height = size.height();
        configure_surface(&self.surface, device, &configuration)?;

        self.configuration = configuration;
        self.size = size;
        self.reconfigure_before_next_frame = false;
        Ok(())
    }

    pub(crate) fn acquire(
        &mut self,
        device: &DeviceOwner,
    ) -> Result<wgpu::SurfaceTexture, RenderError> {
        if self.reconfigure_before_next_frame {
            configure_surface(&self.surface, device, &self.configuration)?;
            self.reconfigure_before_next_frame = false;
        }

        match self.surface.get_current_texture() {
            Ok(texture) => Ok(texture),
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                configure_surface(&self.surface, device, &self.configuration)?;
                self.surface
                    .get_current_texture()
                    .map_err(map_surface_error)
            }
            Err(error) => Err(map_surface_error(error)),
        }
    }

    pub(crate) fn note_suboptimal_frame(&mut self, suboptimal: bool) {
        self.reconfigure_before_next_frame |= suboptimal;
    }
}

fn surface_configuration(
    surface: &wgpu::Surface<'_>,
    adapter: &wgpu::Adapter,
    size: TargetSize,
) -> Result<wgpu::SurfaceConfiguration, RenderError> {
    let capabilities = surface.get_capabilities(adapter);
    let format = preferred_surface_format(&capabilities.formats)
        .ok_or(RenderError::UnsupportedCapabilities)?;
    let present_mode = capabilities
        .present_modes
        .iter()
        .copied()
        .find(|mode| *mode == wgpu::PresentMode::Fifo)
        .or_else(|| capabilities.present_modes.first().copied())
        .ok_or(RenderError::UnsupportedCapabilities)?;
    let alpha_mode = capabilities
        .alpha_modes
        .iter()
        .copied()
        .find(|mode| *mode == wgpu::CompositeAlphaMode::Auto)
        .or_else(|| capabilities.alpha_modes.first().copied())
        .ok_or(RenderError::UnsupportedCapabilities)?;

    Ok(wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format,
        width: size.width(),
        height: size.height(),
        present_mode,
        desired_maximum_frame_latency: 2,
        alpha_mode,
        view_formats: Vec::new(),
    })
}

fn preferred_surface_format(formats: &[wgpu::TextureFormat]) -> Option<wgpu::TextureFormat> {
    [
        wgpu::TextureFormat::Bgra8Unorm,
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureFormat::Bgra8UnormSrgb,
        wgpu::TextureFormat::Rgba8UnormSrgb,
    ]
    .into_iter()
    .find(|preferred| formats.contains(preferred))
}

fn configure_surface(
    surface: &wgpu::Surface<'_>,
    device: &DeviceOwner,
    configuration: &wgpu::SurfaceConfiguration,
) -> Result<(), RenderError> {
    device
        .with_error_scopes(|gpu| {
            surface.configure(gpu, configuration);
            Ok(())
        })
        .map_err(map_device_error)
}

fn map_surface_error(error: wgpu::SurfaceError) -> RenderError {
    match error {
        wgpu::SurfaceError::Lost => RenderError::SurfaceLost,
        wgpu::SurfaceError::Outdated => RenderError::SurfaceOutdated,
        wgpu::SurfaceError::Timeout => RenderError::SurfaceTimeout,
        wgpu::SurfaceError::OutOfMemory => RenderError::SurfaceOutOfMemory,
        wgpu::SurfaceError::Other => RenderError::SurfaceUnavailable,
    }
}
