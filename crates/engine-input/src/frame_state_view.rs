use super::super::input_action_types::Trigger;
use super::{ControlState, DeviceId, FrameInput, InputValue, ReleaseTombstone};

impl FrameInput {
    pub(crate) fn action_values(
        &self,
        device: Option<DeviceId>,
        path: &str,
        trigger: Trigger,
    ) -> Vec<(Option<DeviceId>, InputValue)> {
        let mut values = Vec::new();

        for state in &self.devices {
            if device.is_some_and(|id| id != state.id) {
                continue;
            }
            append_live_value(&mut values, state.id, &state.controls, path, trigger);
        }

        if matches!(trigger, Trigger::Released) {
            append_release_values(&mut values, &self.releases, device, path);
        }

        values.sort_by_key(|(id, _)| id.map(DeviceId::to_raw).unwrap_or(0));
        values
    }
}

fn append_live_value(
    values: &mut Vec<(Option<DeviceId>, InputValue)>,
    device: DeviceId,
    controls: &[ControlState],
    path: &str,
    trigger: Trigger,
) {
    let Some(control) = controls.iter().find(|item| item.path == path) else {
        return;
    };
    if let Some(value) = action_value(control, trigger) {
        values.push((Some(device), value));
    }
}

fn append_release_values(
    values: &mut Vec<(Option<DeviceId>, InputValue)>,
    releases: &[ReleaseTombstone],
    device: Option<DeviceId>,
    path: &str,
) {
    for release in releases {
        if release.released && release.path == path && device.is_none_or(|id| id == release.device)
        {
            values.push((Some(release.device), InputValue::Button(false)));
        }
    }
}

fn action_value(control: &ControlState, trigger: Trigger) -> Option<InputValue> {
    match trigger {
        Trigger::Held if matches!(control.value, InputValue::Button(true)) => {
            Some(control.value.clone())
        }
        Trigger::Pressed if control.pressed => Some(InputValue::Button(true)),
        Trigger::Released if control.released => Some(InputValue::Button(false)),
        Trigger::Value => Some(control.value.clone()),
        _ => None,
    }
}
