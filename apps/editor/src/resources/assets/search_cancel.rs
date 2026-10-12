//! Reviewed search cancel image declarations; no runtime asset discovery.

use super::IconAsset;

pub(super) const ASSETS: &[IconAsset] = &[
    IconAsset {
        id: "unity-icon:-5230443368201148223",
        bytes: local_reference_bytes!(
            "../../../assets/unity-7000.0.0a8/search-cancel/toolbarsearchcancelbutton---5230443368201148223.png"
        ),
        dimensions: [14, 14],
        expected_bytes: 454,
    },
    IconAsset {
        id: "unity-icon:3847086183118950969",
        bytes: local_reference_bytes!(
            "../../../assets/unity-7000.0.0a8/search-cancel/toolbarsearchcancelbutton-2x--3847086183118950969.png"
        ),
        dimensions: [28, 28],
        expected_bytes: 805,
    },
    IconAsset {
        id: "unity-icon:1146234159502602164",
        bytes: local_reference_bytes!(
            "../../../assets/unity-7000.0.0a8/search-cancel/toolbarsearchcancelbuttonactive--1146234159502602164.png"
        ),
        dimensions: [14, 14],
        expected_bytes: 460,
    },
    IconAsset {
        id: "unity-icon:8100730101251085683",
        bytes: local_reference_bytes!(
            "../../../assets/unity-7000.0.0a8/search-cancel/toolbarsearchcancelbuttonactive-2x--8100730101251085683.png"
        ),
        dimensions: [28, 28],
        expected_bytes: 815,
    },
    IconAsset {
        id: "unity-icon:4511231667045328251",
        bytes: local_reference_bytes!(
            "../../../assets/unity-7000.0.0a8/search-cancel/toolbarsearchcancelbuttonoff--4511231667045328251.png"
        ),
        dimensions: [14, 14],
        expected_bytes: 305,
    },
    IconAsset {
        id: "unity-icon:-2936632161100208059",
        bytes: local_reference_bytes!(
            "../../../assets/unity-7000.0.0a8/search-cancel/toolbarsearchcancelbuttonoff-2x---2936632161100208059.png"
        ),
        dimensions: [28, 28],
        expected_bytes: 622,
    },
];
