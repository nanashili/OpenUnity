//! Record discovery warnings before publishing a candidate source project.

use openunity_editor_core::console::{Console, Severity};
use openunity_editor_core::project::ProjectDiagnostic;

pub(super) fn record_project_warnings(
    console: &mut Console,
    diagnostics: &[ProjectDiagnostic],
) -> Result<(), String> {
    for diagnostic in diagnostics {
        let message = format!(
            "{}: {}",
            diagnostic.relative_path.display(),
            diagnostic.message
        );
        console.push(Severity::Warning, &message).map_err(|error| {
            format!("project was not opened: recording project warning '{message}' failed: {error}")
        })?;
    }

    Ok(())
}

#[cfg(test)]
#[path = "project_warnings_tests.rs"]
mod tests;
