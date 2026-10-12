//! Independently authored controls for OpenUnity editor events, without Unity log capture.

mod detail;
mod header;
mod legacy;
mod observation;
mod presentation;
mod records;
mod severity;
mod state;
#[cfg(test)]
mod tests;

use crate::console_detail::{
    ConsoleDetailController, ConsoleLifetime, ConsoleRecordIdentity, severity_label,
};
use eframe::egui::{self, Label, ScrollArea, TextEdit, TextStyle, Ui};
use openunity_editor_core::console::{Console, Severity};

/// Severity and literal-message filters for the live editor console.
pub struct ConsolePanel {
    info: bool,
    warning: bool,
    error: bool,
    filter: String,
    detail: ConsoleDetailController,
    controls: Vec<serde_json::Value>,
    rows: Vec<serde_json::Value>,
    severity_icons: Vec<serde_json::Value>,
    visible_sequences: Vec<u64>,
    paint_clip: Option<egui::Rect>,
    detail_scroll: Option<serde_json::Value>,
    scroll_identity: Option<ConsoleRecordIdentity>,
}
