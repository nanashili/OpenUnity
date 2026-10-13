//! Bounded parsing for embedded-package and project-package metadata.

use std::path::PathBuf;

use serde_json::{Map, Value};

use super::super::ProjectPackageDiagnosticKind;
use super::model::{CatalogDiagnostics, PackageSourceDescriptor};

const MAX_PACKAGE_NAME_BYTES: usize = 255;
const MAX_DISPLAY_NAME_BYTES: usize = 256;
const MAX_VERSION_BYTES: usize = 128;

pub(in crate::project) fn is_valid_technical_name(name: &str) -> bool {
    if name.is_empty() || name.len() > MAX_PACKAGE_NAME_BYTES {
        return false;
    }

    let mut bytes = name.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    if !first.is_ascii_alphanumeric() {
        return false;
    }

    bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte))
}

pub(super) fn parse_package_manifest(
    bytes: &[u8],
    directory_name: &str,
    source_root: PathBuf,
    diagnostics: &mut CatalogDiagnostics,
) -> Option<PackageSourceDescriptor> {
    let candidate_name = validated_name(directory_name);
    let value = match parse_json(bytes) {
        Some(value) => value,
        None => {
            push_metadata_invalid(diagnostics, candidate_name.clone());
            return None;
        }
    };

    let Some(object) = value.as_object() else {
        push_metadata_invalid(diagnostics, candidate_name.clone());
        return None;
    };
    let Some(name) = object.get("name").and_then(Value::as_str) else {
        diagnostics.push(
            ProjectPackageDiagnosticKind::CandidateInvalid,
            candidate_name,
            Some(PathBuf::from("package.json")),
        );
        return None;
    };

    if !is_valid_technical_name(name) || name != directory_name {
        diagnostics.push(
            ProjectPackageDiagnosticKind::CandidateInvalid,
            candidate_name,
            Some(PathBuf::from("package.json")),
        );
        return None;
    }

    let display_name = optional_string(
        object,
        "displayName",
        MAX_DISPLAY_NAME_BYTES,
        diagnostics,
        name,
    );
    let version = optional_string(object, "version", MAX_VERSION_BYTES, diagnostics, name);
    let hide_in_editor = hide_in_editor(object, name, diagnostics);

    Some(PackageSourceDescriptor {
        name: name.to_owned(),
        display_name,
        version,
        hide_in_editor,
        source_root,
    })
}

pub(super) fn project_manifest_dependencies(
    bytes: &[u8],
    diagnostics: &mut CatalogDiagnostics,
) -> Option<Vec<String>> {
    let value = match parse_json(bytes) {
        Some(value) => value,
        None => {
            push_manifest_invalid(diagnostics);
            return None;
        }
    };

    let Some(object) = value.as_object() else {
        push_manifest_invalid(diagnostics);
        return None;
    };

    let Some(dependencies) = object.get("dependencies") else {
        return Some(Vec::new());
    };
    let Some(dependencies) = dependencies.as_object() else {
        push_manifest_invalid(diagnostics);
        return None;
    };

    Some(validated_dependency_names(dependencies, diagnostics))
}

fn parse_json(bytes: &[u8]) -> Option<Value> {
    serde_json::from_slice(bytes).ok()
}

fn push_metadata_invalid(diagnostics: &mut CatalogDiagnostics, name: Option<String>) {
    diagnostics.push(
        ProjectPackageDiagnosticKind::MetadataInvalid,
        name,
        Some(PathBuf::from("package.json")),
    );
}

fn push_manifest_invalid(diagnostics: &mut CatalogDiagnostics) {
    diagnostics.push(
        ProjectPackageDiagnosticKind::ManifestInvalid,
        None,
        Some(PathBuf::from("manifest.json")),
    );
}

fn optional_string(
    object: &Map<String, Value>,
    field: &str,
    maximum_bytes: usize,
    diagnostics: &mut CatalogDiagnostics,
    package_name: &str,
) -> Option<String> {
    match object.get(field) {
        None => None,
        Some(Value::String(value)) if value.len() <= maximum_bytes => Some(value.clone()),
        Some(_) => {
            diagnostics.push(
                ProjectPackageDiagnosticKind::MetadataInvalid,
                Some(package_name.to_owned()),
                Some(PathBuf::from("package.json")),
            );
            None
        }
    }
}

fn hide_in_editor(
    object: &Map<String, Value>,
    package_name: &str,
    diagnostics: &mut CatalogDiagnostics,
) -> bool {
    match object.get("hideInEditor") {
        None => true,
        Some(Value::Bool(value)) => *value,
        Some(_) => {
            diagnostics.push(
                ProjectPackageDiagnosticKind::MetadataInvalid,
                Some(package_name.to_owned()),
                Some(PathBuf::from("package.json")),
            );
            true
        }
    }
}

fn validated_dependency_names(
    dependencies: &Map<String, Value>,
    diagnostics: &mut CatalogDiagnostics,
) -> Vec<String> {
    let mut names = Vec::new();
    for (name, value) in dependencies {
        if !is_valid_technical_name(name) || !value.is_string() {
            diagnostics.push(
                ProjectPackageDiagnosticKind::DependencyInvalid,
                None,
                Some(PathBuf::from("manifest.json")),
            );
            continue;
        }

        names.push(name.clone());
    }

    names.sort_unstable();
    names
}

fn validated_name(name: &str) -> Option<String> {
    is_valid_technical_name(name).then(|| name.to_owned())
}
