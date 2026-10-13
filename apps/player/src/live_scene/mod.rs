//! Interactive windowed presentation for the bounded native physics scene.

pub(crate) mod arguments;

mod application;
mod clock;
mod completion;
mod error;
mod session;
mod surface;

use engine_platform::{WindowOptions, run_windowed};
use error::LiveSceneError;

const DEFAULT_WINDOW_WIDTH: u32 = 960;
const DEFAULT_WINDOW_HEIGHT: u32 = 640;

pub(crate) fn run(values: &[String]) -> Result<(), LiveSceneError> {
    let arguments = arguments::Arguments::parse(values)?;
    let options = WindowOptions::new(
        "OpenUnity Native Physics",
        DEFAULT_WINDOW_WIDTH,
        DEFAULT_WINDOW_HEIGHT,
    );
    let mut application = application::LiveApplication::new(arguments);

    run_windowed(options, &mut application)
        .map_err(|error| LiveSceneError::context("run live physics window", error))?;

    if let Some(summary) = application.take_summary() {
        let json = serde_json::to_string(&summary)
            .map_err(|error| LiveSceneError::context("serialize live physics summary", error))?;
        println!("{json}");
    }

    Ok(())
}
