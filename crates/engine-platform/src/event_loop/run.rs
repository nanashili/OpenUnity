use winit::event_loop::EventLoop;

use crate::{PlatformFailure, WindowApplication, WindowOptions, WindowRunError};

use super::handler::WindowHandler;

/// Runs an application on the desktop winit event loop.
///
/// Call this from the process main thread. Mobile and browser event lifecycles
/// are outside this API.
pub fn run_windowed<A: WindowApplication>(
    options: WindowOptions,
    application: &mut A,
) -> Result<(), WindowRunError<A::Error>> {
    if options.width == 0 || options.height == 0 {
        return finish_before_start(
            application,
            PlatformFailure::InvalidInitialSize {
                width: options.width,
                height: options.height,
            },
        );
    }

    let event_loop = match EventLoop::new() {
        Ok(event_loop) => event_loop,
        Err(error) => {
            return finish_before_start(
                application,
                PlatformFailure::EventLoopInitialization(error.to_string()),
            );
        }
    };

    let mut handler = WindowHandler::new(options, application);
    let event_loop_result = event_loop.run_app(&mut handler);

    handler.finish_application();

    if let Err(error) = event_loop_result {
        handler.record_platform_failure(PlatformFailure::EventLoopRuntime(error.to_string()));
    }

    let failure = handler.into_failure();
    if failure.is_empty() {
        Ok(())
    } else {
        Err(failure)
    }
}

fn finish_before_start<A: WindowApplication>(
    application: &mut A,
    platform: PlatformFailure,
) -> Result<(), WindowRunError<A::Error>> {
    let cleanup = application.exiting().err();
    Err(WindowRunError {
        platform: Some(platform),
        application: None,
        cleanup,
    })
}
