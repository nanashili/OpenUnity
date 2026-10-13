//! Thin player entrypoint with one explicitly owned render route.

mod arguments;
mod render_frames;
mod render_manifest;
mod render_output;
mod render_preflight;
mod render_scene;
mod scene_reference_binding;

use std::error::Error;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result: Result<(), Box<dyn Error>> = if args
        .first()
        .is_some_and(|argument| argument == arguments::RENDER_SCENE_FLAG)
    {
        render_scene::run(&args).map_err(|error| Box::new(error) as Box<dyn Error>)
    } else {
        openunity_cli::run()
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("openunity-player: {error}");
            ExitCode::FAILURE
        }
    }
}
