//! Reviewed inspector image declarations; no runtime asset discovery.

use super::IconAsset;

pub(super) const ASSETS: &[IconAsset] = &[
    IconAsset {
        id: "unity-icon:6368833644570539117",
        bytes: local_reference_bytes!(
            "../../../assets/unity-7000.0.0a8/icons/in-lockbutton-6368833644570539117.png"
        ),
        dimensions: [16, 16],
        expected_bytes: 327,
    },
    IconAsset {
        id: "unity-icon:-2756979610131851051",
        bytes: local_reference_bytes!(
            "../../../assets/unity-7000.0.0a8/icons/in-lockbutton-on--2756979610131851051.png"
        ),
        dimensions: [16, 16],
        expected_bytes: 284,
    },
    IconAsset {
        id: "unity-icon:8554052077915636359",
        bytes: local_reference_bytes!(
            "../../../assets/unity-7000.0.0a8/icons/in-lockbutton-2x-8554052077915636359.png"
        ),
        dimensions: [32, 32],
        expected_bytes: 356,
    },
    IconAsset {
        id: "unity-icon:3623881695602462711",
        bytes: local_reference_bytes!(
            "../../../assets/unity-7000.0.0a8/icons/in-lockbutton-on-2x-3623881695602462711.png"
        ),
        dimensions: [32, 32],
        expected_bytes: 320,
    },
];
