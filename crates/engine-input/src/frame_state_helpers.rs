use super::{
    ControlKind, DeviceDescriptor, DeviceKind, DeviceState, FrameError, InputValue,
    ReleaseTombstone,
};

const MAX_DEVICE_NAME_BYTES: usize = 256;
const MAX_CONTROL_PATH_BYTES: usize = 256;
const MAX_CUSTOM_KIND_BYTES: usize = 64;
const MAX_CONTROLS: usize = 1024;

pub(super) fn release_buttons(device: &mut DeviceState) {
    for control in &mut device.controls {
        if let InputValue::Button(held) = &mut control.value {
            if *held {
                *held = false;
                control.released = true;
            }
        }
    }
}

pub(super) fn clear_non_buttons(device: &mut DeviceState) {
    for control in &mut device.controls {
        if !matches!(control.kind, ControlKind::Button) {
            control.value = neutral_value(control.kind);
        }
    }
}

pub(super) fn build_release_tombstones(
    state: &DeviceState,
) -> Result<Vec<ReleaseTombstone>, FrameError> {
    let mut releases = Vec::new();
    releases
        .try_reserve(state.controls.len())
        .map_err(|_| FrameError::CapacityOverflow)?;

    for control in &state.controls {
        if matches!(control.kind, ControlKind::Button) {
            releases.push(ReleaseTombstone {
                device: state.id,
                path: clone_string(&control.path)?,
                released: control.released || matches!(control.value, InputValue::Button(true)),
            });
        }
    }

    Ok(releases)
}

pub(super) fn neutral_value(kind: ControlKind) -> InputValue {
    match kind {
        ControlKind::Button => InputValue::Button(false),
        ControlKind::Axis => InputValue::Scalar(0.0),
        ControlKind::Vector2 => InputValue::Vector2([0.0; 2]),
        ControlKind::Vector3 => InputValue::Vector3([0.0; 3]),
    }
}

pub(super) fn validate_value(value: &InputValue, kind: ControlKind) -> Result<(), FrameError> {
    let finite = match value {
        InputValue::Button(_) => true,
        InputValue::Scalar(value) => value.is_finite(),
        InputValue::Vector2(values) => values.iter().all(|value| value.is_finite()),
        InputValue::Vector3(values) => values.iter().all(|value| value.is_finite()),
    };
    if !finite {
        return Err(FrameError::NonFiniteValue);
    }

    let matching = matches!(
        (kind, value),
        (ControlKind::Button, InputValue::Button(_))
            | (ControlKind::Axis, InputValue::Scalar(_))
            | (ControlKind::Vector2, InputValue::Vector2(_))
            | (ControlKind::Vector3, InputValue::Vector3(_))
    );
    matching.then_some(()).ok_or(FrameError::TypeMismatch)
}

pub(super) fn validate_descriptor(descriptor: &DeviceDescriptor) -> Result<(), FrameError> {
    if descriptor.name.is_empty() {
        return Err(FrameError::UnknownControl);
    }
    if descriptor.name.len() > MAX_DEVICE_NAME_BYTES || descriptor.controls.len() > MAX_CONTROLS {
        return Err(FrameError::CapacityOverflow);
    }
    if let DeviceKind::Custom(name) = &descriptor.kind {
        if name.len() > MAX_CUSTOM_KIND_BYTES {
            return Err(FrameError::CapacityOverflow);
        }
    }

    let mut paths = Vec::new();
    paths
        .try_reserve(descriptor.controls.len())
        .map_err(|_| FrameError::CapacityOverflow)?;
    for control in &descriptor.controls {
        if control.path.is_empty() {
            return Err(FrameError::UnknownControl);
        }
        if control.path.len() > MAX_CONTROL_PATH_BYTES {
            return Err(FrameError::CapacityOverflow);
        }
        if paths.iter().any(|path: &&str| *path == control.path) {
            return Err(FrameError::UnknownControl);
        }
        paths.push(control.path.as_str());
    }
    Ok(())
}

pub(super) fn clone_string(source: &str) -> Result<String, FrameError> {
    let mut value = String::new();
    value
        .try_reserve(source.len())
        .map_err(|_| FrameError::CapacityOverflow)?;
    value.push_str(source);
    Ok(value)
}
