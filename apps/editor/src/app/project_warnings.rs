//! Record discovery warnings before publishing a candidate source project.

use openunity_editor_core::console::{Console, Severity};
use openunity_editor_core::project::{
    Project, ProjectDiagnostic, ProjectPackageDiagnostic, ProjectSourceDiagnostics,
};

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

pub(super) fn record_package_warnings(
    console: &mut Console,
    project: &Project,
) -> Result<(), String> {
    record_package_catalog_warnings(console, project.package_diagnostics())?;
    record_package_content_warnings(console, project)?;

    Ok(())
}

fn record_package_catalog_warnings(
    console: &mut Console,
    diagnostics: &[ProjectPackageDiagnostic],
) -> Result<(), String> {
    for diagnostic in diagnostics {
        let message = package_catalog_warning_message(diagnostic);
        record_package_warning(console, &message)?;
    }

    Ok(())
}

fn record_package_content_warnings(console: &mut Console, project: &Project) -> Result<(), String> {
    for package in project.packages() {
        let package_name = package.name();
        let source = project.source(Some(package_name)).ok_or_else(|| {
            format!(
                "project was not opened: embedded package source '{package_name}' was unavailable while recording warnings"
            )
        })?;

        let ProjectSourceDiagnostics::EmbeddedPackage(diagnostics) = source.diagnostics() else {
            return Err(format!(
                "project was not opened: embedded package '{package_name}' returned Assets diagnostics"
            ));
        };

        for diagnostic in diagnostics {
            let message = package_content_warning_message(package_name, diagnostic);
            record_package_warning(console, &message)?;
        }
    }

    Ok(())
}

fn package_catalog_warning_message(diagnostic: &ProjectPackageDiagnostic) -> String {
    match (diagnostic.package_name(), diagnostic.relative_path()) {
        (Some(package_name), Some(relative_path)) => format!(
            "package catalog '{package_name}' at {}: {}",
            relative_path.display(),
            diagnostic.message()
        ),
        (Some(package_name), None) => {
            format!("package catalog '{package_name}': {}", diagnostic.message())
        }
        (None, Some(relative_path)) => format!(
            "package catalog at {}: {}",
            relative_path.display(),
            diagnostic.message()
        ),
        (None, None) => format!("package catalog: {}", diagnostic.message()),
    }
}

fn package_content_warning_message(
    package_name: &str,
    diagnostic: &ProjectPackageDiagnostic,
) -> String {
    match diagnostic.relative_path() {
        Some(relative_path) if !relative_path.as_os_str().is_empty() => format!(
            "Packages/{package_name}/{}: {}",
            relative_path.display(),
            diagnostic.message()
        ),
        _ => format!("Packages/{package_name}: {}", diagnostic.message()),
    }
}

fn record_package_warning(console: &mut Console, message: &str) -> Result<(), String> {
    console.push(Severity::Warning, message).map_err(|error| {
        format!("project was not opened: recording package warning '{message}' failed: {error}")
    })?;

    Ok(())
}

#[cfg(test)]
#[path = "project_warnings_tests.rs"]
mod tests;
