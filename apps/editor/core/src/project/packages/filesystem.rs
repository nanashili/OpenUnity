//! Filesystem admission and bounded reads for package metadata.

mod candidates;
mod manifest;
mod read;
mod root;

pub(super) use candidates::{PackageDirectory, classify_package_directory, package_json_path};
pub(super) use manifest::project_manifest_path;
pub(super) use read::{BoundedReadError, read_bounded_file};
pub(super) use root::{PackagesRoot, inspect_packages_root};
