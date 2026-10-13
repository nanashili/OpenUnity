//! Usage and the standalone native identity diagnostic.

use engine_runtime::{NATIVE_ABI_VERSION, Runtime};
use std::error::Error;

pub(super) fn print_usage() {
    println!(
        "OpenUnity runtime foundation\nUsage: openunity-player --self-check\n       openunity-player --run-script DOTNET_ROOT HOSTFXR BRIDGE CONFIG SCRIPT TYPE INITIAL [TICK ...] SHUTDOWN\n       openunity-player --run-gameplay DOTNET_ROOT HOSTFXR BRIDGE CONFIG SCRIPT TYPE INITIAL [TICK ...] SHUTDOWN\n       openunity-player --run-scene DOTNET_ROOT HOSTFXR BRIDGE CONFIG ASSEMBLY PROJECT SCENE BINDINGS [TICKS...]\n       openunity-player --run-timed-scene DOTNET_ROOT HOSTFXR BRIDGE CONFIG ASSEMBLY PROJECT SCENE BINDINGS SOURCE STEP [TICKS...]\n       openunity-cli --export-wav INPUT.wav OUTPUT.wav BLOCK_FRAMES\nGameplay and saved scenes support the reviewed authored subset. Rendering remains unsupported."
    );
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
