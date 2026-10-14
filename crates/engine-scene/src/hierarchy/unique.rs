use super::HierarchyError;
use engine_identifiers::ObjectId;
use std::collections::HashSet;

pub(super) fn collect_unique(transforms: &[ObjectId]) -> Result<HashSet<ObjectId>, HierarchyError> {
    let mut unique = HashSet::new();
    unique
        .try_reserve(transforms.len())
        .map_err(|_| HierarchyError::Allocation)?;

    for &transform in transforms {
        if !unique.insert(transform) {
            return Err(HierarchyError::DuplicateNode);
        }
    }

    Ok(unique)
}

pub(super) fn ensure_unique(transforms: &[ObjectId]) -> Result<(), HierarchyError> {
    let mut seen = HashSet::new();
    seen.try_reserve(transforms.len())
        .map_err(|_| HierarchyError::Allocation)?;

    if transforms.iter().any(|transform| !seen.insert(*transform)) {
        return Err(HierarchyError::DuplicateNode);
    }

    Ok(())
}
