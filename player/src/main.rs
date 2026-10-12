//! Headless native runtime and managed gameplay host.

use openunity_runtime::Runtime;
use openunity_scripting_host::ManagedHost;
use std::error::Error;
use std::path::Path;
use std::process::ExitCode;

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [] => {
            println!(
                "OpenUnity runtime foundation\nUsage: openunity-player --self-check\n       openunity-player --run-script DOTNET_ROOT HOSTFXR BRIDGE CONFIG SCRIPT TYPE INITIAL [TICK ...] SHUTDOWN\n       openunity-player --run-gameplay DOTNET_ROOT HOSTFXR BRIDGE CONFIG SCRIPT TYPE INITIAL [TICK ...] SHUTDOWN\n       openunity-player --run-scene DOTNET_ROOT HOSTFXR BRIDGE CONFIG ASSEMBLY PROJECT SCENE BINDINGS [TICKS...]\n       openunity-player --run-timed-scene DOTNET_ROOT HOSTFXR BRIDGE CONFIG ASSEMBLY PROJECT SCENE BINDINGS SOURCE STEP [TICKS...]\nGameplay and saved scenes support the reviewed authored subset. Rendering remains unsupported."
            );
        }
        [arg] if arg == "--self-check" => {
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
                openunity_engine::ABI_VERSION,
                live_before,
                destroyed,
                live_after
            );
        }
        [
            flag,
            root,
            fxr,
            bridge,
            config,
            assembly,
            project,
            scene,
            bindings,
            ticks @ ..,
        ] if flag == "--run-scene" => {
            let plan = openunity_scene::import_project(
                Path::new(project),
                Path::new(scene),
                Path::new(bindings),
            )?;
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
            let mut session = host.create_scene(Path::new(assembly), &payload)?;
            let values = if ticks.is_empty() {
                vec![0, 0, 0]
            } else {
                ticks
                    .iter()
                    .map(|v| v.parse::<i64>())
                    .collect::<Result<Vec<_>, _>>()?
            };
            let mut outputs = vec![session.initialize(0)?];
            for tick in values {
                outputs.push(session.tick(tick)?);
            }
            outputs.push(session.shutdown(0)?);
            session.close()?;
            println!("{{\"mode\":\"managed-scene\",\"outputs\":{outputs:?}}}");
        }
        [
            flag,
            root,
            fxr,
            bridge,
            config,
            assembly,
            project,
            scene,
            bindings,
            source,
            step,
            ticks @ ..,
        ] if flag == "--run-timed-scene" => {
            let source = match source.as_str() {
                "wall" => 0,
                "explicit" => 1,
                _ => return Err("timed source must be wall or explicit".into()),
            };
            let step = step.parse::<f64>()?;
            openunity_scripting_host::validate_time_source(source, step)?;
            let values = if ticks.is_empty() {
                vec![0, 0, 0]
            } else {
                ticks
                    .iter()
                    .map(|value| value.parse::<i64>())
                    .collect::<Result<Vec<_>, _>>()?
            };
            let plan = openunity_scene::import_project(
                Path::new(project),
                Path::new(scene),
                Path::new(bindings),
            )?;
            let payload = serde_json::to_vec(&plan)?;
            // SAFETY: explicit developer paths must implement the reviewed trusted
            // hostfxr and INFRA-TIME-001 managed timed-scene signatures.
            let host = unsafe {
                ManagedHost::load_timed_scene(
                    Path::new(root),
                    Path::new(fxr),
                    Path::new(bridge),
                    Path::new(config),
                )
            }?;
            let mut session =
                host.create_timed_scene(Path::new(assembly), &payload, source, step)?;
            let execution = (|| -> Result<Vec<i64>, Box<dyn Error>> {
                let mut outputs = vec![session.initialize(0)?];
                for value in values {
                    outputs.push(session.tick(value)?);
                }
                outputs.push(session.shutdown(0)?);
                Ok(outputs)
            })();
            let cleanup = session.close();
            let outputs = match (execution, cleanup) {
                (Ok(outputs), Ok(())) => outputs,
                (Err(failure), Ok(())) => return Err(failure),
                (Ok(_), Err(cleanup)) => return Err(cleanup.into()),
                (Err(failure), Err(cleanup)) => {
                    return Err(format!(
                        "{failure}; managed session cleanup also failed: {cleanup}"
                    )
                    .into());
                }
            };
            println!("{{\"mode\":\"managed-timed-scene\",\"outputs\":{outputs:?}}}");
        }
        [flag, root, fxr, bridge, config, assembly, name, values @ ..]
            if matches!(flag.as_str(), "--run-script" | "--run-gameplay") && values.len() >= 2 =>
        {
            let values: Vec<i64> = values.iter().map(|v| v.parse()).collect::<Result<_, _>>()?;
            let gameplay = flag == "--run-gameplay";
            // SAFETY: this developer CLI explicitly accepts the trusted SDK hostfxr
            // and independently built OpenUnity.ManagedHosting assembly. Its paths
            // must implement the reviewed host signatures, including the gameplay
            // extension for that route; arbitrary binaries are unsafe.
            let host = unsafe {
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
            }?;
            let mut script = if gameplay {
                host.create_gameplay(Path::new(assembly), name)?
            } else {
                host.create_script(Path::new(assembly), name)?
            };
            let mut outputs = vec![script.initialize(values[0])?];
            for value in &values[1..values.len() - 1] {
                outputs.push(script.tick(*value)?);
            }
            outputs.push(script.shutdown(*values.last().expect("at least two values"))?);
            script.close()?;
            let mode = if gameplay {
                "managed-gameplay"
            } else {
                "managed-script"
            };
            println!("{{\"mode\":\"{mode}\",\"outputs\":{outputs:?}}}");
        }
        _ => return Err("unsupported arguments; run without arguments for usage".into()),
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("openunity-player: {error}");
            ExitCode::FAILURE
        }
    }
}
