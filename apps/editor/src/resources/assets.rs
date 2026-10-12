//! Embedded image metadata and deterministic cache installation order.

mod common_controls;
mod inspector;
mod project;
mod search_cancel;
mod workspace;

pub(super) struct IconAsset {
    pub(super) id: &'static str,
    pub(super) bytes: &'static [u8],
    pub(super) dimensions: [u32; 2],
    pub(super) expected_bytes: usize,
}

// Preserve the original resource order, including decode diagnostic ordering.
// Unselected provenance inventory never enters the cache.
pub(super) const ICONS: &[&IconAsset] = &[
    &inspector::ASSETS[0],
    &inspector::ASSETS[1],
    &inspector::ASSETS[2],
    &inspector::ASSETS[3],
    &project::ASSETS[0],
    &workspace::ASSETS[0],
    &project::ASSETS[1],
    &workspace::ASSETS[1],
    &workspace::ASSETS[2],
    &workspace::ASSETS[3],
    &workspace::ASSETS[4],
    &workspace::ASSETS[5],
    &workspace::ASSETS[6],
    &workspace::ASSETS[7],
    &workspace::ASSETS[8],
    &workspace::ASSETS[9],
    &workspace::ASSETS[10],
    &workspace::ASSETS[11],
    &workspace::ASSETS[12],
    &workspace::ASSETS[13],
    &workspace::ASSETS[14],
    &workspace::ASSETS[15],
    &workspace::ASSETS[16],
    &project::ASSETS[2],
    &project::ASSETS[3],
    &workspace::ASSETS[17],
    &workspace::ASSETS[18],
    &project::ASSETS[4],
    &workspace::ASSETS[19],
    &workspace::ASSETS[20],
    &workspace::ASSETS[21],
    &workspace::ASSETS[22],
    &project::ASSETS[5],
    &workspace::ASSETS[23],
    &project::ASSETS[6],
    &common_controls::ASSETS[0],
    &common_controls::ASSETS[1],
    &common_controls::ASSETS[2],
    &common_controls::ASSETS[3],
    &common_controls::ASSETS[4],
    &common_controls::ASSETS[5],
    &common_controls::ASSETS[6],
    &common_controls::ASSETS[7],
    &common_controls::ASSETS[8],
    &common_controls::ASSETS[9],
    &common_controls::ASSETS[10],
    &common_controls::ASSETS[11],
    &common_controls::ASSETS[12],
    &common_controls::ASSETS[13],
    &search_cancel::ASSETS[0],
    &search_cancel::ASSETS[1],
    &search_cancel::ASSETS[2],
    &search_cancel::ASSETS[3],
    &search_cancel::ASSETS[4],
    &search_cancel::ASSETS[5],
];
