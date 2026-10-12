use super::*;
use openunity_editor_core::scene::Primitive;
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let path = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "openunity-shell-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        if let Err(cause) = fs::remove_dir_all(&self.0) {
            let message = format!("failed to remove fixture {}: {cause}", self.0.display());
            if std::thread::panicking() {
                use std::io::Write;
                // Reporting must not replace the initiating assertion panic.
                let _ = writeln!(std::io::stderr().lock(), "{message}");
            } else {
                panic!("{message}");
            }
        }
    }
}

#[cfg(unix)]
#[test]
fn project_open_records_discovery_warnings_and_protects_source_without_editing_it() {
    let directory = Directory::new();
    fs::create_dir(directory.0.join("Assets")).unwrap();
    fs::create_dir(directory.0.join("ProjectSettings")).unwrap();
    fs::write(
        directory.0.join("ProjectSettings/ProjectVersion.txt"),
        "m_EditorVersion: 7000.0.0a8\n",
    )
    .unwrap();
    let source = directory.0.join("Assets/source.asset");
    fs::write(&source, "independently authored source").unwrap();
    let link = directory.0.join("Assets/linked.asset");
    std::os::unix::fs::symlink(&source, &link).unwrap();
    let mut app = EditorApp::new(AppOptions::default()).unwrap();
    app.apply_action(EditorAction::Create {
        primitive: Primitive::Cube,
        parent: None,
    })
    .unwrap();
    let before = app.semantic_snapshot().unwrap();

    app.open_project(&directory.0).unwrap();

    let console = app.console_snapshot();
    assert_eq!(console["records"].as_array().unwrap().len(), 1);
    assert_eq!(console["records"][0]["severity"], "Warning");
    assert_eq!(
        console["records"][0]["message"],
        "linked.asset: symlink asset entries are unsupported"
    );
    assert!(app.check_output_path(&source).is_err());
    assert_eq!(app.semantic_snapshot().unwrap(), before);
    assert!(app.is_dirty());
    assert_eq!(
        fs::read_to_string(&source).unwrap(),
        "independently authored source"
    );
    assert_eq!(fs::read_link(&link).unwrap(), source);
}

#[test]
fn replacement_guard_preserves_work_on_cancel_and_failed_save() {
    let directory = Directory::new();
    let path = directory.0.join("replacement.ouscene");
    let policy = WritePolicy::new([]).unwrap();
    scene_io::save(&path, &Scene::new("Replacement").unwrap(), &policy).unwrap();
    let mut app = EditorApp::new(AppOptions::default()).unwrap();
    app.apply_action(EditorAction::Create {
        primitive: Primitive::Cube,
        parent: None,
    })
    .unwrap();
    let selected = app.document().selection().primary();
    app.request_open(&path).unwrap();
    assert_eq!(app.document().scene().entities().len(), 1);
    app.resolve_unsaved(UnsavedChoice::Cancel).unwrap();
    assert_eq!(app.document().selection().primary(), selected);
    app.request_open(&path).unwrap();
    app.scene_text = directory
        .0
        .join("missing/failed.ouscene")
        .display()
        .to_string();
    assert!(app.resolve_unsaved(UnsavedChoice::Save).is_err());
    assert!(app.document().is_dirty());
    assert_eq!(app.document().scene().entities().len(), 1);
    assert!(app.save_path().is_none());
    app.resolve_unsaved(UnsavedChoice::Discard).unwrap();
    assert_eq!(app.document().scene().name(), "Replacement");
    assert!(app.document().selection().primary().is_none());
    assert!(!app.document().can_undo());
    assert!(!app.document().is_dirty());
}

#[test]
fn save_as_preserves_existing_target_and_save_failure_keeps_current_path() {
    let directory = Directory::new();
    let path = directory.0.join("owned.ouscene");
    let mut app = EditorApp::new(AppOptions::default()).unwrap();
    app.save_to(&path, true).unwrap();
    let before = fs::read(&path).unwrap();
    app.apply_action(EditorAction::Create {
        primitive: Primitive::Camera,
        parent: None,
    })
    .unwrap();
    assert!(app.save_to(&path, true).is_err());
    assert_eq!(fs::read(&path).unwrap(), before);
    let failed = directory.0.join("missing/failure.ouscene");
    assert!(app.save_to(&failed, true).is_err());
    assert_eq!(app.save_path(), Some(path.as_path()));
    assert!(app.document().is_dirty());
    app.save_to(&path, false).unwrap();
    assert!(!app.document().is_dirty());
    assert_eq!(
        scene_io::load(&path, &app.policy).unwrap(),
        *app.document().scene()
    );
}

#[test]
fn escape_cancels_unsaved_close_and_discard_emits_close() {
    let mut app = EditorApp::new(AppOptions::default()).unwrap();
    app.apply_action(EditorAction::Create {
        primitive: Primitive::Cube,
        parent: None,
    })
    .unwrap();
    app.request_close().unwrap();
    let context = Context::default();
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1200.0, 800.0),
        )),
        events: vec![egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
        ..Default::default()
    };
    let output = context.run(input, |ctx| app.show(ctx));
    assert!(!output.shapes.is_empty());
    assert!(!output.viewport_output.values().any(|viewport| {
        viewport
            .commands
            .iter()
            .any(|command| matches!(command, egui::ViewportCommand::Close))
    }));
    assert_eq!(app.document().scene().entities().len(), 1);
    app.resolve_unsaved(UnsavedChoice::Discard).unwrap();
    let output = context.run(egui::RawInput::default(), |ctx| app.show(ctx));
    assert!(!output.viewport_output.values().any(|viewport| {
        viewport
            .commands
            .iter()
            .any(|command| matches!(command, egui::ViewportCommand::Close))
    }));
    app.request_close().unwrap();
    app.resolve_unsaved(UnsavedChoice::Discard).unwrap();
    let output = context.run(egui::RawInput::default(), |ctx| app.show(ctx));
    assert!(output.viewport_output.values().any(|viewport| {
        viewport
            .commands
            .iter()
            .any(|command| matches!(command, egui::ViewportCommand::Close))
    }));
}
