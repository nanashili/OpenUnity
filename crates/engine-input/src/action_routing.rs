//! Deterministic action binding and snapshot sampling.

use engine_identifiers::ObjectId;

use super::frame_state::FrameInput;
use super::input_action_types::{
    ActionName, ActionSample, Binding, BindingError, MAX_BINDINGS, Trigger,
};
use super::input_event_types::{DeviceId, InputValue};

const MAX_CONTROL_PATH_BYTES: usize = 256;

/// Stores action bindings and samples them in deterministic priority order.
#[derive(Clone, Debug, Default)]
pub struct ActionRouter {
    bindings: Vec<Binding>,
}

impl ActionRouter {
    /// Create an action router with no bindings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a binding unless an identical binding already exists.
    pub fn bind(&mut self, binding: Binding) -> Result<(), BindingError> {
        validate_binding(&binding)?;

        if self.bindings.len() == MAX_BINDINGS {
            return Err(BindingError::CapacityOverflow);
        }

        if self
            .bindings
            .iter()
            .any(|existing| same_binding(existing, &binding))
        {
            return Err(BindingError::DuplicateBinding);
        }

        self.bindings
            .try_reserve(1)
            .map_err(|_| BindingError::CapacityOverflow)?;

        self.bindings.push(binding);
        Ok(())
    }

    /// Remove bindings matching the action, control, and optional device.
    pub fn unbind(
        &mut self,
        action: &ActionName,
        control: &str,
        device: Option<DeviceId>,
    ) -> Result<bool, BindingError> {
        if control.is_empty() {
            return Err(BindingError::EmptyControl);
        }
        if control.len() > MAX_CONTROL_PATH_BYTES {
            return Err(BindingError::CapacityOverflow);
        }

        let previous_len = self.bindings.len();
        self.bindings.retain(|binding| {
            binding.action != *action || binding.control != control || binding.device != device
        });
        Ok(previous_len != self.bindings.len())
    }

    /// Cancel every binding associated with the given action.
    pub fn cancel(&mut self, action: &ActionName) -> Result<usize, BindingError> {
        if action.as_str().is_empty() || action.as_str().len() > 128 {
            return Err(BindingError::InvalidName);
        }

        let previous_len = self.bindings.len();
        self.bindings.retain(|binding| binding.action != *action);
        Ok(previous_len - self.bindings.len())
    }

    /// Sample every matching binding from one immutable frame snapshot.
    ///
    /// Higher priority values come first, followed by action, control, device,
    /// and trigger order to make ties reproducible.
    pub fn sample(&self, state: &FrameInput) -> Vec<ActionSample> {
        self.sample_view(state)
    }

    fn sample_view(&self, state: &FrameInput) -> Vec<ActionSample> {
        let mut routed = Vec::new();
        for binding in &self.bindings {
            let values = state.action_values(binding.device, &binding.control, binding.trigger);
            routed.extend(values.into_iter().map(|(device, value)| RoutedSample {
                action: binding.action.clone(),
                control: binding.control.clone(),
                priority: binding.priority,
                trigger: binding.trigger,
                value,
                device,
            }));
        }

        routed.sort_by(sample_order);
        routed.into_iter().map(RoutedSample::into_public).collect()
    }
}

fn validate_binding(binding: &Binding) -> Result<(), BindingError> {
    if binding.action.as_str().is_empty() || binding.action.as_str().len() > 128 {
        return Err(BindingError::InvalidName);
    }
    if binding.control.is_empty() {
        return Err(BindingError::EmptyControl);
    }
    if binding.control.len() > MAX_CONTROL_PATH_BYTES {
        return Err(BindingError::CapacityOverflow);
    }
    Ok(())
}

fn same_binding(left: &Binding, right: &Binding) -> bool {
    left.action == right.action
        && left.device == right.device
        && left.control == right.control
        && left.trigger == right.trigger
}

#[derive(Debug)]
struct RoutedSample {
    action: ActionName,
    control: String,
    priority: i32,
    trigger: Trigger,
    value: InputValue,
    device: Option<DeviceId>,
}

impl RoutedSample {
    fn into_public(self) -> ActionSample {
        ActionSample {
            action: self.action,
            trigger: self.trigger,
            value: self.value,
            device: self.device,
        }
    }
}

fn sample_order(left: &RoutedSample, right: &RoutedSample) -> std::cmp::Ordering {
    right
        .priority
        .cmp(&left.priority)
        .then_with(|| left.action.cmp(&right.action))
        .then_with(|| left.control.cmp(&right.control))
        .then_with(|| {
            left.device
                .map(ObjectId::to_raw)
                .unwrap_or(0)
                .cmp(&right.device.map(ObjectId::to_raw).unwrap_or(0))
        })
        .then_with(|| left.trigger.cmp(&right.trigger))
}
