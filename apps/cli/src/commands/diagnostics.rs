//! Usage and the standalone native identity diagnostic.

use engine_runtime::{NATIVE_ABI_VERSION, Runtime};
use std::error::Error;

const USAGE_TEXT: &str = concat!(
    "OpenUnity runtime foundation\n",
    "Usage: openunity-player --help | -h\n",
    "       openunity-player --self-check\n",
    "       openunity-player --run-script DOTNET_ROOT HOSTFXR BRIDGE CONFIG SCRIPT TYPE INITIAL [TICK ...] SHUTDOWN\n",
    "       openunity-player --run-gameplay DOTNET_ROOT HOSTFXR BRIDGE CONFIG SCRIPT TYPE INITIAL [TICK ...] SHUTDOWN\n",
    "       openunity-player --run-scene DOTNET_ROOT HOSTFXR BRIDGE CONFIG ASSEMBLY PROJECT SCENE BINDINGS [TICKS...]\n",
    "       openunity-player --run-timed-scene DOTNET_ROOT HOSTFXR BRIDGE CONFIG ASSEMBLY PROJECT SCENE BINDINGS SOURCE STEP [TICKS...]\n",
    "       openunity-player --render-scene DOTNET_ROOT HOSTFXR BRIDGE CONFIG ASSEMBLY PROJECT SCENE BINDINGS MANIFEST WIDTH HEIGHT {hardware|software} OUTPUT [TICK ...]\n",
    "       openunity-player --physics-scene <2d|3d> <steps:60..600> WIDTH HEIGHT {hardware|software} OUTPUT\n",
    "       openunity-player --live-physics-scene <2d|3d> {hardware|software} [--frames COUNT]\n",
    "       openunity-cli --export-wav INPUT.wav OUTPUT.wav BLOCK_FRAMES\n",
    "       openunity-cli --mix-wav OUTPUT.wav BLOCK_FRAMES INPUT.wav START_FRAME GAIN [INPUT.wav START_FRAME GAIN ...]\n",
    "Gameplay and saved scenes support their reviewed authored subsets. ",
    "--render-scene is a bounded offscreen route and does not imply Unity rendering parity. ",
    "Native physics routes use the current Rapier backend; Unity physics parity remains unverified. ",
    "--live-physics-scene opens a native window; COUNT limits present requests, not GPU completion. ",
    "The render route accepts at most 255 signed tick values and initializes its scene with 0.",
);

pub(super) fn print_usage() {
    println!("{USAGE_TEXT}");
}

pub(super) fn self_check() -> Result<(), Box<dyn Error>> {
    let mut runtime = Runtime::default();
    let object = runtime.create_object()?;
    let live_before = runtime.is_alive(object);
    let destroyed = runtime.destroy_object(object);
    let live_after = runtime.is_alive(object);
    if !live_before || !destroyed || live_after {
        return Err("runtime identity self-check failed".into());
    }
    println!(
        "{{\"mode\":\"identity-self-check\",\"abi_version\":{},\"live_before\":{},\"destroyed\":{},\"live_after\":{}}}",
        NATIVE_ABI_VERSION, live_before, destroyed, live_after
    );
    Ok(())
}
