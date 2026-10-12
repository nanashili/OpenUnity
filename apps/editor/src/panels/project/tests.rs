//! Existing Project browser input, selection and read-only assertions.

use super::*;
use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "openunity-project-panel-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::create_dir(root.join("Assets")).unwrap();
        fs::create_dir(root.join("ProjectSettings")).unwrap();
        fs::write(
            root.join("ProjectSettings/ProjectVersion.txt"),
            "m_EditorVersion: 6000.0.23f1\n",
        )
        .unwrap();
        Self(root)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn frame(
    ctx: &egui::Context,
    panel: &mut ProjectBrowserPanel,
    project: Option<&Project>,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(900.0, 700.0),
            )),
            events,
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| panel.show(ui, project));
        },
    )
}

fn text_position(output: &egui::FullOutput, needle: &str) -> Option<egui::Pos2> {
    output.shapes.iter().find_map(|shape| match &shape.shape {
        egui::epaint::Shape::Text(text) if text.galley.text().contains(needle) => {
            Some(text.pos + text.galley.size() * 0.5)
        }
        _ => None,
    })
}

fn click(pos: egui::Pos2) -> Vec<egui::Event> {
    vec![
        egui::Event::PointerMoved(pos),
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        },
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        },
    ]
}

#[test]
fn renders_missing_and_empty_projects_with_honest_guidance() {
    let ctx = egui::Context::default();
    let mut panel = ProjectBrowserPanel::default();
    let output = frame(&ctx, &mut panel, None, vec![]);
    assert!(text_position(&output, "No project is open").is_some());
    assert!(panel.selected_path().is_none());

    let fixture = Fixture::new();
    let project = Project::open(&fixture.0).unwrap();
    let output = frame(&ctx, &mut panel, Some(&project), vec![]);
    assert!(text_position(&output, "6000.0.23f1").is_some());
    assert!(text_position(&output, "no discovered files").is_some());
}

#[test]
fn selects_real_paths_filters_and_clears_selection_on_project_replacement() {
    let fixture = Fixture::new();
    fs::create_dir(fixture.0.join("Assets/Models")).unwrap();
    fs::write(
        fixture.0.join("Assets/Models/Cube.txt"),
        "independent fixture",
    )
    .unwrap();
    fs::write(
        fixture.0.join("Assets/Camera.txt"),
        "independent camera fixture",
    )
    .unwrap();
    let project = Project::open(&fixture.0).unwrap();
    let ctx = egui::Context::default();
    let mut panel = ProjectBrowserPanel::default();
    let output = frame(&ctx, &mut panel, Some(&project), vec![]);
    let cube = text_position(&output, "Models/Cube.txt (file)").unwrap();
    let output = frame(&ctx, &mut panel, Some(&project), click(cube));
    assert_eq!(panel.selected_path(), Some(Path::new("Models/Cube.txt")));
    assert!(text_position(&output, "not imported").is_some());
    assert!(text_position(&output, "Relative path: Models/Cube.txt").is_some());

    let filter = text_position(&output, "Search Assets paths").unwrap();
    frame(&ctx, &mut panel, Some(&project), click(filter));
    let output = frame(
        &ctx,
        &mut panel,
        Some(&project),
        vec![egui::Event::Text("cAmErA".into())],
    );
    assert!(text_position(&output, "Camera.txt (file)").is_some());
    assert!(text_position(&output, "Models/Cube.txt (file)").is_none());
    assert!(text_position(&output, "1 of 3 entries").is_some());

    let replacement = Fixture::new();
    let replacement = Project::open(&replacement.0).unwrap();
    frame(&ctx, &mut panel, Some(&replacement), vec![]);
    assert!(panel.selected_path().is_none());
    frame(&ctx, &mut panel, None, vec![]);
    assert!(panel.selected_path().is_none());
    assert_eq!(
        fs::read_to_string(fixture.0.join("Assets/Models/Cube.txt")).unwrap(),
        "independent fixture"
    );
}
