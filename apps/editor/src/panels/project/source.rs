//! Project source labels and logical paths at the UI boundary.

use std::path::Path;

use openunity_editor_core::project::{
    Project, ProjectSource, ProjectSourceDiagnostics, ProjectSourceKey,
};

pub(super) fn resolve<'a>(
    project: &'a Project,
    key: &ProjectSourceKey,
) -> Option<ProjectSource<'a>> {
    project.source(package_name(key))
}

pub(super) fn package_name(key: &ProjectSourceKey) -> Option<&str> {
    match key {
        ProjectSourceKey::Assets => None,
        ProjectSourceKey::EmbeddedPackage(name) => Some(name),
    }
}

pub(super) fn label(project: &Project, key: &ProjectSourceKey) -> String {
    let Some(name) = package_name(key) else {
        return "Assets".to_owned();
    };

    project
        .packages()
        .iter()
        .find(|package| package.name() == name)
        .and_then(|package| package.display_name())
        .filter(|display_name| !display_name.is_empty())
        .unwrap_or(name)
        .to_owned()
}

pub(super) fn source_role(key: &ProjectSourceKey) -> String {
    match key {
        ProjectSourceKey::Assets => "project.source.Assets".to_owned(),
        ProjectSourceKey::EmbeddedPackage(name) => format!("project.source.{name}"),
    }
}

pub(super) fn folder_role(key: &ProjectSourceKey, relative_path: &Path) -> String {
    format!("{}.folder.{}", source_role(key), relative_path.display())
}

pub(super) fn entry_role(key: &ProjectSourceKey, relative_path: &Path) -> String {
    format!("{}.entry.{}", source_role(key), relative_path.display())
}

pub(super) fn observation_key(key: &ProjectSourceKey) -> serde_json::Value {
    match key {
        ProjectSourceKey::Assets => serde_json::json!({ "kind": "assets" }),
        ProjectSourceKey::EmbeddedPackage(name) => {
            serde_json::json!({ "kind": "embedded_package", "name": name })
        }
    }
}

pub(super) fn diagnostic_count(source: ProjectSource<'_>) -> usize {
    match source.diagnostics() {
        ProjectSourceDiagnostics::Assets(diagnostics) => diagnostics.len(),
        ProjectSourceDiagnostics::EmbeddedPackage(diagnostics) => diagnostics.len(),
    }
}

pub(super) fn incomplete_warning(source: ProjectSource<'_>) -> Option<String> {
    let count = diagnostic_count(source);
    (count > 0).then(|| {
        format!(
            "Active source may be incomplete ({count} discovery diagnostic{}).",
            if count == 1 { "" } else { "s" }
        )
    })
}

pub(super) fn logical_root(key: &ProjectSourceKey) -> String {
    match key {
        ProjectSourceKey::Assets => "Assets".to_owned(),
        ProjectSourceKey::EmbeddedPackage(name) => format!("Packages/{name}"),
    }
}

pub(super) fn reference_search_hint(key: &ProjectSourceKey) -> String {
    format!("Search {}", logical_root(key))
}

pub(super) fn legacy_search_hint(key: &ProjectSourceKey) -> String {
    format!("Search {} paths", logical_root(key))
}

pub(super) fn legacy_filter_label(key: &ProjectSourceKey) -> String {
    format!("Filter {} paths:", logical_root(key))
}

pub(super) fn logical_path(key: &ProjectSourceKey, relative_path: &Path) -> String {
    match key {
        ProjectSourceKey::Assets => {
            if relative_path.as_os_str().is_empty() {
                "Assets".to_owned()
            } else {
                format!("Assets/{}", relative_path.display())
            }
        }
        ProjectSourceKey::EmbeddedPackage(_) => {
            let root = logical_root(key);
            let package_path = slash_separated_path(relative_path);

            if package_path.is_empty() {
                root
            } else {
                format!("{root}/{package_path}")
            }
        }
    }
}

pub(super) fn legacy_entry_label(key: &ProjectSourceKey, relative_path: &Path) -> String {
    match key {
        ProjectSourceKey::Assets => relative_path.display().to_string(),
        ProjectSourceKey::EmbeddedPackage(_) => logical_path(key, relative_path),
    }
}

pub(super) fn breadcrumb(key: &ProjectSourceKey, directory: &Path) -> String {
    match (key, directory.as_os_str().is_empty()) {
        (ProjectSourceKey::Assets, true) => "Assets".to_owned(),
        (ProjectSourceKey::Assets, false) => format!("Assets / {}", directory.display()),
        (ProjectSourceKey::EmbeddedPackage(_), true) => logical_root(key),
        (ProjectSourceKey::EmbeddedPackage(_), false) => {
            format!(
                "{} / {}",
                logical_root(key),
                slash_separated_path(directory)
            )
        }
    }
}

fn slash_separated_path(path: &Path) -> String {
    let mut logical_path = String::new();

    for component in path.components() {
        if !logical_path.is_empty() {
            logical_path.push('/');
        }

        logical_path.push_str(&component.as_os_str().to_string_lossy());
    }

    logical_path
}
