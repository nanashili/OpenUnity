//! Black-box WAVE export, bounded input, and no-clobber recovery acceptance.

use engine_audio::wav::import_wav;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_INPUT_BYTES: usize = 16 * 1024 * 1024;

struct Chunk {
    id: [u8; 4],
    payload: Vec<u8>,
}

struct TestWorkspace {
    root: PathBuf,
}

impl TestWorkspace {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock must be after the Unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("openunity-wav-cli-{nonce}"));
        fs::create_dir(&root).expect("create unique test workspace");
        Self { root }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }
}

impl Drop for TestWorkspace {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.root) {
            if std::thread::panicking() {
                eprintln!(
                    "failed to remove test workspace {} while unwinding: {error}",
                    self.root.display()
                );
            } else {
                panic!(
                    "failed to remove test workspace {}: {error}",
                    self.root.display()
                );
            }
        }
    }
}

fn run_export(workspace: &TestWorkspace, input: &Path, output: &Path, block_frames: u32) -> Output {
    Command::new(env!("CARGO_BIN_EXE_openunity-cli"))
        .current_dir(&workspace.root)
        .args([
            "--export-wav",
            input.to_str().expect("input path is valid UTF-8"),
            output.to_str().expect("output path is valid UTF-8"),
            &block_frames.to_string(),
        ])
        .output()
        .expect("run openunity-cli")
}

fn write_input(workspace: &TestWorkspace, bytes: &[u8]) -> PathBuf {
    let input = workspace.path("input.wav");
    fs::write(&input, bytes).expect("write input WAVE");
    input
}

fn wav(chunks: &[Chunk]) -> Vec<u8> {
    let body_len = 4 + chunks.iter().map(encoded_len).sum::<usize>();
    let mut bytes = Vec::with_capacity(8 + body_len);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(body_len as u32).to_le_bytes());
    bytes.extend_from_slice(b"WAVE");

    for chunk in chunks {
        bytes.extend_from_slice(&chunk.id);
        bytes.extend_from_slice(&(chunk.payload.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&chunk.payload);
        if chunk.payload.len() % 2 == 1 {
            bytes.push(0);
        }
    }
    bytes
}

fn encoded_len(chunk: &Chunk) -> usize {
    8 + chunk.payload.len() + chunk.payload.len() % 2
}

fn fmt_chunk(tag: u16, channels: u16, rate: u32, bits: u16) -> Chunk {
    let block_align = channels * (bits / 8);
    let byte_rate = rate * u32::from(block_align);
    let mut payload = Vec::with_capacity(16);
    payload.extend_from_slice(&tag.to_le_bytes());
    payload.extend_from_slice(&channels.to_le_bytes());
    payload.extend_from_slice(&rate.to_le_bytes());
    payload.extend_from_slice(&byte_rate.to_le_bytes());
    payload.extend_from_slice(&block_align.to_le_bytes());
    payload.extend_from_slice(&bits.to_le_bytes());
    Chunk {
        id: *b"fmt ",
        payload,
    }
}

fn f32_data(samples: &[f32]) -> Chunk {
    let mut payload = Vec::with_capacity(samples.len() * 4);
    for sample in samples {
        payload.extend_from_slice(&sample.to_bits().to_le_bytes());
    }
    Chunk {
        id: *b"data",
        payload,
    }
}

fn i16_data(samples: &[i16]) -> Chunk {
    let mut payload = Vec::with_capacity(samples.len() * 2);
    for sample in samples {
        payload.extend_from_slice(&sample.to_le_bytes());
    }
    Chunk {
        id: *b"data",
        payload,
    }
}

fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap())
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

fn stderr_text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn assert_no_export_files(workspace: &TestWorkspace, input: &Path) {
    let files: Vec<_> = fs::read_dir(&workspace.root)
        .expect("read test workspace")
        .map(|entry| entry.expect("read workspace entry").path())
        .filter(|path| path != input)
        .collect();
    assert!(files.is_empty(), "unexpected export artifacts: {files:?}");
}

