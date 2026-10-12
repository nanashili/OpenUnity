//! Bounded PNG decoding and per-Context texture ownership.

use std::sync::Arc;

use eframe::egui::{self, Context, Id};

use super::assets::{ICONS, IconAsset};
use crate::appearance::AppearanceProfile;

pub(super) struct CachedIcon {
    pub(super) texture: egui::TextureHandle,
    pub(super) dimensions: [u32; 2],
}
#[derive(Default)]
pub(super) struct IconCache {
    pub(super) icons: std::collections::BTreeMap<&'static str, CachedIcon>,
    pub(super) errors: Vec<String>,
}

fn decode_icon(asset: &IconAsset) -> Result<egui::ColorImage, String> {
    if asset.bytes.len() != asset.expected_bytes || asset.bytes.len() > 65_536 {
        return Err("encoded size is outside the reviewed bound".into());
    }
    let decoder = png::Decoder::new_with_limits(
        std::io::Cursor::new(asset.bytes),
        png::Limits { bytes: 1_048_576 },
    );
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let info = reader.info();
    if [info.width, info.height] != asset.dimensions
        || info.width == 0
        || info.height == 0
        || info.width > 256
        || info.height > 256
        || info.color_type != png::ColorType::Rgba
        || info.bit_depth != png::BitDepth::Eight
    {
        return Err("dimensions or RGBA8 format differ from the reviewed resource".into());
    }
    let size = reader
        .output_buffer_size()
        .filter(|size| *size <= 262_144)
        .ok_or_else(|| "decoded image exceeds the reviewed allocation bound".to_owned())?;
    let expected = asset.dimensions[0] as usize * asset.dimensions[1] as usize * 4;
    if size != expected {
        return Err("decoded RGBA size differs from declared dimensions".into());
    }
    let mut rgba = vec![0; size];
    let output = reader.next_frame(&mut rgba).map_err(|e| e.to_string())?;
    if output.buffer_size() != expected {
        return Err("decoded frame has an unexpected size".into());
    }
    // Approved PNGs already use top-left orientation and unassociated RGBA.
    Ok(egui::ColorImage::from_rgba_unmultiplied(
        [asset.dimensions[0] as usize, asset.dimensions[1] as usize],
        &rgba,
    ))
}

/// Clear receipts every retained UI pass. Each Context owns its textures.
pub(crate) fn begin_frame(ctx: &Context, profile: AppearanceProfile) -> Option<String> {
    ctx.data_mut(|d| {
        d.insert_temp(
            Id::new("editor.icon.paint_records"),
            Vec::<serde_json::Value>::new(),
        );
        d.insert_temp(Id::new("editor.icon.records_truncated"), false);
    });
    if profile == AppearanceProfile::Legacy
        || ctx.data(|d| {
            d.get_temp::<Arc<IconCache>>(Id::new("editor.icon.cache"))
                .is_some()
        })
    {
        return None;
    }
    let mut cache = IconCache::default();
    if cfg!(feature = "local-reference-assets") {
        for asset in ICONS {
            match decode_icon(asset) {
                Ok(image) => {
                    let texture = ctx.load_texture(asset.id, image, egui::TextureOptions::LINEAR);
                    cache.icons.insert(
                        asset.id,
                        CachedIcon {
                            texture,
                            dimensions: asset.dimensions,
                        },
                    );
                }
                Err(error) => cache.errors.push(format!(
                    "{}: {error}; independent geometry remains in use.",
                    asset.id
                )),
            }
        }
    } else {
        cache.errors.push("Local reference icons unavailable; existing semantic glyph and geometry fallbacks remain active.".to_owned());
    }
    let error = (!cache.errors.is_empty()).then(|| cache.errors.join("\n"));
    ctx.data_mut(|d| d.insert_temp(Id::new("editor.icon.cache"), Arc::new(cache)));
    error
}
