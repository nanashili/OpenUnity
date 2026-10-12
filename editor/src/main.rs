//! Native entry point for the independently specified OpenUnity editor.

use openunity_editor::app::{AppOptions, EditorApp, SmokeStatus, UnsavedChoice};
use openunity_editor::appearance::AppearanceProfile;
use openunity_editor_core::write_guard::WritePolicy;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct Arguments {
    project: Option<PathBuf>,
    scene: Option<PathBuf>,
    import_scene: Option<PathBuf>,
    bindings: Option<PathBuf>,
    smoke: bool,
    receipt: Option<PathBuf>,
    screenshot: Option<PathBuf>,
    appearance: Option<AppearanceProfile>,
}

fn arguments() -> Result<Option<Arguments>, String> {
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

fn output_policy(project: Option<&Path>) -> Result<WritePolicy, String> {
    let mut roots = Vec::new();
    if let Some(project) = project {
        roots.push(project.to_path_buf());
    }
    if cfg!(target_os = "macos") && Path::new("/Applications").is_dir() {
        roots.push(PathBuf::from("/Applications"));
    }
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        for path in [
            home.join(".local/share/openunity-research"),
            home.join("Library/Application Support/OpenUnityPrivateResearch"),
        ] {
            if path.is_dir() {
                roots.push(path);
            }
        }
    }
    WritePolicy::new(roots).map_err(|e| e.to_string())
}

fn write_new(policy: &WritePolicy, path: &Path, data: &[u8]) -> Result<(), String> {
    policy
        .atomic_write(path, data, |_| {
            Err("Smoke outputs require a new destination".into())
        })
        .map_err(|e| e.to_string())
}

fn run() -> Result<(), String> {
    let Some(args) = arguments()? else {
        return Ok(());
    };
    let output_policy = output_policy(args.project.as_deref())?;
    let status = args
        .smoke
        .then(|| Arc::new(Mutex::new(SmokeStatus::default())));
    let mut app = EditorApp::new(AppOptions {
        project_path: args.project,
        scene_path: args.scene,
        smoke: status.clone(),
    })?;
    app.set_appearance_profile(args.appearance.unwrap_or(AppearanceProfile::Reference));
    if let (Some(scene), Some(bindings)) = (&args.import_scene, &args.bindings) {
        app.request_import(scene, bindings)?;
        // Startup owns only the initial blank document; no interactive work exists yet.
        app.resolve_unsaved(UnsavedChoice::Discard)?;
    }
    if args.smoke {
        // Exercise the imported Inspector in native captures without editing scene data.
        let first_root = app
            .imported_document()
            .and_then(|d| d.roots().first().copied());
        if let Some(id) = first_root {
            app.apply_imported_action(
                openunity_editor::panels::imported::ImportedAction::Select {
                    id: Some(id),
                    toggle: false,
                },
            )?;
        }
    }
    for path in [&args.receipt, &args.screenshot].into_iter().flatten() {
        app.check_output_path(path)?;
    }
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "OpenUnity · Independent Authoring",
        options,
        Box::new(move |_creation| Ok(Box::new(app))),
    )
    .map_err(|e| e.to_string())?;
    if let Some(status) = status {
        let status = status
            .lock()
            .map_err(|_| "Native smoke status mutex was poisoned")?;
        let passed = status.completed && status.error.is_none() && status.screenshot_size.is_some();
        let receipt = serde_json::json!({
            "schema_version":1,"specification":if status.document_snapshot.as_ref().and_then(|s| s.get("format")).and_then(|v| v.as_str()) == Some("openunity.imported-scene") { "INFRA-EDITOR-SCENE-001" } else { "INFRA-EDITOR-001" },"verification_kind":"independent_native_frame",
            "os":std::env::consts::OS,"architecture":std::env::consts::ARCH,"renderer_requested":"Wgpu",
            "frames":status.frames,"screenshot_size":status.screenshot_size,"completed":status.completed,"error":status.error,
            "appearance_profile":status.appearance_profile,"workspace_snapshot":status.workspace_snapshot,"appearance_snapshot":status.appearance_snapshot,
            "document_snapshot":status.document_snapshot,
            "document_kind":if status.document_snapshot.as_ref().and_then(|s| s.get("format")).and_then(|v| v.as_str()) == Some("openunity.imported-scene") { "imported" } else { "authoring" },
            "unity_compatibility":"not_verified","status":if passed {"passed"} else {"failed"}
        });
        if let Some(path) = args.receipt {
            write_new(
                &output_policy,
                &path,
                &serde_json::to_vec_pretty(&receipt).map_err(|e| e.to_string())?,
            )?;
        }
        println!("{receipt}");
        if let Some(path) = args.screenshot {
            let image = status
                .screenshot_image
                .as_ref()
                .ok_or("Native renderer returned no screenshot")?;
            let mut bytes = Vec::new();
            {
                let mut encoder = png::Encoder::new(
                    &mut bytes,
                    u32::try_from(image.size[0]).map_err(|e| e.to_string())?,
                    u32::try_from(image.size[1]).map_err(|e| e.to_string())?,
                );
                encoder.set_color(png::ColorType::Rgba);
                encoder.set_depth(png::BitDepth::Eight);
                let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
                let rgba: Vec<_> = image
                    .pixels
                    .iter()
                    .flat_map(|pixel| pixel.to_array())
                    .collect();
                writer.write_image_data(&rgba).map_err(|e| e.to_string())?;
            }
            write_new(&output_policy, &path, &bytes)?;
        }
        if !passed {
            return Err(status
                .error
                .clone()
                .unwrap_or_else(|| "Native frame/screenshot smoke did not complete".into()));
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("OpenUnity editor: {error}");
            ExitCode::FAILURE
        }
    }
}
