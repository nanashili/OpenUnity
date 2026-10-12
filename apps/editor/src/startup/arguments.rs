//! Parse explicit startup options before any filesystem or renderer work.

use std::path::PathBuf;

use openunity_editor::appearance::AppearanceProfile;

use super::Arguments;

pub(super) fn arguments() -> Result<Option<Arguments>, String> {
    let mut args = std::env::args().skip(1);
    let mut parsed = Arguments::default();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                println!(
                    "OpenUnity independent authoring editor\nUsage: openunity-editor [--appearance reference|legacy] [--project PATH] [--scene PATH.ouscene|PATH.ouimport] [--import-scene SCENE --bindings BINDINGS] [--smoke [--smoke-receipt NEW.json] [--smoke-screenshot NEW.png]]\nNative startup uses the reference appearance. Bounded saved-scene import requires --project and explicit bindings. General Unity import, Play, C# compilation and builds are unsupported."
                );
                return Ok(None);
            }
            "--project" => {
                parsed.project = Some(PathBuf::from(
                    args.next().ok_or("--project requires a path")?,
                ))
            }
            "--scene" => {
                parsed.scene = Some(PathBuf::from(args.next().ok_or("--scene requires a path")?))
            }
            "--import-scene" => {
                parsed.import_scene = Some(PathBuf::from(
                    args.next().ok_or("--import-scene requires a path")?,
                ))
            }
            "--bindings" => {
                parsed.bindings = Some(PathBuf::from(
                    args.next().ok_or("--bindings requires a path")?,
                ))
            }
            "--smoke" => parsed.smoke = true,
            "--appearance" => {
                parsed.appearance = Some(match args.next().as_deref() {
                    Some("legacy") => AppearanceProfile::Legacy,
                    Some("reference") => AppearanceProfile::Reference,
                    _ => return Err("--appearance requires legacy or reference".into()),
                });
            }
            "--smoke-receipt" => {
                parsed.receipt = Some(PathBuf::from(
                    args.next().ok_or("--smoke-receipt requires a new path")?,
                ))
            }
            "--smoke-screenshot" => {
                parsed.screenshot = Some(PathBuf::from(
                    args.next()
                        .ok_or("--smoke-screenshot requires a new path")?,
                ))
            }
            _ => return Err(format!("Unknown argument: {arg}")),
        }
    }
    if !parsed.smoke && (parsed.receipt.is_some() || parsed.screenshot.is_some()) {
        return Err("Smoke output paths require --smoke".into());
    }
    if parsed.import_scene.is_some() != parsed.bindings.is_some() {
        return Err("--import-scene and --bindings must be supplied together".into());
    }
    if parsed.import_scene.is_some() && (parsed.project.is_none() || parsed.scene.is_some()) {
        return Err("--import-scene requires --project and cannot be combined with --scene".into());
    }
    Ok(Some(parsed))
}
