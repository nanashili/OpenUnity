//! Adapter classification and required target-format capability checks.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum AdapterClass {
    Hardware,
    Software,
}

pub(super) fn validate_adapter_target_formats(
    adapter: &wgpu::Adapter,
) -> Result<(), wgpu::TextureFormat> {
    let targets = [
        (
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        ),
        (
            wgpu::TextureFormat::Depth32Float,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
        ),
    ];
    for (format, required_usages) in targets {
        let features = adapter.get_texture_format_features(format);
        if !features.allowed_usages.contains(required_usages) {
            return Err(format);
        }
    }
    Ok(())
}

pub(super) fn classify_adapter(device_type: wgpu::DeviceType) -> Option<AdapterClass> {
    if matches!(
        device_type,
        wgpu::DeviceType::IntegratedGpu
            | wgpu::DeviceType::DiscreteGpu
            | wgpu::DeviceType::VirtualGpu
    ) {
        Some(AdapterClass::Hardware)
    } else if device_type == wgpu::DeviceType::Cpu {
        Some(AdapterClass::Software)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{AdapterClass, classify_adapter};

    #[test]
    fn classifies_supported_hardware_types() {
        for device_type in [
            wgpu::DeviceType::IntegratedGpu,
            wgpu::DeviceType::DiscreteGpu,
            wgpu::DeviceType::VirtualGpu,
        ] {
            assert_eq!(classify_adapter(device_type), Some(AdapterClass::Hardware));
        }
    }

    #[test]
    fn classifies_cpu_as_software() {
        assert_eq!(
            classify_adapter(wgpu::DeviceType::Cpu),
            Some(AdapterClass::Software)
        );
    }

    #[test]
    fn rejects_unknown_adapter_types() {
        assert_eq!(classify_adapter(wgpu::DeviceType::Other), None);
    }
}
