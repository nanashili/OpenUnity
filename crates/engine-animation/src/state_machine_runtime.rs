use super::state_graph::{
    AnimationStateId, ConditionKey, InterruptionRule, StateError, StateGraph, Transition,
};

#[derive(Clone, Debug, Eq, PartialEq)]
/// Commands accepted by the animation state machine.
pub enum StateCommand {
    /// Requests a transition to a destination matching the optional condition.
    TransitionTo {
        /// Requested destination state.
        destination: AnimationStateId,
        /// Optional condition used to select the transition.
        condition: Option<ConditionKey>,
    },
}

#[derive(Clone, Debug, PartialEq)]
/// Records emitted when a pending transition is applied.
pub enum StateRecord {
    /// The previous state was exited.
    Exited(AnimationStateId),
    /// The destination state was entered.
    Entered(AnimationStateId),
    /// The transition completed with its authored duration.
    Transitioned {
        /// Source state of the transition.
        source: AnimationStateId,
        /// Destination state of the transition.
        destination: AnimationStateId,
        /// Authored transition duration in seconds.
        duration: f32,
    },
}

struct PendingTransition {
    transition: Transition,
}

/// Applies validated commands against a borrowed state graph.
pub struct StateMachine<'graph> {
    graph: &'graph StateGraph,
    current: AnimationStateId,
    pending: Option<PendingTransition>,
}

impl<'graph> StateMachine<'graph> {
    /// Creates a machine positioned at the graph's initial state.
    pub fn new(graph: &'graph StateGraph) -> Self {
        Self {
            graph,
            current: graph.initial_state(),
            pending: None,
        }
    }

    /// Returns the currently active state.
    pub fn current(&self) -> AnimationStateId {
        self.current
    }

    /// Queues a matching transition for the next update.
    pub fn command(&mut self, command: StateCommand) -> Result<(), StateError> {
        let StateCommand::TransitionTo {
            destination,
            condition,
        } = command;
        let transition = self.find_transition(destination, condition.as_ref())?;

        if self.is_blocked() {
            return Err(StateError::TransitionBlocked);
        }

        self.pending = Some(PendingTransition { transition });
        Ok(())
    }

    /// Applies the pending transition and returns ordered lifecycle records.
    pub fn update(&mut self) -> Vec<StateRecord> {
        let Some(pending) = self.pending.take() else {
            return Vec::new();
        };
        let source = self.current;
        let destination = pending.transition.destination;
        self.current = destination;

        vec![
            StateRecord::Exited(source),
            StateRecord::Entered(destination),
            StateRecord::Transitioned {
                source,
                destination,
                duration: pending.transition.duration,
            },
        ]
    }

    fn find_transition(
        &self,
        destination: AnimationStateId,
        condition: Option<&ConditionKey>,
    ) -> Result<Transition, StateError> {
        self.graph
            .transitions()
            .iter()
            .find(|candidate| {
                candidate.source == self.current
                    && candidate.destination == destination
                    && candidate.condition.as_ref() == condition
            })
            .cloned()
            .ok_or(StateError::NoTransition)
    }

    fn is_blocked(&self) -> bool {
        self.pending
            .as_ref()
            .is_some_and(|pending| pending.transition.interruption == InterruptionRule::Block)
    }
}
