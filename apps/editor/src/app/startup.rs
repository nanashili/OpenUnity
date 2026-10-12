use super::*;

impl EditorApp {
    /// Construct a validated app; startup filesystem/model errors are returned.
    pub fn new(options: AppOptions) -> Result<Self, String> {
        let protected_roots = Self::startup_protected_roots()?;
        let policy = WritePolicy::new(protected_roots.clone()).map_err(|e| e.to_string())?;
        let preferences = Preferences::default();
        let mut app = Self {
            document: Document::new(
                Scene::new("Untitled").map_err(|e| e.to_string())?,
                preferences.history_capacity,
            ),
            imported: None,
            provenance_roots: Vec::new(),
            imported_inspector: imported::InspectorPanel::default(),
            imported_viewport: imported::ViewportPanel::default(),
            bindings_text: String::new(),
            project: None,
            save_path: None,
            forbidden_reads: protected_roots.clone(),
            protected_roots,
            policy,
            workspace: Workspace::default(),
            hierarchy_browser: HierarchyBrowser::default(),
            inspector_scroll: serde_json::Value::Null,
            inspector_scope: 1,
            inspector_controller: InspectorController::deferred(),
            inspector_controls: Vec::new(),
            inspector_menu_popup: None,
            inspector_component_menu_context: None,
            component_expansion: ComponentExpansionController::default(),
            resource_snapshot: serde_json::Value::Null,
            path_form: None,
            preferences,
            console: Console::new(512).map_err(|e| e.to_string())?,
            project_panel: ProjectBrowserPanel::default(),
            inspector_panel: InspectorPanel::default(),
            console_panel: ConsolePanel::default(),
            viewport_panel: ViewportPanel::default(),
            project_text: String::new(),
            scene_text: String::new(),
            status: "Independent OpenUnity authoring; Unity compatibility is unverified.".into(),
            pending: None,
            close_allowed: false,
            smoke: options.smoke,
            smoke_started: Instant::now(),
            smoke_requested: false,
            frames: 0,
        };
        if let Some(path) = options.project_path {
            app.open_project(&path)?;
        }
        if let Some(path) = options.scene_path {
            let candidate = app.load_document(&path)?;
            app.protect_candidate(&candidate);
            app.perform(candidate)?;
        }
        Ok(app)
    }

    fn startup_protected_roots() -> Result<Vec<PathBuf>, String> {
        let mut candidates = vec![PathBuf::from("/Applications")];
        if let Some(home) = std::env::var_os("HOME") {
            let home = PathBuf::from(home);
            candidates.push(home.join(".local/share/openunity-research"));
            candidates.push(home.join("Library/Application Support/OpenUnityPrivateResearch"));
        }
        let mut protected_roots = Vec::new();
        // Directory metadata only: never inspect reference/research contents.
        for path in candidates {
            match fs::symlink_metadata(&path) {
                Ok(_) => protected_roots.push(path),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(format!("protect {}: {error}", path.display())),
            }
        }
        Ok(protected_roots)
    }
}
