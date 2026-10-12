//! Runtime-owned composition for the reviewed `engine-input` contract.

use engine_input::{
    ActionName, ActionRouter, ActionSample, Binding, BindingError, DeviceId, EventError,
    EventQueue, FrameError, FrameInput, InputEvent,
};

use crate::RuntimeError;

const DEFAULT_QUEUE_CAPACITY: usize = 4_096;

/// Failure taxonomy for runtime input admission and frame preparation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeInputError {
    /// Existing runtime lifecycle, staging, timing, or frame admission error.
    Runtime(RuntimeError),
    /// Immediate event submission was rejected without queue mutation.
    Event(EventError),
    /// Immediate binding mutation was rejected without router mutation.
    Binding(BindingError),
    /// Candidate preparation failed and can be retried without publication.
    RetryablePreparation(InputPreparationError),
    /// An accepted event diverged from frame-state validation.
    InvariantFailure(FrameError),
}

/// Fallible preparation steps that leave canonical runtime state unchanged.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputPreparationError {
    /// The complete queue snapshot could not be allocated.
    QueueSnapshot(EventError),
    /// The exact pending-event batch could not be allocated.
    EventBatchCapacity,
    /// The frame snapshot or frame-state preparation could not be allocated.
    FrameState(FrameError),
}

/// A complete, uncommitted outer-frame input transition.
pub(crate) struct PreparedInputFrame {
    queue: Option<EventQueue>,
    frame: FrameInput,
}

/// Runtime-owned queue, published frame snapshot, and action bindings.
#[derive(Debug)]
pub(crate) struct InputState {
    queue: Option<EventQueue>,
    frame: FrameInput,
    router: ActionRouter,
    invariant_failure: Option<FrameError>,
}

impl InputState {
    /// Construct empty input state without allocating a queue.
    pub(crate) fn new() -> Self {
        Self {
            queue: None,
            frame: FrameInput::new(),
            router: ActionRouter::new(),
            invariant_failure: None,
        }
    }

    /// Accept one event; callback-time submission is deliberately supported.
    pub(crate) fn submit(&mut self, event: InputEvent) -> Result<(), RuntimeInputError> {
        self.ensure_healthy()?;
        let Some(queue) = self.queue.as_mut() else {
            let mut queue = EventQueue::with_capacity(DEFAULT_QUEUE_CAPACITY)
                .map_err(RuntimeInputError::Event)?;
            queue.push(event).map_err(RuntimeInputError::Event)?;
            self.queue = Some(queue);
            return Ok(());
        };
        queue.push(event).map_err(RuntimeInputError::Event)
    }

    /// Prepare the next snapshot without changing canonical queue or frame.
    pub(crate) fn prepare_outer_frame(&mut self) -> Result<PreparedInputFrame, RuntimeInputError> {
        self.ensure_healthy()?;
        let mut queue = self
            .queue
            .as_ref()
            .map(EventQueue::try_clone)
            .transpose()
            .map_err(retryable_queue_snapshot)?;
        let mut frame = self.frame.try_clone().map_err(retryable_frame_state)?;
        frame.begin_frame().map_err(retryable_frame_state)?;

        let mut events = Vec::new();
        if let Some(queue) = queue.as_mut() {
            events.try_reserve_exact(queue.len()).map_err(|_| {
                RuntimeInputError::RetryablePreparation(InputPreparationError::EventBatchCapacity)
            })?;
            queue.drain_into(&mut events).map_err(|_| {
                RuntimeInputError::RetryablePreparation(InputPreparationError::EventBatchCapacity)
            })?;
        }

        for event in &events {
            if let Err(error) = frame.apply(event) {
                if matches!(error, FrameError::CapacityOverflow) {
                    return Err(retryable_frame_state(error));
                }
                self.invariant_failure = Some(error);
                return Err(RuntimeInputError::InvariantFailure(error));
            }
        }

        Ok(PreparedInputFrame { queue, frame })
    }

    /// Publish a prepared ordinary or timed outer-frame transition.
    pub(crate) fn publish_outer_frame(&mut self, prepared: PreparedInputFrame) {
        self.queue = prepared.queue;
        self.frame = prepared.frame;
    }

    /// Install a binding after the lifecycle owner proves the runtime is idle.
    pub(crate) fn bind(
        &mut self,
        binding: Binding,
        lifecycle_ready: Result<(), RuntimeError>,
    ) -> Result<(), RuntimeInputError> {
        lifecycle_ready.map_err(RuntimeInputError::Runtime)?;
        self.ensure_healthy()?;
        self.router
            .bind(binding)
            .map_err(RuntimeInputError::Binding)
    }

    /// Remove bindings after the lifecycle owner proves the runtime is idle.
    pub(crate) fn unbind(
        &mut self,
        action: &engine_input::ActionName,
        control: &str,
        device: Option<DeviceId>,
        lifecycle_ready: Result<(), RuntimeError>,
    ) -> Result<bool, RuntimeInputError> {
        lifecycle_ready.map_err(RuntimeInputError::Runtime)?;
        self.ensure_healthy()?;
        self.router
            .unbind(action, control, device)
            .map_err(RuntimeInputError::Binding)
    }

    /// Cancel an action after the lifecycle owner proves the runtime is idle.
    pub(crate) fn cancel(
        &mut self,
        action: &ActionName,
        lifecycle_ready: Result<(), RuntimeError>,
    ) -> Result<usize, RuntimeInputError> {
        lifecycle_ready.map_err(RuntimeInputError::Runtime)?;
        self.ensure_healthy()?;
        self.router
            .cancel(action)
            .map_err(RuntimeInputError::Binding)
    }

    /// Sample actions from the immutable published frame.
    pub(crate) fn sample(&self) -> Vec<ActionSample> {
        self.router.sample(&self.frame)
    }

    /// Borrow the last successfully published frame snapshot.
    pub(crate) fn input_frame(&self) -> &FrameInput {
        &self.frame
    }

    fn ensure_healthy(&self) -> Result<(), RuntimeInputError> {
        self.invariant_failure.map_or(Ok(()), |error| {
            Err(RuntimeInputError::InvariantFailure(error))
        })
    }
}

fn retryable_queue_snapshot(error: EventError) -> RuntimeInputError {
    RuntimeInputError::RetryablePreparation(InputPreparationError::QueueSnapshot(error))
}

fn retryable_frame_state(error: FrameError) -> RuntimeInputError {
    RuntimeInputError::RetryablePreparation(InputPreparationError::FrameState(error))
}
