use std::time::{Duration, Instant};

use winit::{
    event::StartCause,
    event_loop::{ActiveEventLoop, ControlFlow},
};

use crate::WindowEvent;

use super::handler::WindowHandler;

pub(super) const REDRAW_INTERVAL: Duration = Duration::from_nanos(16_666_667);

impl<A: crate::WindowApplication> WindowHandler<'_, A> {
    pub(super) fn timed_redraw(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        if !matches!(cause, StartCause::ResumeTimeReached { .. }) || !self.is_presentable() {
            return;
        }

        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
            self.next_redraw = Some(Instant::now() + REDRAW_INTERVAL);
        }

        if let Some(deadline) = self.next_redraw {
            event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
        }
    }

    pub(super) fn redraw_requested(&mut self, event_loop: &ActiveEventLoop) {
        if !self.is_presentable() {
            self.next_redraw = None;
            return;
        }

        self.dispatch_event(
            WindowEvent::RedrawRequested { at: Instant::now() },
            event_loop,
        );
        self.next_redraw = Some(Instant::now() + REDRAW_INTERVAL);
    }

    pub(super) fn set_wait_strategy(&self, event_loop: &ActiveEventLoop) {
        match (self.is_presentable(), self.next_redraw) {
            (true, Some(deadline)) => event_loop.set_control_flow(ControlFlow::WaitUntil(deadline)),
            _ => event_loop.set_control_flow(ControlFlow::Wait),
        }
    }
}
