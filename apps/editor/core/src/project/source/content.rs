//! Bounded, source-relative package content discovery.

mod budget;
mod discovery;

pub(in crate::project) use budget::PackageContentBudget;
pub(in crate::project) use discovery::discover_package_content;
