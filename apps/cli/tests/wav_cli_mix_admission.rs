//! Black-box input admission, output bounds, and no-clobber recovery acceptance.

#[path = "common/admission_support.rs"]
mod admission_support;
#[path = "common/mod.rs"]
mod common;

use admission_support::{MAX_INPUT_BYTES, assert_no_artifacts, i16_wav, run_raw};
use common::wav_fixtures::{f32_data, fmt_chunk};
use common::{Workspace, assert_output, mono, run_mix, stderr, wav};
use std::fs;
use std::path::PathBuf;

#[test]
fn rejects_i16_and_rate_or_channel_mismatches_before_output() {
    let workspace = Workspace::new();
    let good = mono(&workspace, "good.wav", &[0.25]);
    let i16 = workspace.write("i16.wav", &i16_wav());
    let rate = workspace.write("rate.wav", &wav(&[fmt_chunk(1, 44_100), f32_data(&[0.25])]));
    let stereo = workspace.write(
        "stereo.wav",
        &wav(&[fmt_chunk(2, 48_000), f32_data(&[0.25, 0.5])]),
    );
    for (name, source, words) in [
        ("i16-out.wav", &i16, "i16"),
        ("rate-out.wav", &rate, "format"),
        ("channel-out.wav", &stereo, "format"),
    ] {
        let output = workspace.path(name);
        let result = run_mix(
            &workspace,
            &output,
            "1",
            &[(&good, "0", "1"), (source, "0", "1")],
        );
        let message = stderr(&result);
        assert!(!result.status.success());
        assert!(message.contains(words), "stderr: {message}");
        assert_no_artifacts(&workspace, &[&good, &i16, &rate, &stereo], &output);
    }
}

#[test]
fn rejects_malformed_later_input_without_destination_or_temporary_file() {
    let workspace = Workspace::new();
    let good = mono(&workspace, "good.wav", &[0.25]);
    let bad = workspace.write("bad.wav", b"not a WAVE");
    let output = workspace.path("mix.wav");
    let result = run_mix(
        &workspace,
        &output,
        "1",
        &[(&good, "0", "1"), (&bad, "0", "1")],
    );
    let message = stderr(&result);
    assert!(!result.status.success());
    assert!(
        message.contains("bad.wav") && (message.contains("import") || message.contains("wave"))
    );
    assert_no_artifacts(&workspace, &[&good, &bad], &output);
}

#[test]
fn enforces_parser_input_and_output_bounds() {
    let workspace = Workspace::new();
    let source = mono(&workspace, "source.wav", &[0.25]);
    let output = workspace.path("out.wav");
    let result = run_mix(&workspace, &output, "0", &[(&source, "0", "1")]);
    assert!(!result.status.success());
    assert!(stderr(&result).contains("block"));
    assert_no_artifacts(&workspace, &[&source], &output);

    let result = run_raw(
        &workspace,
        &[
            "--mix-wav",
            output.to_str().unwrap(),
            "1",
            source.to_str().unwrap(),
            "0",
        ],
    );
    assert!(!result.status.success());
    assert!(stderr(&result).contains("usage"));
    assert_no_artifacts(&workspace, &[&source], &output);

    let too_many = vec![(source.as_path(), "0", "1"); 9];
    let result = run_mix(&workspace, &output, "1", &too_many);
    let message = stderr(&result);
    assert!(!result.status.success());
    assert!(message.contains("maximum is 8"), "stderr: {message}");
    assert_no_artifacts(&workspace, &[&source], &output);

    for gain in ["-1", "NaN", "inf"] {
        let result = run_mix(&workspace, &output, "1", &[(&source, "0", gain)]);
        assert!(!result.status.success());
        assert!(stderr(&result).contains("gain"));
        assert_no_artifacts(&workspace, &[&source], &output);
    }

    let result = run_mix(&workspace, &output, "1", &[(&source, "not-a-frame", "1")]);
    assert!(!result.status.success());
    assert!(stderr(&result).contains("frame"));
    assert_no_artifacts(&workspace, &[&source], &output);
}

#[test]
fn enforces_per_file_aggregate_and_frame_bounds_but_accepts_exact_bound() {
    let workspace = Workspace::new();
    let oversized = workspace.write("oversized.wav", &vec![0u8; MAX_INPUT_BYTES + 1]);
    let output = workspace.path("oversized-out.wav");
    let result = run_mix(&workspace, &output, "1", &[(&oversized, "0", "1")]);
    assert!(!result.status.success());
    assert!(stderr(&result).contains("16 mib") || stderr(&result).contains("input"));
    assert_no_artifacts(&workspace, &[&oversized], &output);

    let source = mono(&workspace, "small.wav", &[0.25]);
    for start in ["18446744073709551615", "1048576"] {
        let output = workspace.path("frame-error.wav");
        let result = run_mix(&workspace, &output, "1", &[(&source, start, "1")]);
        assert!(!result.status.success());
        assert!(stderr(&result).contains("frame") || stderr(&result).contains("limit"));
        assert_no_artifacts(&workspace, &[&oversized, &source], &output);
    }
    let exact = workspace.path("exact-bound.wav");
    let result = run_mix(&workspace, &exact, "4096", &[(&source, "1048575", "1")]);
    assert!(result.status.success(), "stderr: {}", stderr(&result));
    assert_output(
        &exact,
        48_000,
        1,
        &vec![0.0; 1_048_575]
            .into_iter()
            .chain([0.25])
            .collect::<Vec<_>>(),
    );
}

#[test]
fn rejects_aggregate_input_above_sixty_four_mib() {
    let workspace = Workspace::new();
    let samples = vec![0.0; 2_097_152];
    let bytes = wav(&[fmt_chunk(2, 48_000), f32_data(&samples)]);
    let mut paths = Vec::new();
    for index in 0..8 {
        paths.push(workspace.write(&format!("source-{index}.wav"), &bytes));
    }
    let tuples: Vec<_> = paths
        .iter()
        .map(|path| (path.as_path(), "0", "1"))
        .collect();
    let output = workspace.path("aggregate-out.wav");
    let result = run_mix(&workspace, &output, "4096", &tuples);
    assert!(!result.status.success());
    assert!(stderr(&result).contains("64 mib") || stderr(&result).contains("aggregate"));
    assert_no_artifacts(
        &workspace,
        &paths.iter().map(PathBuf::as_path).collect::<Vec<_>>(),
        &output,
    );
}

#[test]
fn preserves_existing_destination_and_reports_reimportable_recovery() {
    let workspace = Workspace::new();
    let source = mono(&workspace, "source.wav", &[0.25, -0.5]);
    let output = workspace.path("existing.wav");
    let original = b"preserve this destination";
    fs::write(&output, original).unwrap();
    let result = run_mix(&workspace, &output, "1", &[(&source, "0", "1")]);
    assert!(!result.status.success());
    assert_eq!(fs::read(&output).unwrap(), original);
    let recovery = fs::read_dir(&workspace.root)
        .expect("read recovery workspace")
        .map(|entry| entry.expect("read recovery entry").path())
        .find(|path| path != &source && path != &output && path.is_file())
        .expect("complete recovery file");
    let message = stderr(&result);
    let raw_message = String::from_utf8_lossy(&result.stderr);
    assert!(message.contains("recovery"));
    assert!(
        raw_message.contains(recovery.to_str().expect("recovery path is UTF-8")),
        "stderr: {raw_message}"
    );
    assert_output(&recovery, 48_000, 1, &[0.25, -0.5]);
}
