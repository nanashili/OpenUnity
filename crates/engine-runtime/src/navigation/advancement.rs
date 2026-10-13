use engine_navigation::FollowResult;

use crate::Runtime;

use super::{NavigationRoute, RuntimeNavigationError, planning::navigation_agent_transform};

impl Runtime {
    /// Advance a route by an explicit maximum world-space distance.
    ///
    /// The route publishes its progress only after the root Transform write
    /// succeeds. Calls after completion return the same completed result.
    ///
    /// # Errors
    /// Returns an object, kind, hierarchy, physics, position, or follow error.
    pub fn advance_navigation_route(
        &mut self,
        route: &mut NavigationRoute<'_>,
        max_distance: f32,
    ) -> Result<FollowResult, RuntimeNavigationError> {
        let (transform_id, transform) = navigation_agent_transform(self, route.agent)?;
        let current_position = transform.position;

        if current_position != route.follower.expected_position() {
            return Err(RuntimeNavigationError::AgentPositionChanged);
        }

        let prepared = route.follower.prepare_advance(max_distance)?;
        let result = prepared.result();

        if result.position != current_position {
            self.transform_write(transform_id, 4, &result.position)?;
        }

        prepared.commit();
        Ok(result)
    }
}
