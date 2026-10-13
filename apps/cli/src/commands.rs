//! Shared standalone command selection; command modules own preparation and execution.

mod audio_export;
mod audio_output;
mod diagnostics;
mod scene;
mod script;
mod session;
mod timed_scene;

use std::error::Error;

/// Execute supported standalone commands using process arguments.
pub fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();

    match args.as_slice() {
        [] => {
            diagnostics::print_usage();
            Ok(())
        }
        [arg] if arg == "--self-check" => diagnostics::self_check(),
        [flag, ..] if flag == "--export-wav" => audio_export::run(&args),
        [flag, ..] if flag == "--run-scene" => scene::run(&args),
        [flag, ..] if flag == "--run-timed-scene" => timed_scene::run(&args),
        [flag, ..] if matches!(flag.as_str(), "--run-script" | "--run-gameplay") => {
            script::run(&args)
        }
        _ => Err("unsupported arguments; run without arguments for usage".into()),
    }
}
