//! Headless native runtime and managed gameplay host.

use std::process::ExitCode;

fn main() -> ExitCode {
    match openunity_cli::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("openunity-cli: {error}");
            ExitCode::FAILURE
        }
    }
}
