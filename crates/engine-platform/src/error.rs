use std::{error::Error, fmt};

/// A failure while creating or running the native platform event loop.
#[derive(Debug)]
pub enum PlatformFailure {
    /// A zero width or height was supplied for the initial window.
    InvalidInitialSize {
        /// Requested width in physical pixels.
        width: u32,
        /// Requested height in physical pixels.
        height: u32,
    },
    /// Winit could not initialize the operating system event loop.
    EventLoopInitialization(String),
    /// Winit could not create the native window.
    WindowCreation(String),
    /// Winit reported an event-loop runtime error.
    EventLoopRuntime(String),
}

impl fmt::Display for PlatformFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInitialSize { width, height } => {
                write!(
                    formatter,
                    "initial window size must be nonzero, got {width}x{height}"
                )
            }
            Self::EventLoopInitialization(message) => {
                write!(formatter, "event-loop initialization failed: {message}")
            }
            Self::WindowCreation(message) => {
                write!(formatter, "native window creation failed: {message}")
            }
            Self::EventLoopRuntime(message) => {
                write!(formatter, "event-loop runtime failed: {message}")
            }
        }
    }
}

impl Error for PlatformFailure {}

/// Preserves platform, application, and application-cleanup failures together.
#[derive(Debug)]
pub struct WindowRunError<E> {
    pub(crate) platform: Option<PlatformFailure>,
    pub(crate) application: Option<E>,
    pub(crate) cleanup: Option<E>,
}

impl<E> WindowRunError<E> {
    pub(crate) fn is_empty(&self) -> bool {
        self.platform.is_none() && self.application.is_none() && self.cleanup.is_none()
    }

    /// Returns the platform failure, if one occurred.
    pub fn platform_failure(&self) -> Option<&PlatformFailure> {
        self.platform.as_ref()
    }

    /// Returns the first application failure, if one occurred.
    pub fn application_failure(&self) -> Option<&E> {
        self.application.as_ref()
    }

    /// Returns a failure from the application's final cleanup callback.
    pub fn cleanup_failure(&self) -> Option<&E> {
        self.cleanup.as_ref()
    }
}

impl<E: fmt::Display> fmt::Display for WindowRunError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut wrote_failure = false;

        if let Some(platform) = &self.platform {
            write!(formatter, "{platform}")?;
            wrote_failure = true;
        }

        if let Some(application) = &self.application {
            write_failure(
                formatter,
                &mut wrote_failure,
                "application failed",
                application,
            )?;
        }

        if let Some(cleanup) = &self.cleanup {
            write_failure(
                formatter,
                &mut wrote_failure,
                "application cleanup failed",
                cleanup,
            )?;
        }

        Ok(())
    }
}

fn write_failure(
    formatter: &mut fmt::Formatter<'_>,
    wrote_failure: &mut bool,
    label: &str,
    error: &impl fmt::Display,
) -> fmt::Result {
    if *wrote_failure {
        write!(formatter, "; ")?;
    }

    write!(formatter, "{label}: {error}")?;
    *wrote_failure = true;
    Ok(())
}

impl<E: Error + 'static> Error for WindowRunError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.application
            .as_ref()
            .map(|error| error as &(dyn Error + 'static))
            .or_else(|| {
                self.cleanup
                    .as_ref()
                    .map(|error| error as &(dyn Error + 'static))
            })
            .or_else(|| {
                self.platform
                    .as_ref()
                    .map(|error| error as &(dyn Error + 'static))
            })
    }
}
