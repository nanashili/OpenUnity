//! Reviewed project image declarations; no runtime asset discovery.

use super::IconAsset;

pub(super) const ASSETS: &[IconAsset] = &[
    IconAsset {
        id: "unity-icon:-8204150473280056172",
        bytes: local_reference_bytes!(
            "../../../assets/unity-7000.0.0a8/icons/d-cs-script-icon--8204150473280056172.png"
        ),
        dimensions: [256, 256],
        expected_bytes: 4396,
    },
    IconAsset {
        id: "unity-icon:-7606759526039521141",
        bytes: local_reference_bytes!(
            "../../../assets/unity-7000.0.0a8/icons/d-gameobject-icon--7606759526039521141.png"
        ),
        dimensions: [256, 256],
        expected_bytes: 8184,
    },
    IconAsset {
        id: "unity-icon:3764734208316478041",
        bytes: local_reference_bytes!(
            "../../../assets/unity-7000.0.0a8/icons/d-folder-icon-3764734208316478041.png"
        ),
        dimensions: [256, 256],
        expected_bytes: 1532,
    },
    IconAsset {
        id: "unity-icon:4387052765368226613",
        bytes: local_reference_bytes!(
            "../../../assets/unity-7000.0.0a8/icons/d-folderempty-icon-4387052765368226613.png"
        ),
        dimensions: [256, 256],
        expected_bytes: 1741,
    },
    IconAsset {
        id: "unity-icon:6115457227726860241",
        bytes: local_reference_bytes!(
            "../../../assets/unity-7000.0.0a8/icons/d-folderopened-icon-6115457227726860241.png"
        ),
        dimensions: [256, 256],
        expected_bytes: 2739,
    },
    IconAsset {
        id: "unity-icon:8316842575475577922",
        bytes: local_reference_bytes!(
            "../../../assets/unity-7000.0.0a8/icons/d-textasset-icon-8316842575475577922.png"
        ),
        dimensions: [256, 256],
        expected_bytes: 2657,
    },
    IconAsset {
        id: "unity-icon:8779834242249542326",
        bytes: local_reference_bytes!(
            "../../../assets/unity-7000.0.0a8/icons/d-defaultasset-icon-8779834242249542326.png"
        ),
        dimensions: [64, 64],
        expected_bytes: 382,
    },
];
