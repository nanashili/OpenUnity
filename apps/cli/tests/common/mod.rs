pub(crate) mod cli_process;
pub(crate) mod wav_fixtures;
pub(crate) mod wave_verification;
pub(crate) mod workspace;

pub(crate) use cli_process::{run_mix, stderr};
pub(crate) use wav_fixtures::{mono, wav};
pub(crate) use wave_verification::assert_output;
pub(crate) use workspace::Workspace;
