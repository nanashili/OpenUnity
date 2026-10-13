//! Public state-machine transition rejection and recovery acceptance.

use engine_animation::{
    AnimationStateId, InterruptionRule, StateCommand, StateError, StateGraph, StateMachine,
    StateRecord, Transition,
};

#[test]
fn blocked_interruption_keeps_pending_transition_until_completion() {
    let idle = AnimationStateId::new(1);
    let running = AnimationStateId::new(2);
    let jumping = AnimationStateId::new(3);
    let graph = StateGraph::new(
        vec![idle, running, jumping],
        Some(idle),
        vec![
            Transition {
                source: idle,
                destination: running,
                condition: None,
                duration: 0.25,
                interruption: InterruptionRule::Block,
            },
            Transition {
                source: idle,
                destination: jumping,
                condition: None,
                duration: 0.5,
                interruption: InterruptionRule::Allow,
            },
        ],
    )
    .expect("graph references valid states and transitions");
    let mut machine = StateMachine::new(&graph);

    machine
        .command(StateCommand::TransitionTo {
            destination: running,
            condition: None,
        })
        .expect("first transition is available");

    assert_eq!(
        machine.command(StateCommand::TransitionTo {
            destination: jumping,
            condition: None,
        }),
        Err(StateError::TransitionBlocked)
    );
    assert_eq!(machine.current(), idle);

    assert_eq!(
        machine.update(),
        vec![
            StateRecord::Exited(idle),
            StateRecord::Entered(running),
            StateRecord::Transitioned {
                source: idle,
                destination: running,
                duration: 0.25,
            },
        ]
    );
    assert_eq!(machine.current(), running);
    assert!(machine.update().is_empty());
}
