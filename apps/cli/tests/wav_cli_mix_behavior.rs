//! Black-box offline mixing, frame scheduling, gains, and channel ordering.

#[path = "common/mod.rs"]
mod common;

use common::wav_fixtures::{f32_data, fmt_chunk};
use common::{Workspace, assert_output, mono, run_mix, stderr, wav};
use std::fs;

#[test]
fn mixes_overlap_and_gain_with_hand_computed_samples() {
    let workspace = Workspace::new();
    let a = mono(&workspace, "a.wav", &[0.25, 0.5, -0.25, 0.0]);
    let b = mono(&workspace, "b.wav", &[0.5, -0.25, 0.75]);
    let output = workspace.path("mix.wav");
    let result = run_mix(
        &workspace,
        &output,
        "3",
        &[(&a, "0", "1.0"), (&b, "1", "0.5")],
    );
    assert!(result.status.success(), "stderr: {}", stderr(&result));
    assert_output(&output, 48_000, 1, &[0.25, 0.75, -0.375, 0.375]);
}

#[test]
fn preserves_offset_silence_and_saturates_overlapping_sources() {
    let workspace = Workspace::new();
    let a = mono(&workspace, "a.wav", &[0.25, -0.5]);
    let b = mono(&workspace, "b.wav", &[-0.25, 0.5]);
    let output = workspace.path("offset.wav");
    let result = run_mix(&workspace, &output, "2", &[(&a, "0", "1"), (&b, "4", "1")]);
    assert!(result.status.success(), "stderr: {}", stderr(&result));
    assert_output(&output, 48_000, 1, &[0.25, -0.5, 0.0, 0.0, -0.25, 0.5]);

    let positive = mono(&workspace, "positive.wav", &[0.75]);
    let negative = mono(&workspace, "negative.wav", &[-0.75]);
    let clipped = workspace.path("clipped.wav");
    let result = run_mix(
        &workspace,
        &clipped,
        "1",
        &[(&positive, "0", "1"), (&positive, "0", "1")],
    );
    assert!(result.status.success(), "stderr: {}", stderr(&result));
    assert_output(&clipped, 48_000, 1, &[1.0]);
    let clipped_negative = workspace.path("clipped-negative.wav");
    let result = run_mix(
        &workspace,
        &clipped_negative,
        "1",
        &[(&negative, "0", "1"), (&negative, "0", "1")],
    );
    assert!(result.status.success(), "stderr: {}", stderr(&result));
    assert_output(&clipped_negative, 48_000, 1, &[-1.0]);
}

#[test]
fn mixes_stereo_channels_and_is_block_size_invariant() {
    let workspace = Workspace::new();
    let a = workspace.write(
        "a.wav",
        &wav(&[fmt_chunk(2, 48_000), f32_data(&[0.25, -0.5, 0.5, -0.25])]),
    );
    let b = workspace.write(
        "b.wav",
        &wav(&[fmt_chunk(2, 48_000), f32_data(&[-0.25, 0.5, 0.75, -0.75])]),
    );
    let sources = [(a.as_path(), "0", "1"), (b.as_path(), "1", "0.5")];
    let expected = [0.25, -0.5, 0.375, 0.0, 0.375, -0.375];
    let mut outputs = Vec::new();
    for (name, block) in [("one.wav", "1"), ("two.wav", "2"), ("large.wav", "99")] {
        let output = workspace.path(name);
        let result = run_mix(&workspace, &output, block, &sources);
        assert!(result.status.success(), "stderr: {}", stderr(&result));
        assert_output(&output, 48_000, 2, &expected);
        outputs.push(fs::read(output).unwrap());
    }
    assert_eq!(outputs[0], outputs[1]);
    assert_eq!(outputs[1], outputs[2]);
}
