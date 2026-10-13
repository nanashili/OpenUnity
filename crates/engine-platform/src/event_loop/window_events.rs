use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, StartCause, WindowEvent as WinitWindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::WindowId,
};

use crate::{WindowApplication, WindowEvent};

use super::handler::WindowHandler;
impl<A: WindowApplication> WindowHandler<'_, A> {
    pub(super) fn handle_window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WinitWindowEvent,
    ) {
        if self
            .window
            .as_ref()
            .is_none_or(|window| window.id() != window_id)
        {
            return;
        }

        match event {
            WinitWindowEvent::CloseRequested => {
                self.dispatch_event(WindowEvent::CloseRequested, event_loop);
            }
            WinitWindowEvent::Resized(size) => self.resized(size, event_loop),
            WinitWindowEvent::RedrawRequested => self.redraw_requested(event_loop),
            WinitWindowEvent::Focused(focused) => {
                self.dispatch_event(WindowEvent::Focused(focused), event_loop);
            }
            WinitWindowEvent::Occluded(occluded) => self.occlusion_changed(occluded, event_loop),
            WinitWindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed && !event.repeat =>
            {
                match event.physical_key {
                    PhysicalKey::Code(KeyCode::Escape) => {
                        self.dispatch_event(WindowEvent::EscapePressed, event_loop);
                    }
                    PhysicalKey::Code(KeyCode::Space) => {
                        self.dispatch_event(WindowEvent::SpacePressed, event_loop);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn resized(&mut self, size: PhysicalSize<u32>, event_loop: &ActiveEventLoop) {
        self.size = size.into();
        self.dispatch_event(
            WindowEvent::Resized {
                width: size.width,
                height: size.height,
            },
            event_loop,
        );

        if self.is_presentable() {
            self.request_immediate_redraw();
        } else {
            self.next_redraw = None;
        }
    }

    fn occlusion_changed(&mut self, occluded: bool, event_loop: &ActiveEventLoop) {
        self.occluded = occluded;
        self.dispatch_event(WindowEvent::Occluded(occluded), event_loop);

        if self.is_presentable() {
            self.request_immediate_redraw();
        } else {
            self.next_redraw = None;
        }
    }
}

impl<A: WindowApplication> ApplicationHandler for WindowHandler<'_, A> {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        self.timed_redraw(event_loop, cause);
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.window_resumed(event_loop);
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        self.window_suspended(event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WinitWindowEvent,
    ) {
        self.handle_window_event(event_loop, window_id, event);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.set_wait_strategy(event_loop);
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.finish_application();
    }
}
