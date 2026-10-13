use std::fs;
use std::path::Path;

use engine_audio::wav::import_wav;

pub(crate) fn assert_output(path: &Path, rate: u32, channels: u16, samples: &[f32]) {
    let bytes = fs::read(path).expect("read mix output");
    let data_bytes = samples.len() * 4;

    assert_eq!(bytes.len(), 44 + data_bytes);
    assert_eq!(&bytes[0..4], b"RIFF");
    assert_eq!(
        u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
        (36 + data_bytes) as u32
    );
    assert_eq!(&bytes[8..12], b"WAVE");
    assert_eq!(&bytes[12..16], b"fmt ");
    assert_eq!(u32::from_le_bytes(bytes[16..20].try_into().unwrap()), 16);
    assert_eq!(u16::from_le_bytes(bytes[20..22].try_into().unwrap()), 3);
    assert_eq!(
        u16::from_le_bytes(bytes[22..24].try_into().unwrap()),
        channels
    );
    assert_eq!(u32::from_le_bytes(bytes[24..28].try_into().unwrap()), rate);
    assert_eq!(
        u16::from_le_bytes(bytes[32..34].try_into().unwrap()),
        channels * 4
    );
    assert_eq!(u16::from_le_bytes(bytes[34..36].try_into().unwrap()), 32);
    assert_eq!(&bytes[36..40], b"data");
    assert_eq!(
        u32::from_le_bytes(bytes[40..44].try_into().unwrap()),
        data_bytes as u32
    );

    let clip = import_wav(&bytes).expect("mix output re-imports");
    assert_eq!(clip.sample_rate_hz(), rate);
    assert_eq!(clip.channel_count(), channels);
    assert_eq!(clip.frame_count(), samples.len() / usize::from(channels));

    let actual_bits: Vec<_> = clip
        .f32_samples()
        .expect("mix output contains F32 samples")
        .iter()
        .map(|sample| sample.to_bits())
        .collect();
    let expected_bits: Vec<_> = samples.iter().map(|sample| sample.to_bits()).collect();
    assert_eq!(actual_bits, expected_bits);
}
