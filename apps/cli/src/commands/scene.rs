//! Saved-scene import and untimed managed session execution.

use super::session;
use engine_scripting_bridge::ManagedHost;
use std::error::Error;
use std::path::Path;

pub(super) fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    let [
        _,
        root,
        fxr,
        bridge,
        config,
        assembly,
        project,
        scene,
        bindings,
        ticks @ ..,
    ] = args
    else {
        return Err("unsupported arguments; run without arguments for usage".into());
    };

    let plan =
        engine_scene::import_project(Path::new(project), Path::new(scene), Path::new(bindings))?;
    let payload = serde_json::to_vec(&plan)?;
    // SAFETY: explicit developer CLI paths supply trusted hostfxr and the
    // independently built bridge implementing the reviewed scene ABI.
    let host = unsafe {
        ManagedHost::load_scene(
            Path::new(root),
            Path::new(fxr),
            Path::new(bridge),
            Path::new(config),
        )
    }?;
    let script = host.create_scene(Path::new(assembly), &payload)?;
    let values = if ticks.is_empty() {
        vec![0, 0, 0]
    } else {
        ticks
            .iter()
            .map(|v| v.parse::<i64>())
            .collect::<Result<Vec<_>, _>>()?
    };
    let outputs = session::execute(script, 0, &values, 0)?;

    println!("{{\"mode\":\"managed-scene\",\"outputs\":{outputs:?}}}");
    Ok(())
}
