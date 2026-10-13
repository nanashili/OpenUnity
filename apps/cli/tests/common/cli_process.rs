use std::path::Path;
use std::process::{Command, Output};

use super::workspace::Workspace;

pub(crate) fn run_mix(
    workspace: &Workspace,
    output: &Path,
    block: &str,
    sources: &[(&Path, &str, &str)],
) -> Output {
    let mut args = vec![
        "--mix-wav".to_owned(),
        output.to_str().expect("output path is UTF-8").to_owned(),
        block.to_owned(),
    ];

    for (path, start, gain) in sources {
        args.push(path.to_str().expect("source path is UTF-8").to_owned());
        args.push((*start).to_owned());
        args.push((*gain).to_owned());
    }

    Command::new(env!("CARGO_BIN_EXE_openunity-cli"))
        .current_dir(&workspace.root)
        .args(args)
        .output()
        .expect("run openunity-cli")
}

pub(crate) fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_ascii_lowercase()
}
