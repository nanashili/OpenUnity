use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use crate::common::Workspace;
use crate::common::wav_fixtures::{Chunk, fmt_chunk, wav};

pub(crate) const MAX_INPUT_BYTES: usize = 16 * 1024 * 1024;

pub(crate) fn run_raw(workspace: &Workspace, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_openunity-cli"))
        .current_dir(&workspace.root)
        .args(args)
        .output()
        .expect("run openunity-cli")
}

pub(crate) fn i16_wav() -> Vec<u8> {
    let mut format = fmt_chunk(1, 48_000).payload;
    format[0..2].copy_from_slice(&1u16.to_le_bytes());
    format[14..16].copy_from_slice(&16u16.to_le_bytes());
    format[8..12].copy_from_slice(&96_000u32.to_le_bytes());
    format[12..14].copy_from_slice(&2u16.to_le_bytes());

    wav(&[
        Chunk {
            id: *b"fmt ",
            payload: format,
        },
        Chunk {
            id: *b"data",
            payload: Vec::from(1234i16.to_le_bytes()),
        },
    ])
}

pub(crate) fn assert_no_artifacts(workspace: &Workspace, inputs: &[&Path], output: &Path) {
    let extras: Vec<_> = fs::read_dir(&workspace.root)
        .expect("read workspace")
        .map(|entry| entry.expect("read workspace entry").path())
        .filter(|path| !inputs.iter().any(|input| *input == path))
        .filter(|path| path != output)
        .collect();

    assert!(extras.is_empty(), "unexpected output artifacts: {extras:?}");
    assert!(!output.exists());
}
