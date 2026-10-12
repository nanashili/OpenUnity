//! Protected native smoke output serialization and PNG encoding.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use openunity_editor::app::SmokeStatus;
use openunity_editor_core::write_guard::WritePolicy;

pub(super) fn output_policy(project: Option<&Path>) -> Result<WritePolicy, String> {
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

pub(super) fn finish(
    status: Arc<Mutex<SmokeStatus>>,
    receipt_path: Option<PathBuf>,
    screenshot_path: Option<PathBuf>,
    output_policy: &WritePolicy,
) -> Result<(), String> {
    let status = status
        .lock()
        .map_err(|_| "Native smoke status mutex was poisoned")?;
    let passed = status.completed && status.error.is_none() && status.screenshot_size.is_some();
    let receipt = receipt(&status, passed);
    if let Some(path) = receipt_path {
        write_new(
            output_policy,
            &path,
            &serde_json::to_vec_pretty(&receipt).map_err(|e| e.to_string())?,
        )?;
    }
    println!("{receipt}");
    if let Some(path) = screenshot_path {
        let image = status
            .screenshot_image
            .as_ref()
            .ok_or("Native renderer returned no screenshot")?;
        let bytes = encode_image(image)?;
        write_new(output_policy, &path, &bytes)?;
    }
    if !passed {
        return Err(status
            .error
            .clone()
            .unwrap_or_else(|| "Native frame/screenshot smoke did not complete".into()));
    }
    Ok(())
}

fn receipt(status: &SmokeStatus, passed: bool) -> serde_json::Value {
    let imported = status
        .document_snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.get("format"))
        .and_then(|value| value.as_str())
        == Some("openunity.imported-scene");

    serde_json::json!({
        "schema_version": 1,
        "specification": if imported { "INFRA-EDITOR-SCENE-001" } else { "INFRA-EDITOR-001" },
        "verification_kind":"independent_native_frame",
        "os":std::env::consts::OS,
        "architecture":std::env::consts::ARCH,
        "renderer_requested":"Wgpu",
        "frames":status.frames,
        "screenshot_size":status.screenshot_size,
        "completed":status.completed,
        "error":status.error,
        "appearance_profile":status.appearance_profile,
        "workspace_snapshot":status.workspace_snapshot,
        "appearance_snapshot":status.appearance_snapshot,
        "document_snapshot":status.document_snapshot,
        "document_kind": if imported { "imported" } else { "authoring" },
        "unity_compatibility":"not_verified",
        "status":if passed {"passed"} else {"failed"}
    })
}

fn encode_image(image: &eframe::egui::ColorImage) -> Result<Vec<u8>, String> {
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
    Ok(bytes)
}
