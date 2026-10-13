use std::sync::Arc;
use std::time::Instant;

use engine_platform::{NativeWindow, WindowAction, WindowApplication, WindowEvent};
use engine_render_api::TargetSize;

use super::arguments::Arguments;
use super::clock::FixedStepClock;
use super::completion::finish_session;
use super::error::LiveSceneError;
use super::session::{LiveExecutionSummary, LiveSceneSession};
use super::surface::LiveSurface;

/// Owns live physics state and coordinates its window lifecycle.
pub(super) struct LiveApplication {
    arguments: Arguments,
    session: Option<LiveSceneSession>,
    surface: Option<LiveSurface>,
    fixed_clock: FixedStepClock,
    paused: bool,
    presented_frames: u64,
    summary: Option<LiveExecutionSummary>,
}

impl LiveApplication {
    pub(super) fn new(arguments: Arguments) -> Self {
        Self {
            arguments,
            session: None,
            surface: None,
            fixed_clock: FixedStepClock::new(),
            paused: false,
            presented_frames: 0,
            summary: None,
        }
    }

    pub(super) fn take_summary(&mut self) -> Option<LiveExecutionSummary> {
        self.summary.take()
    }

    fn resume(&mut self, window: Arc<NativeWindow>) -> Result<(), LiveSceneError> {
        if self.session.is_none() {
            self.session = Some(LiveSceneSession::create(self.arguments.dimension)?);
        }

        self.surface.take();
        self.surface = Some(LiveSurface::create(window, self.arguments.policy)?);
        self.fixed_clock.reset();
        self.request_redraw();
        Ok(())
    }

    fn resize(&mut self, width: u32, height: u32) -> Result<(), LiveSceneError> {
        let Some(surface) = self.surface.as_mut() else {
            self.fixed_clock.reset();
            return Ok(());
        };

        surface.resize(width, height)?;
        self.fixed_clock.reset();
        self.request_redraw();
        Ok(())
    }

    fn set_occluded(&mut self, occluded: bool) {
        if let Some(surface) = self.surface.as_mut() {
            surface.set_occluded(occluded);
        }

        self.fixed_clock.reset();
    }

    fn suspend(&mut self) {
        self.surface = None;
        self.fixed_clock.reset();
    }

    fn present_frame(&mut self, at: Instant) -> Result<WindowAction, LiveSceneError> {
        let Some(target_size) = self.render_target() else {
            return Ok(WindowAction::Continue);
        };

        let due_steps = self.steps_due(at);
        self.advance_scene(due_steps)?;
        self.present_scene(target_size)?;

        self.record_presented_frame()
    }

    fn render_target(&mut self) -> Option<TargetSize> {
        let target_size = self.surface.as_ref().and_then(LiveSurface::render_target);
        if target_size.is_none() {
            self.fixed_clock.reset();
        }

        target_size
    }

    fn steps_due(&mut self, at: Instant) -> usize {
        if self.paused {
            self.fixed_clock.reset();
            return 0;
        }

        self.fixed_clock.steps_due(at)
    }

    fn advance_scene(&mut self, step_count: usize) -> Result<(), LiveSceneError> {
        if self.paused {
            return Ok(());
        }

        let session = self
            .session
            .as_mut()
            .ok_or_else(|| LiveSceneError::new("live physics scene is not initialized"))?;

        for _ in 0..step_count {
            session.step_fixed()?;
        }

        Ok(())
    }

    fn present_scene(&mut self, target_size: TargetSize) -> Result<(), LiveSceneError> {
        let frame = self
            .session
            .as_ref()
            .ok_or_else(|| LiveSceneError::new("live physics scene is not initialized"))?
            .render_frame(target_size, self.presented_frames)?;
        let surface = self
            .surface
            .as_mut()
            .ok_or_else(|| LiveSceneError::new("live window surface is not initialized"))?;

        surface.present(&frame)
    }

    fn request_redraw(&self) {
        if let Some(surface) = self.surface.as_ref() {
            surface.request_redraw();
        }
    }

    fn toggle_pause(&mut self) {
        self.paused = !self.paused;
        self.fixed_clock.reset();
    }

    fn record_presented_frame(&mut self) -> Result<WindowAction, LiveSceneError> {
        self.presented_frames = self.presented_frames.checked_add(1).ok_or_else(|| {
            LiveSceneError::new("presented frame count exceeded its supported range")
        })?;

        if self
            .arguments
            .frame_limit
            .is_some_and(|limit| self.presented_frames >= limit)
        {
            return Ok(WindowAction::Exit);
        }

        Ok(WindowAction::Continue)
    }

    fn close_resources(&mut self) -> Result<(), LiveSceneError> {
        self.fixed_clock.reset();
        self.surface = None;

        let Some(mut session) = self.session.take() else {
            return Ok(());
        };

        self.summary = Some(finish_session(&mut session, self.presented_frames)?);
        Ok(())
    }
}

impl WindowApplication for LiveApplication {
    type Error = LiveSceneError;

    fn resumed(&mut self, window: Arc<NativeWindow>) -> Result<(), Self::Error> {
        self.resume(window)
    }

    fn event(&mut self, event: WindowEvent) -> Result<WindowAction, Self::Error> {
        match event {
            WindowEvent::RedrawRequested { at } => self.present_frame(at),
            WindowEvent::Resized { width, height } => {
                self.resize(width, height)?;
                Ok(WindowAction::Continue)
            }
            WindowEvent::CloseRequested | WindowEvent::EscapePressed => Ok(WindowAction::Exit),
            WindowEvent::Occluded(occluded) => {
                self.set_occluded(occluded);
                Ok(WindowAction::Continue)
            }
            WindowEvent::Suspended => {
                self.suspend();
                Ok(WindowAction::Continue)
            }
            WindowEvent::Focused(false) => {
                self.fixed_clock.reset();
                Ok(WindowAction::Continue)
            }
            WindowEvent::Focused(true) => Ok(WindowAction::Continue),
            WindowEvent::SpacePressed => {
                self.toggle_pause();
                Ok(WindowAction::Continue)
            }
        }
    }

    fn exiting(&mut self) -> Result<(), Self::Error> {
        self.close_resources()
    }
}
