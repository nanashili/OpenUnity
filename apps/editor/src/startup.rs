//! Prepare the independent editor, launch its window, and finalize optional smoke outputs.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use openunity_editor::app::{AppOptions, EditorApp, SmokeStatus, UnsavedChoice};
use openunity_editor::appearance::AppearanceProfile;

mod arguments;
mod smoke;

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

pub(super) fn run() -> Result<(), String> {
    let Some(args) = arguments::arguments()? else {
        return Ok(());
    };
    let output_policy = smoke::output_policy(args.project.as_deref())?;
    let status = args
        .smoke
        .then(|| Arc::new(Mutex::new(SmokeStatus::default())));
    let mut app = EditorApp::new(AppOptions {
        project_path: args.project,
        scene_path: args.scene,
        smoke: status.clone(),
    })?;

    configure_app(
        &mut app,
        args.appearance,
        args.import_scene.as_deref(),
        args.bindings.as_deref(),
        args.smoke,
    )?;
    for path in [&args.receipt, &args.screenshot].into_iter().flatten() {
        app.check_output_path(path)?;
    }

    launch_window(app)?;

    if let Some(status) = status {
        smoke::finish(status, args.receipt, args.screenshot, &output_policy)?;
    }
    Ok(())
}

fn configure_app(
    app: &mut EditorApp,
    appearance: Option<AppearanceProfile>,
    import_scene: Option<&Path>,
    bindings: Option<&Path>,
    smoke: bool,
) -> Result<(), String> {
    app.set_appearance_profile(appearance.unwrap_or(AppearanceProfile::Reference));
    if let (Some(scene), Some(bindings)) = (import_scene, bindings) {
        app.request_import(scene, bindings)?;
        // Startup owns only the initial blank document; no interactive work exists yet.
        app.resolve_unsaved(UnsavedChoice::Discard)?;
    }
    if smoke {
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
    Ok(())
}

fn launch_window(app: EditorApp) -> Result<(), String> {
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
    .map_err(|e| e.to_string())
}