fn recovery_file(workspace: &TestWorkspace, input: &Path, destination: &Path) -> PathBuf {
    let files: Vec<_> = fs::read_dir(&workspace.root)
        .expect("read recovery workspace")
        .map(|entry| entry.expect("read recovery entry").path())
        .filter(|path| path != input && path != destination)
        .filter(|path| path.is_file())
        .collect();
    assert_eq!(files.len(), 1, "expected one complete recovery file");
    files.into_iter().next().unwrap()
}

fn assert_reported_recovery(stderr: &str, recovery: &Path) {
    assert!(
        stderr.to_ascii_lowercase().contains("recovery"),
        "publication failure must identify recovery output: {stderr}"
    );
    assert!(
        stderr.contains(recovery.to_str().expect("recovery path is UTF-8")),
        "publication failure must report the exact recovery path: {stderr}"
    );
}

fn assert_reimports_exact(path: &Path, rate: u32, channels: u16, samples: &[f32]) {
    let bytes = fs::read(path).expect("read exported WAVE");
    let clip = import_wav(&bytes).expect("exported WAVE re-imports");
    assert_eq!(clip.sample_rate_hz(), rate);
    assert_eq!(clip.channel_count(), channels);
    assert_eq!(clip.frame_count(), samples.len() / usize::from(channels));
    assert_eq!(clip.f32_samples(), Some(samples));
}

#[test]
fn exports_mono_f32_with_exact_riff_header_and_samples() {
    let workspace = TestWorkspace::new();
    let samples = [0.25, -0.5, 1.0, -1.0];
    let input = write_input(
        &workspace,
        &wav(&[fmt_chunk(3, 1, 48_000, 32), f32_data(&samples)]),
    );
    let destination = workspace.path("mono-export.wav");

    let output = run_export(&workspace, &input, &destination, 3);
    assert!(output.status.success(), "stderr: {}", stderr_text(&output));

    let bytes = fs::read(&destination).expect("read mono export");
    assert_eq!(&bytes[0..4], b"RIFF");
    assert_eq!(read_u32(&bytes, 4), 52);
    assert_eq!(&bytes[8..12], b"WAVE");
    assert_eq!(&bytes[12..16], b"fmt ");
    assert_eq!(read_u32(&bytes, 16), 16);
    assert_eq!(read_u16(&bytes, 20), 3);
    assert_eq!(read_u16(&bytes, 22), 1);
    assert_eq!(read_u32(&bytes, 24), 48_000);
    assert_eq!(read_u32(&bytes, 28), 192_000);
    assert_eq!(read_u16(&bytes, 32), 4);
    assert_eq!(read_u16(&bytes, 34), 32);
    assert_eq!(&bytes[36..40], b"data");
    assert_eq!(read_u32(&bytes, 40), 16);
    assert_eq!(
        &bytes[44..],
        &samples
            .iter()
            .flat_map(|s| s.to_le_bytes())
            .collect::<Vec<_>>()
    );
    assert_reimports_exact(&destination, 48_000, 1, &samples);
}

#[test]
fn exports_to_bare_relative_destination_from_workspace() {
    let workspace = TestWorkspace::new();
    let samples = [0.25, -0.5];
    let input = write_input(
        &workspace,
        &wav(&[fmt_chunk(3, 1, 48_000, 32), f32_data(&samples)]),
    );
    let destination = PathBuf::from("relative-export.wav");
    let published = workspace.path(destination.to_str().expect("relative path is UTF-8"));

    let output = run_export(&workspace, &input, &destination, 1);
    assert!(output.status.success(), "stderr: {}", stderr_text(&output));
    assert_reimports_exact(&published, 48_000, 1, &samples);
}

#[test]
fn exports_stereo_f32_in_frame_major_order() {
    let workspace = TestWorkspace::new();
    let samples = [0.25, -0.25, 0.5, -0.5];
    let input = write_input(
        &workspace,
        &wav(&[fmt_chunk(3, 2, 22_050, 32), f32_data(&samples)]),
    );
    let destination = workspace.path("stereo-export.wav");

    let output = run_export(&workspace, &input, &destination, 1);
    assert!(output.status.success(), "stderr: {}", stderr_text(&output));
    assert_reimports_exact(&destination, 22_050, 2, &samples);
}

