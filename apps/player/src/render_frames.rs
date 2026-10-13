//! Same-session callbacks, native snapshots, and completed image publication.

use crate::arguments::RenderArguments;
use crate::render_manifest::RenderManifest;
use crate::render_output::{CallbackResult, RenderOutputPublisher};
use crate::render_preflight::PreparedRender;
use crate::render_scene::SessionPhase;
use engine_render_api::{DrawInstance, OrthographicCamera, RenderBackend, RenderFrame, TargetSize};
use engine_render_wgpu::WgpuRenderer;
use engine_scripting_bridge::{SceneSnapshot, SceneSnapshotRow, ScriptSession, SnapshotState};
use std::collections::HashMap;

pub(super) fn execute(
    session: &mut ScriptSession<'_>,
    renderer: &mut WgpuRenderer,
    prepared: &PreparedRender,
    arguments: &RenderArguments,
    publisher: &mut RenderOutputPublisher,
    phase: &mut SessionPhase,
) -> Result<(), String> {
    let initialized_value = session
        .initialize(0)
        .map_err(|error| format!("initialize scene session: {error}"))?;
    *phase = SessionPhase::Initialized;

    capture_and_publish(
        session,
        renderer,
        prepared,
        arguments.target_size(),
        publisher,
        0,
        CallbackResult::Succeeded {
            tick_input: None,
            returned: initialized_value,
        },
    )?;

    for (index, tick_input) in arguments.ticks().iter().copied().enumerate() {
        let callback_value = session
            .tick(tick_input)
            .map_err(|error| format!("tick callback {index} failed: {error}"))?;
        let ordinal = u64::try_from(index + 1)
            .map_err(|_| "capture ordinal exceeds its integer range".to_string())?;

        capture_and_publish(
            session,
            renderer,
            prepared,
            arguments.target_size(),
            publisher,
            ordinal,
            CallbackResult::Succeeded {
                tick_input: Some(tick_input),
                returned: callback_value,
            },
        )?;
    }

    Ok(())
}

fn capture_and_publish(
    session: &ScriptSession<'_>,
    renderer: &mut WgpuRenderer,
    prepared: &PreparedRender,
    target_size: TargetSize,
    publisher: &mut RenderOutputPublisher,
    ordinal: u64,
    callback: CallbackResult,
) -> Result<(), String> {
    let snapshot = session
        .capture_scene_snapshot(&prepared.snapshot_owners)
        .map_err(|error| format!("capture native scene snapshot: {error}"))?;
    let frame = build_frame(
        &snapshot,
        &prepared.manifest,
        &prepared.snapshot_owners,
        target_size,
    )?;
    let image = renderer
        .render(&frame)
        .map_err(|error| format!("render native frame {}: {error:?}", snapshot.frame()))?;
    if image.receipt().frame_number() != frame.frame_number() {
        return Err("backend returned a different native frame number".into());
    }
    publisher
        .publish_frame(ordinal, &image, callback)
        .map_err(|error| error.to_string())
}

fn build_frame<'assets>(
    snapshot: &SceneSnapshot,
    manifest: &'assets RenderManifest,
    snapshot_owners: &[i64],
    target_size: TargetSize,
) -> Result<RenderFrame<'assets>, String> {
    validate_snapshot_owners(snapshot, snapshot_owners)?;
    let rows = rows_by_owner(snapshot);
    let camera_binding = manifest.camera();
    let camera_row = active_row(&rows, camera_binding.owner(), "camera")?;
    let camera_affine = snapshot_affine(camera_row)?;
    let camera = OrthographicCamera::try_new_with_scale_chain(
        camera_affine,
        camera_row.unit_scale_chain(),
        camera_binding.half_height(),
        camera_binding.near(),
        camera_binding.far(),
        camera_binding.clear_rgba8(),
    )
    .map_err(|error| format!("validate camera snapshot: {error:?}"))?;

    let draws = build_draws(manifest, &rows)?;
    let frame_number = u64::try_from(snapshot.frame())
        .map_err(|_| "snapshot frame number is negative".to_string())?;
    RenderFrame::try_new(frame_number, target_size, camera, manifest.assets(), draws)
        .map_err(|error| format!("validate render frame: {error:?}"))
}

fn validate_snapshot_owners(snapshot: &SceneSnapshot, requested: &[i64]) -> Result<(), String> {
    if snapshot.rows().len() != requested.len() {
        return Err("snapshot row count differs from requested owners".into());
    }
    for (row, requested_owner) in snapshot.rows().iter().zip(requested) {
        if row.local_file_id() != *requested_owner {
            return Err("snapshot row order differs from requested owners".into());
        }
    }
    Ok(())
}

fn rows_by_owner(snapshot: &SceneSnapshot) -> HashMap<i64, &SceneSnapshotRow> {
    snapshot
        .rows()
        .iter()
        .map(|row| (row.local_file_id(), row))
        .collect()
}

fn active_row<'rows>(
    rows: &'rows HashMap<i64, &SceneSnapshotRow>,
    owner: i64,
    role: &str,
) -> Result<&'rows SceneSnapshotRow, String> {
    let row = rows
        .get(&owner)
        .copied()
        .ok_or_else(|| format!("snapshot omitted {role} owner {owner}"))?;
    if row.state() != SnapshotState::AliveActive {
        return Err(format!("{role} owner {owner} is dead or inactive"));
    }
    Ok(row)
}

fn build_draws(
    manifest: &RenderManifest,
    rows: &HashMap<i64, &SceneSnapshotRow>,
) -> Result<Vec<DrawInstance>, String> {
    let mut draws = Vec::with_capacity(manifest.draws().len());
    for binding in manifest.draws() {
        let row = rows
            .get(&binding.owner())
            .copied()
            .ok_or_else(|| format!("snapshot omitted draw owner {}", binding.owner()))?;
        if row.state() != SnapshotState::AliveActive {
            continue;
        }
        let affine = snapshot_affine(row)?;
        let draw = DrawInstance::try_new(
            binding.mesh_index(),
            binding.material_index(),
            affine,
            binding.enabled(),
            manifest.assets(),
        )
        .map_err(|error| format!("validate draw owner {}: {error:?}", binding.owner()))?;
        draws.push(draw);
    }
    Ok(draws)
}

fn snapshot_affine(row: &SceneSnapshotRow) -> Result<engine_math::Affine3x4, String> {
    let values = row.affine();
    engine_render_api::affine_from_snapshot_rows([
        [values[0], values[1], values[2], values[3]],
        [values[4], values[5], values[6], values[7]],
        [values[8], values[9], values[10], values[11]],
    ])
    .map_err(|error| format!("decode snapshot affine: {error:?}"))
}
