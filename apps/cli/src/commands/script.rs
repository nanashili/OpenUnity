//! Plain hosted scripts and supported MonoBehaviour gameplay sessions.

use super::session;
use engine_scripting_bridge::{HostError, ManagedHost};
use std::error::Error;
use std::path::Path;

pub(super) fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    let [flag, root, fxr, bridge, config, assembly, name, values @ ..] = args else {
        return Err("unsupported arguments; run without arguments for usage".into());
    };
    if values.len() < 2 {
        return Err("unsupported arguments; run without arguments for usage".into());
    }

    let values: Vec<i64> = values.iter().map(|v| v.parse()).collect::<Result<_, _>>()?;
    let gameplay = flag == "--run-gameplay";
    let host = load_host(root, fxr, bridge, config, gameplay)?;

    let script = if gameplay {
        host.create_gameplay(Path::new(assembly), name)?
    } else {
        host.create_script(Path::new(assembly), name)?
    };
    let outputs = session::execute(
        script,
        values[0],
        &values[1..values.len() - 1],
        *values.last().expect("at least two values"),
    )?;

    let mode = if gameplay {
        "managed-gameplay"
    } else {
        "managed-script"
    };
    println!("{{\"mode\":\"{mode}\",\"outputs\":{outputs:?}}}");
    Ok(())
}

fn load_host(
    root: &str,
    fxr: &str,
    bridge: &str,
    config: &str,
    gameplay: bool,
) -> Result<ManagedHost, HostError> {
    // SAFETY: this developer CLI explicitly accepts the trusted SDK hostfxr
    // and independently built OpenUnity.ManagedHosting assembly. Its paths
    // must implement the reviewed host signatures, including the gameplay
    // extension for that route; arbitrary binaries are unsafe.
    unsafe {
        if gameplay {
            ManagedHost::load_gameplay(
                Path::new(root),
                Path::new(fxr),
                Path::new(bridge),
                Path::new(config),
            )
        } else {
            ManagedHost::load(
                Path::new(root),
                Path::new(fxr),
                Path::new(bridge),
                Path::new(config),
            )
        }
    }
}
