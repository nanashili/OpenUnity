use super::{DeviceId, FrameError, FrameInput, InputValue};

impl FrameInput {
    /// Return whether the named button is currently held.
    pub fn held(&self, device: DeviceId, control: &str) -> Result<bool, FrameError> {
        match self.lookup(device, control) {
            Ok(InputValue::Button(value)) => Ok(*value),
            Ok(_) => Ok(false),
            Err(FrameError::UnknownControl) if self.known_devices.contains(&device) => Ok(false),
            Err(error) => Err(error),
        }
    }

    /// Return whether the button was pressed during the current frame.
    pub fn pressed(&self, device: DeviceId, control: &str) -> Result<bool, FrameError> {
        Ok(self.lookup_control(device, control)?.pressed)
    }

    /// Return whether the button was released during the current frame.
    pub fn released(&self, device: DeviceId, control: &str) -> Result<bool, FrameError> {
        if let Ok(state) = self.lookup_control(device, control) {
            return Ok(state.released);
        }

        if self
            .releases
            .iter()
            .any(|release| release.device == device && release.path == control && release.released)
        {
            return Ok(true);
        }

        Err(FrameError::UnknownControl)
    }

    /// Return the current typed value for a registered control.
    pub fn value(&self, device: DeviceId, control: &str) -> Result<InputValue, FrameError> {
        Ok(self.lookup(device, control)?.clone())
    }
}
