//! Bounded enumeration of direct `Packages` children.

use std::fs::{self, DirEntry};
use std::path::Path;

pub(super) enum ChildListing {
    Complete(Vec<DirEntry>),
    Overflow,
    Failed,
}

pub(super) fn collect_direct_children(packages_root: &Path) -> ChildListing {
    const MAX_CHILDREN: usize = 1_024;

    let mut directory = match fs::read_dir(packages_root) {
        Ok(directory) => directory,
        Err(_) => return ChildListing::Failed,
    };

    let mut children = Vec::new();
    loop {
        match directory.next() {
            None => return ChildListing::Complete(children),
            Some(Err(_)) => return ChildListing::Failed,
            Some(Ok(_)) if children.len() == MAX_CHILDREN => return ChildListing::Overflow,
            Some(Ok(child)) => children.push(child),
        }
    }
}
