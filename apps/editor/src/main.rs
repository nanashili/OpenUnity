//! Native entry point for the independently specified OpenUnity editor.

use std::process::ExitCode;

mod startup;

fn main() -> ExitCode {
    match startup::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("OpenUnity editor: {error}");
            ExitCode::FAILURE
        }
    }
}
