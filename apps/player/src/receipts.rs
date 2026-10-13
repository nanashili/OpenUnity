use engine_render_api::{AdapterPolicy, RenderedImage};
use serde::Serialize;
use std::path::PathBuf;

use super::frame_artifacts::PublishedFrame;
use super::{CallbackResult, CleanupOutcome, RenderOutputPublisher};

#[derive(Clone, Debug, Serialize)]
pub(super) struct SizeRecord {
    width: u32,
    height: u32,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct FrameRecord {
    ordinal: u64,
    native_frame: u64,
    callback: CallbackResult,
    rgba_sha256: String,
    ppm_sha256: String,
    file: String,
    adapter: AdapterRecord,
    policy: String,
    format: String,
    size: SizeRecord,
    draw_count: usize,
    triangle_count: usize,
    readback_complete: bool,
}

#[derive(Clone, Debug, Serialize)]
struct AdapterRecord {
    backend: String,
    name: String,
    vendor: String,
    device: String,
    driver: String,
    driver_info: String,
    device_type: String,
}

#[derive(Serialize)]
pub(super) struct Receipt {
    schema_version: u32,
    status: &'static str,
    requested_size: SizeRecord,
    requested_policy: String,
    frames: Vec<FrameRecord>,
    cleanup: CleanupOutcome,
    #[serde(skip)]
    pub(super) directory: PathBuf,
}

impl Receipt {
    pub(super) fn success(publisher: RenderOutputPublisher, cleanup: CleanupOutcome) -> Self {
        Self {
            schema_version: 1,
            status: "success",
            requested_size: size_record(publisher.requested_size),
            requested_policy: policy_name(publisher.requested_policy),
            frames: publisher.frames,
            cleanup,
            directory: publisher.directory,
        }
    }

    pub(super) fn into_failure(self, diagnostic: String) -> FailureReceipt {
        FailureReceipt {
            schema_version: 1,
            status: "failed",
            diagnostic,
            requested_size: self.requested_size,
            requested_policy: self.requested_policy,
            frames: self.frames,
            cleanup: self.cleanup,
            directory: self.directory,
        }
    }
}

#[derive(Serialize)]
pub(super) struct FailureReceipt {
    schema_version: u32,
    status: &'static str,
    diagnostic: String,
    requested_size: SizeRecord,
    requested_policy: String,
    frames: Vec<FrameRecord>,
    cleanup: CleanupOutcome,
    #[serde(skip)]
    pub(super) directory: PathBuf,
}

impl FailureReceipt {
    pub(super) fn new(
        publisher: RenderOutputPublisher,
        diagnostic: String,
        cleanup: CleanupOutcome,
    ) -> Self {
        Self {
            schema_version: 1,
            status: "failed",
            diagnostic,
            requested_size: size_record(publisher.requested_size),
            requested_policy: policy_name(publisher.requested_policy),
            frames: publisher.frames,
            cleanup,
            directory: publisher.directory,
        }
    }

    pub(super) fn from_success(receipt: Receipt, diagnostic: String) -> Self {
        receipt.into_failure(diagnostic)
    }
}

pub(super) fn frame_record(
    ordinal: u64,
    artifact: PublishedFrame,
    image: &RenderedImage,
    callback: CallbackResult,
) -> FrameRecord {
    let receipt = image.receipt();
    let adapter = receipt.adapter();
    FrameRecord {
        ordinal,
        native_frame: receipt.frame_number(),
        callback,
        rgba_sha256: artifact.rgba_sha256,
        ppm_sha256: artifact.ppm_sha256,
        file: artifact.filename,
        adapter: AdapterRecord {
            backend: adapter.backend().into(),
            name: adapter.adapter_name().into(),
            vendor: adapter.vendor().into(),
            device: adapter.device().into(),
            driver: adapter.driver().into(),
            driver_info: adapter.driver_info().into(),
            device_type: adapter.device_type().into(),
        },
        policy: policy_name(receipt.requested_policy()),
        format: "rgba8-unorm".into(),
        size: size_record(receipt.target_size()),
        draw_count: receipt.draw_count(),
        triangle_count: receipt.triangle_count(),
        readback_complete: receipt.readback_complete(),
    }
}

fn policy_name(policy: AdapterPolicy) -> String {
    match policy {
        AdapterPolicy::Hardware => "hardware",
        AdapterPolicy::Software => "software",
    }
    .into()
}

fn size_record(size: engine_render_api::TargetSize) -> SizeRecord {
    SizeRecord {
        width: size.width(),
        height: size.height(),
    }
}
