use super::error::LiveSceneError;
use super::session::{LiveExecutionSummary, LiveSceneSession};

/// Captures final solver facts, retires bodies, and builds a clean-close summary.
pub(super) fn finish_session(
    session: &mut LiveSceneSession,
    presented_frame_requests: u64,
) -> Result<LiveExecutionSummary, LiveSceneError> {
    let final_body_state = session.final_body_state();
    let retirement = session.retire_bodies();

    match (final_body_state, retirement) {
        (Ok(final_body_state), Ok(retirement)) => {
            Ok(session.execution_summary(presented_frame_requests, final_body_state, retirement))
        }
        (Err(state_error), Ok(_)) => Err(LiveSceneError::context(
            "capture final live physics body state",
            state_error,
        )),
        (Ok(_), Err(retirement_error)) => Err(retirement_error),
        (Err(state_error), Err(retirement_error)) => Err(LiveSceneError::new(format!(
            "{state_error}; body cleanup also failed: {retirement_error}"
        ))),
    }
}
