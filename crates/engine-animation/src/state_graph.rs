#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
/// Identifies a state in an animation graph.
pub struct AnimationStateId(u32);

impl AnimationStateId {
    /// Creates an identifier from its stable numeric value.
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the stable numeric value of this identifier.
    pub const fn value(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Names a condition that can enable a transition.
pub struct ConditionKey(String);

impl ConditionKey {
    /// Creates a condition key from any string-like value.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Controls whether a pending transition may be replaced.
pub enum InterruptionRule {
    /// Allows a new transition to replace a pending one.
    Allow,
    /// Keeps the pending transition until it is applied.
    Block,
}

#[derive(Clone, Debug, PartialEq)]
/// Describes a directed transition between two animation states.
pub struct Transition {
    /// State from which the transition starts.
    pub source: AnimationStateId,
    /// State reached by the transition.
    pub destination: AnimationStateId,
    /// Optional condition required to select the transition.
    pub condition: Option<ConditionKey>,
    /// Duration of the transition in seconds.
    pub duration: f32,
    /// Policy for replacing a pending transition.
    pub interruption: InterruptionRule,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Errors produced while validating or operating a state graph.
pub enum StateError {
    /// No states were supplied.
    EmptyGraph,
    /// No initial state was supplied.
    MissingInitialState,
    /// A referenced state is absent from the graph.
    UnknownState(AnimationStateId),
    /// A state identifier appears more than once.
    DuplicateState(AnimationStateId),
    /// A transition duration is negative or non-finite.
    InvalidDuration,
    /// A condition key is empty.
    InvalidCondition,
    /// No matching transition exists.
    NoTransition,
    /// A pending transition blocks replacement.
    TransitionBlocked,
}

/// Stores validated states and transitions for a state machine.
pub struct StateGraph {
    initial: AnimationStateId,
    transitions: Vec<Transition>,
}

impl StateGraph {
    /// Validates and creates a graph with the supplied initial state.
    pub fn new(
        states: Vec<AnimationStateId>,
        initial: Option<AnimationStateId>,
        transitions: Vec<Transition>,
    ) -> Result<Self, StateError> {
        validate_states(&states)?;
        let initial = initial.ok_or(StateError::MissingInitialState)?;

        if !states.contains(&initial) {
            return Err(StateError::UnknownState(initial));
        }

        validate_transitions(&states, &transitions)?;

        Ok(Self {
            initial,
            transitions,
        })
    }

    /// Returns the graph's initial state.
    pub fn initial_state(&self) -> AnimationStateId {
        self.initial
    }

    /// Returns transitions in their authored order.
    pub fn transitions(&self) -> &[Transition] {
        &self.transitions
    }
}

fn validate_states(states: &[AnimationStateId]) -> Result<(), StateError> {
    if states.is_empty() {
        return Err(StateError::EmptyGraph);
    }

    for (index, state) in states.iter().enumerate() {
        if states[..index].contains(state) {
            return Err(StateError::DuplicateState(*state));
        }
    }

    Ok(())
}

fn validate_transitions(
    states: &[AnimationStateId],
    transitions: &[Transition],
) -> Result<(), StateError> {
    for transition in transitions {
        if !states.contains(&transition.source) {
            return Err(StateError::UnknownState(transition.source));
        }
        if !states.contains(&transition.destination) {
            return Err(StateError::UnknownState(transition.destination));
        }
        if !transition.duration.is_finite() || transition.duration < 0.0 {
            return Err(StateError::InvalidDuration);
        }
        if transition
            .condition
            .as_ref()
            .is_some_and(|condition| condition.0.is_empty())
        {
            return Err(StateError::InvalidCondition);
        }
    }

    Ok(())
}