#[test]
fn rejects_pcm16_without_creating_destination_or_temporary_files() {
    let workspace = TestWorkspace::new();
    let input = write_input(
        &workspace,
        &wav(&[fmt_chunk(1, 1, 8_000, 16), i16_data(&[1234])]),
    );
    let destination = workspace.path("pcm16-export.wav");

    let output = run_export(&workspace, &input, &destination, 1);
    let stderr = stderr_text(&output).to_ascii_lowercase();
    assert!(!output.status.success());
    assert!(
        stderr.contains("unsupported") && stderr.contains("i16"),
        "stderr: {stderr}"
    );
    assert!(!destination.exists());
    assert_no_export_files(&workspace, &input);
}

#[test]
fn rejects_malformed_input_without_creating_destination_or_temporary_files() {
    let workspace = TestWorkspace::new();
    let input = write_input(&workspace, b"not a wave");
    let destination = workspace.path("malformed-export.wav");

    let output = run_export(&workspace, &input, &destination, 1);
    let stderr = stderr_text(&output).to_ascii_lowercase();
    assert!(!output.status.success());
    assert!(
        stderr.contains("import") || stderr.contains("wave"),
        "stderr: {stderr}"
    );
    assert!(!destination.exists());
    assert_no_export_files(&workspace, &input);
}

#[test]
fn rejects_input_over_sixteen_mib_before_creating_output() {
    let workspace = TestWorkspace::new();
    let input = write_input(&workspace, &vec![0u8; MAX_INPUT_BYTES + 1]);
    let destination = workspace.path("oversized-export.wav");

    let output = run_export(&workspace, &input, &destination, 1);
    let stderr = stderr_text(&output).to_ascii_lowercase();
    assert!(!output.status.success());
    assert!(
        stderr.contains("input") && (stderr.contains("16 mib") || stderr.contains("exceeds")),
        "stderr: {stderr}"
    );
    assert!(!destination.exists());
    assert_no_export_files(&workspace, &input);
}

#[test]
fn rejects_zero_block_size_without_creating_output() {
    let workspace = TestWorkspace::new();
    let input = write_input(
        &workspace,
        &wav(&[fmt_chunk(3, 1, 48_000, 32), f32_data(&[0.25])]),
    );
    let destination = workspace.path("zero-block-export.wav");

    let output = run_export(&workspace, &input, &destination, 0);
    let stderr = stderr_text(&output).to_ascii_lowercase();
    assert!(!output.status.success());
    assert!(
        stderr.contains("zero") && stderr.contains("block"),
        "stderr: {stderr}"
    );
    assert!(!destination.exists());
    assert_no_export_files(&workspace, &input);
}

#[test]
fn preserves_existing_destination_and_reports_complete_recovery_file() {
    let workspace = TestWorkspace::new();
    let samples = [0.25, -0.5];
    let input = write_input(
        &workspace,
        &wav(&[fmt_chunk(3, 1, 48_000, 32), f32_data(&samples)]),
    );
    let destination = workspace.path("existing-export.wav");
    let original = b"keep these destination bytes";
    fs::write(&destination, original).unwrap();

    let output = run_export(&workspace, &input, &destination, 1);
    assert!(!output.status.success());
    assert_eq!(fs::read(&destination).unwrap(), original);

    let recovery = recovery_file(&workspace, &input, &destination);
    assert_reported_recovery(&stderr_text(&output), &recovery);
    assert_reimports_exact(&recovery, 48_000, 1, &samples);
}

#[test]
fn preserves_existing_directory_and_reports_complete_recovery_file() {
    let workspace = TestWorkspace::new();
    let samples = [0.5, -0.5];
    let input = write_input(
        &workspace,
        &wav(&[fmt_chunk(3, 1, 48_000, 32), f32_data(&samples)]),
    );
    let destination = workspace.path("existing-directory");
    fs::create_dir(&destination).unwrap();
    let marker = destination.join("marker");
    fs::write(&marker, b"directory state").unwrap();

    let output = run_export(&workspace, &input, &destination, 1);
    assert!(!output.status.success());
    assert!(destination.is_dir());
    assert_eq!(fs::read(&marker).unwrap(), b"directory state");

    let recovery = recovery_file(&workspace, &input, &destination);
    assert_reported_recovery(&stderr_text(&output), &recovery);
    assert_reimports_exact(&recovery, 48_000, 1, &samples);
}
