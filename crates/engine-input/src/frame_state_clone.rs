use super::frame_state_helpers::clone_string;
use super::{ControlState, DeviceState, FrameError, FrameInput, ReleaseTombstone};

impl FrameInput {
    /// Build a fallible deep snapshot without changing this frame state.
    pub fn try_clone(&self) -> Result<Self, FrameError> {
        let mut devices = Vec::new();
        devices
            .try_reserve(self.devices.len())
            .map_err(|_| FrameError::CapacityOverflow)?;
        for device in &self.devices {
            devices.push(clone_device(device)?);
        }

        let mut known_devices = Vec::new();
        known_devices
            .try_reserve(self.known_devices.len())
            .map_err(|_| FrameError::CapacityOverflow)?;
        known_devices.extend(self.known_devices.iter().copied());

        let mut releases = Vec::new();
        releases
            .try_reserve(self.releases.len())
            .map_err(|_| FrameError::CapacityOverflow)?;
        for release in &self.releases {
            releases.push(ReleaseTombstone {
                device: release.device,
                path: clone_string(&release.path)?,
                released: release.released,
            });
        }

        Ok(Self {
            devices,
            known_devices,
            releases,
        })
    }
}

fn clone_device(device: &DeviceState) -> Result<DeviceState, FrameError> {
    let mut controls = Vec::new();
    controls
        .try_reserve(device.controls.len())
        .map_err(|_| FrameError::CapacityOverflow)?;
    for control in &device.controls {
        controls.push(ControlState {
            path: clone_string(&control.path)?,
            kind: control.kind,
            value: control.value.clone(),
            pressed: control.pressed,
            released: control.released,
        });
    }
    Ok(DeviceState {
        id: device.id,
        controls,
    })
}
