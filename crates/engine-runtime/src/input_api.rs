//! Public runtime input forwarding surface.

use crate::{Runtime, RuntimeInputError};
use engine_input::{ActionName, ActionSample, Binding, DeviceId, FrameInput, InputEvent};

impl Runtime {
    /// Queue one validated in-process device event for the next outer frame.
    ///
    /// Events may be submitted while lifecycle callbacks are running; those
    /// events remain queued until the following outer-frame commit.
    /// At most 4,096 events may be pending. A full queue rejects the event
    /// without consuming its sequence or changing previously accepted input.
    pub fn input_push_event(&mut self, event: InputEvent) -> Result<(), RuntimeInputError> {
        self.input.submit(event)
    }

    /// Borrow the most recently committed immutable frame input snapshot.
    pub fn input_frame(&self) -> &FrameInput {
        self.input.input_frame()
    }

    /// Sample action bindings from the published frame snapshot.
    pub fn input_actions(&self) -> Vec<ActionSample> {
        self.input.sample()
    }

    /// Install one action binding while the runtime is idle.
    pub fn input_bind(&mut self, binding: Binding) -> Result<(), RuntimeInputError> {
        let lifecycle_ready = self.input_binding_ready();
        self.input.bind(binding, lifecycle_ready)
    }

    /// Remove one matching action binding while the runtime is idle.
    pub fn input_unbind(
        &mut self,
        action: &ActionName,
        control: &str,
        device: Option<DeviceId>,
    ) -> Result<bool, RuntimeInputError> {
        let lifecycle_ready = self.input_binding_ready();
        self.input.unbind(action, control, device, lifecycle_ready)
    }

    /// Cancel all bindings for one action while the runtime is idle.
    pub fn input_cancel(&mut self, action: &ActionName) -> Result<usize, RuntimeInputError> {
        let lifecycle_ready = self.input_binding_ready();
        self.input.cancel(action, lifecycle_ready)
    }
}
