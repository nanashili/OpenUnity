use engine_render_api::{AdapterPolicy, RenderedImage, TargetFormat, TargetSize};
use sha2::{Digest, Sha256};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use super::PublishError;
use super::receipt_files::io_error;

pub(super) struct PublishedFrame {
    pub(super) filename: String,
    pub(super) rgba_sha256: String,
    pub(super) ppm_sha256: String,
}

pub(super) fn publish(
    directory: &Path,
    ordinal: u64,
    image: &RenderedImage,
    requested_size: TargetSize,
    requested_policy: AdapterPolicy,
) -> Result<PublishedFrame, PublishError> {
    validate_image(image, requested_size, requested_policy)?;

    let rgba_hash = digest(image.rgba8());
    let ppm = ppm_bytes(image.size(), image.rgba8());
    let ppm_hash = digest(&ppm);
    let filename = format!("frame-{ordinal:06}.ppm");
    write_ppm(directory, &filename, &ppm)?;

    Ok(PublishedFrame {
        filename,
        rgba_sha256: rgba_hash,
        ppm_sha256: ppm_hash,
    })
}

fn validate_image(
    image: &RenderedImage,
    requested: TargetSize,
    requested_policy: AdapterPolicy,
) -> Result<(), PublishError> {
    let receipt = image.receipt();
    if image.size() != requested || receipt.target_size() != requested {
        return Err(PublishError::InvalidFrame("rendered size mismatch".into()));
    }
    if receipt.target_format() != TargetFormat::Rgba8Unorm || !receipt.readback_complete() {
        return Err(PublishError::InvalidFrame(
            "incomplete or unsupported readback".into(),
        ));
    }
    if receipt.requested_policy() != requested_policy {
        return Err(PublishError::InvalidFrame(
            "backend policy differs from request".into(),
        ));
    }
    if image.rgba8().len() != requested.rgba8_byte_len() {
        return Err(PublishError::InvalidFrame(
            "RGBA byte count mismatch".into(),
        ));
    }
    Ok(())
}

fn write_ppm(directory: &Path, filename: &str, ppm: &[u8]) -> Result<(), PublishError> {
    let path = directory.join(filename);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|source| io_error("exclusively create frame", source))?;
    file.write_all(ppm)
        .map_err(|source| io_error("write frame", source))?;
    file.flush()
        .map_err(|source| io_error("flush frame", source))?;
    file.sync_all()
        .map_err(|source| io_error("close frame", source))?;
    drop(file);
    Ok(())
}

fn ppm_bytes(size: TargetSize, rgba: &[u8]) -> Vec<u8> {
    let header = format!("P6\n{} {}\n255\n", size.width(), size.height());
    let mut ppm = Vec::with_capacity(header.len() + rgba.len() / 4 * 3);
    ppm.extend_from_slice(header.as_bytes());
    for pixel in rgba.chunks_exact(4) {
        ppm.extend_from_slice(&pixel[..3]);
    }
    ppm
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
