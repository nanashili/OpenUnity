//! Explicit or wall-clock scene execution and ordered teardown.

use engine_scripting_bridge::{ManagedHost, ScriptSession};
use std::error::Error;
use std::path::Path;

struct TimedSceneArguments<'a> {
    root: &'a Path,
    hostfxr: &'a Path,
    bridge: &'a Path,
    config: &'a Path,
    assembly: &'a Path,
    project: &'a Path,
    scene: &'a Path,
    bindings: &'a Path,
    source: i32,
    step: f64,
    ticks: Vec<i64>,
}

impl<'a> TimedSceneArguments<'a> {
    fn parse(args: &'a [String]) -> Result<Self, Box<dyn Error>> {
        let [
            _,
            root,
            hostfxr,
            bridge,
            config,
            assembly,
            project,
            scene,
            bindings,
            source,
            step,
            ticks @ ..,
        ] = args
        else {
            return Err("timed scene requires host paths, scene paths, source and step".into());
        };
        let source = match source.as_str() {
            "wall" => 0,
            "explicit" => 1,
            _ => return Err("timed source must be wall or explicit".into()),
        };
        let step = step.parse::<f64>()?;
        engine_scripting_bridge::validate_time_source(source, step)?;
        let ticks = if ticks.is_empty() {
            vec![0, 0, 0]
        } else {
            ticks
                .iter()
                .map(|value| value.parse::<i64>())
                .collect::<Result<Vec<_>, _>>()?
        };
        Ok(Self {
            root: Path::new(root),
            hostfxr: Path::new(hostfxr),
            bridge: Path::new(bridge),
            config: Path::new(config),
            assembly: Path::new(assembly),
            project: Path::new(project),
            scene: Path::new(scene),
            bindings: Path::new(bindings),
            source,
            step,
            ticks,
        })
    }
}

pub(super) fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    let args = TimedSceneArguments::parse(args)?;
    let plan = engine_scene::import_project(args.project, args.scene, args.bindings)?;
    let payload = engine_serialization::serialize_plan(&plan)?;
    // SAFETY: explicit developer paths must implement the reviewed trusted
    // hostfxr and INFRA-TIME-001 managed timed-scene signatures.
    let host = unsafe {
        ManagedHost::load_timed_scene(args.root, args.hostfxr, args.bridge, args.config)
    }?;
    let session = host.create_timed_scene(args.assembly, &payload, args.source, args.step)?;
    let outputs = execute_timed_session(session, &args.ticks)?;
    println!("{{\"mode\":\"managed-timed-scene\",\"outputs\":{outputs:?}}}");
    Ok(())
}

fn execute_timed_session(
    mut session: ScriptSession<'_>,
    ticks: &[i64],
) -> Result<Vec<i64>, Box<dyn Error>> {
    let execution = (|| -> Result<Vec<i64>, Box<dyn Error>> {
        let mut outputs = vec![session.initialize(0)?];
        for value in ticks {
            outputs.push(session.tick(*value)?);
        }
        outputs.push(session.shutdown(0)?);
        Ok(outputs)
    })();
    let cleanup = session.close();
    match (execution, cleanup) {
        (Ok(outputs), Ok(())) => Ok(outputs),
        (Err(failure), Ok(())) => Err(failure),
        (Ok(_), Err(cleanup)) => Err(cleanup.into()),
        (Err(failure), Err(cleanup)) => {
            Err(format!("{failure}; managed session cleanup also failed: {cleanup}").into())
        }
    }
}
