use std::{sync::Arc, time::Instant};

use winit::{dpi::PhysicalSize, event_loop::ActiveEventLoop, window::Window};

use crate::{
    PlatformFailure, WindowAction, WindowApplication, WindowEvent, WindowOptions, WindowRunError,
};

use super::super::window::{NativeWindow, WindowSize};

pub(super) struct WindowHandler<'application, A: WindowApplication> {
    pub(super) options: WindowOptions,
    pub(super) application: &'application mut A,
    pub(super) window: Option<Arc<NativeWindow>>,
    pub(super) size: WindowSize,
    pub(super) next_redraw: Option<Instant>,
    pub(super) occluded: bool,
    pub(super) suspended: bool,
    exiting_called: bool,
    platform_failure: Option<PlatformFailure>,
    application_failure: Option<A::Error>,
    cleanup_failure: Option<A::Error>,
}

impl<'application, A: WindowApplication> WindowHandler<'application, A> {
    pub(super) fn new(options: WindowOptions, application: &'application mut A) -> Self {
        Self {
            options,
            application,
            window: None,
            size: WindowSize::default(),
            next_redraw: None,
            occluded: false,
            suspended: false,
            exiting_called: false,
            platform_failure: None,
            application_failure: None,
            cleanup_failure: None,
        }
    }

    pub(super) fn finish_application(&mut self) {
        if self.exiting_called {
            return;
        }

        self.exiting_called = true;
        if let Err(error) = self.application.exiting() {
            self.cleanup_failure = Some(error);
        }
    }

    pub(super) fn record_platform_failure(&mut self, failure: PlatformFailure) {
        if self.platform_failure.is_none() {
            self.platform_failure = Some(failure);
        }
    }

    pub(super) fn into_failure(self) -> WindowRunError<A::Error> {
        WindowRunError {
            platform: self.platform_failure,
            application: self.application_failure,
            cleanup: self.cleanup_failure,
        }
    }

    pub(super) fn is_presentable(&self) -> bool {
        self.window.is_some()
            && self.size.width > 0
            && self.size.height > 0
            && !self.occluded
            && !self.suspended
    }

    pub(super) fn request_immediate_redraw(&mut self) {
        let Some(window) = self.window.as_ref() else {
            return;
        };

        if self.is_presentable() {
            window.request_redraw();
            self.next_redraw = Some(Instant::now() + super::redraw::REDRAW_INTERVAL);
        }
    }

    pub(super) fn dispatch_event(&mut self, event: WindowEvent, event_loop: &ActiveEventLoop) {
        if self.application_failure.is_some() {
            event_loop.exit();
            return;
        }

        let should_exit = matches!(
            event,
            WindowEvent::CloseRequested | WindowEvent::EscapePressed
        );
        match self.application.event(event) {
            Ok(WindowAction::Exit) => event_loop.exit(),
            Ok(WindowAction::Continue) if should_exit => event_loop.exit(),
            Ok(WindowAction::Continue) => {}
            Err(error) => {
                self.application_failure = Some(error);
                event_loop.exit();
            }
        }
    }

    fn create_window(&mut self, event_loop: &ActiveEventLoop) -> Option<Arc<NativeWindow>> {
        let attributes = Window::default_attributes()
            .with_title(self.options.title.clone())
            .with_inner_size(PhysicalSize::new(self.options.width, self.options.height));

        match event_loop.create_window(attributes) {
            Ok(window) => Some(Arc::new(NativeWindow::new(Arc::new(window)))),
            Err(error) => {
                self.record_platform_failure(PlatformFailure::WindowCreation(error.to_string()));
                event_loop.exit();
                None
            }
        }
    }

    pub(super) fn window_resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.platform_failure.is_some() || self.application_failure.is_some() {
            event_loop.exit();
            return;
        }

        let was_occluded = std::mem::replace(&mut self.occluded, false);
        self.suspended = false;

        if self.window.is_none() {
            self.window = self.create_window(event_loop);
        }

        let Some(window) = self.window.as_ref() else {
            return;
        };

        self.size = window.inner_size();
        if let Err(error) = self.application.resumed(Arc::clone(window)) {
            self.application_failure = Some(error);
            event_loop.exit();
            return;
        }

        if was_occluded {
            self.dispatch_event(WindowEvent::Occluded(false), event_loop);
        }

        self.request_immediate_redraw();
    }

    pub(super) fn window_suspended(&mut self, event_loop: &ActiveEventLoop) {
        self.suspended = true;
        self.next_redraw = None;
        self.dispatch_event(WindowEvent::Suspended, event_loop);
    }
}
