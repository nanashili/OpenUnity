//! Independent Reference Console selection, owner identity and detail layout.

use eframe::egui::{Rect, pos2};
use openunity_editor_core::console::{Console, Severity};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_LIFETIME: AtomicU64 = AtomicU64::new(1);

/// Opaque process-session Console owner identity, independent of storage addresses.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConsoleLifetime(u64);
impl ConsoleLifetime {
    /// Read-only diagnostic identity; callers cannot create an owner from a number.
    pub fn value(self) -> u64 {
        self.0
    }
    fn allocate() -> Result<Self, String> {
        NEXT_LIFETIME
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
            .map(Self)
            .map_err(|_| "Console lifetime identities exhausted".into())
    }
}
/// One selection request bound to the lifetime that produced its row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConsoleRecordIdentity {
    /// Actual owner identity captured when the record control was produced.
    pub lifetime: ConsoleLifetime,
    /// Existing retained Console sequence, never a filtered row index.
    pub sequence: u64,
}
impl ConsoleRecordIdentity {
    /// Encode scope and sequence without introducing a textual identity namespace.
    pub fn to_json(self) -> serde_json::Value {
        serde_json::json!({"lifetime":self.lifetime.value(),"sequence":self.sequence})
    }
}
/// Independently designed disjoint subdivision of the remaining Console pane.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConsoleDetailLayout {
    /// Upper two-thirds record list allocation.
    pub list_rect: Rect,
    /// Lower one-third detail allocation.
    pub detail_rect: Rect,
}
/// Committed view state over existing immutable retained records.
#[derive(Clone, Debug)]
pub struct ConsoleDetailSnapshot {
    /// Bound owner, absent only when a new lifetime could not be allocated.
    pub lifetime: Option<ConsoleLifetime>,
    /// Optional retained and currently visible selected record identity.
    pub selected: Option<ConsoleRecordIdentity>,
    /// Exact unmodified selected message, bounded by the existing Console limit.
    pub detail_message: Option<String>,
    /// Severity from the same selected record as the message.
    pub detail_severity: Option<Severity>,
    /// Current logical detail offset; zero when unselected.
    pub detail_scroll_y: f32,
    /// Last accepted finite nonnegative allocation.
    pub layout: Option<ConsoleDetailLayout>,
    /// Explicit selection, lifetime or layout diagnostic, never a synthetic log record.
    pub diagnostic: Option<String>,
}
/// Readable severity for existing Console records, without parsing message text.
pub fn severity_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Info => "Info",
        Severity::Warning => "Warning",
        Severity::Error => "Error",
    }
}
impl ConsoleDetailSnapshot {
    /// Encode committed semantics and last valid logical layout.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({"lifetime":self.lifetime.map(ConsoleLifetime::value),
            "selected":self.selected.map(ConsoleRecordIdentity::to_json),
            "detail_message":self.detail_message,
            "detail_severity":self.detail_severity.map(severity_label),
            "detail_scroll_y":self.detail_scroll_y,"diagnostic":self.diagnostic,
            "list_rect":self.layout.map(|l|crate::appearance::rect_json(l.list_rect)),
            "detail_rect":self.layout.map(|l|crate::appearance::rect_json(l.detail_rect))})
    }
}
/// Bounded UI state only: one selected message and one last valid layout.
pub struct ConsoleDetailController {
    state: ConsoleDetailSnapshot,
}
impl Default for ConsoleDetailController {
    fn default() -> Self {
        let lifetime = ConsoleLifetime::allocate();
        Self {
            state: ConsoleDetailSnapshot {
                lifetime: lifetime.as_ref().ok().copied(),
                selected: None,
                detail_message: None,
                detail_severity: None,
                detail_scroll_y: 0.0,
                layout: None,
                diagnostic: lifetime.err(),
            },
        }
    }
}
impl ConsoleDetailController {
    /// Current actual Console owner identity.
    pub fn lifetime(&self) -> Option<ConsoleLifetime> {
        self.state.lifetime
    }
    /// Capture an existing sequence with the current owner; liveness is checked at dispatch.
    pub fn identity(&self, sequence: u64) -> Option<ConsoleRecordIdentity> {
        self.lifetime()
            .map(|lifetime| ConsoleRecordIdentity { lifetime, sequence })
    }
    /// Complete read-only committed selection and detail state.
    pub fn snapshot(&self) -> &ConsoleDetailSnapshot {
        &self.state
    }
    /// Allocate a fresh owner before replacing/binding a Console. Failure is atomic.
    pub fn reset_owner(&mut self) -> Result<(), String> {
        let lifetime = ConsoleLifetime::allocate()?;
        self.state.lifetime = Some(lifetime);
        self.clear_selection();
        self.state.layout = None;
        self.state.diagnostic = None;
        Ok(())
    }
    fn clear_selection(&mut self) {
        self.state.selected = None;
        self.state.detail_message = None;
        self.state.detail_severity = None;
        self.state.detail_scroll_y = 0.0;
    }
    /// Validate current retention/visibility before actions or paint, including hidden returns.
    pub fn reconcile(&mut self, console: &Console, severities: &[Severity], filter: &str) {
        let Some(selected) = self.state.selected else {
            return;
        };
        let record = (self.state.lifetime == Some(selected.lifetime))
            .then(|| {
                console
                    .filtered(severities, filter)
                    .find(|r| r.sequence == selected.sequence)
            })
            .flatten();
        if let Some(record) = record {
            if self.state.detail_message.as_deref() != Some(record.message.as_str()) {
                self.state.detail_message = Some(record.message.clone());
            }
            self.state.detail_severity = Some(record.severity);
        } else {
            self.clear_selection();
        }
    }
    /// Resolve one captured identity against the current owner, retention and filters.
    pub fn select(
        &mut self,
        identity: ConsoleRecordIdentity,
        console: &Console,
        severities: &[Severity],
        filter: &str,
    ) -> Result<(), String> {
        self.reconcile(console, severities, filter);
        let error = if self.state.lifetime != Some(identity.lifetime) {
            Some("Stale console selection")
        } else if !console
            .filtered(severities, filter)
            .any(|r| r.sequence == identity.sequence)
        {
            Some("No retained visible console record")
        } else {
            None
        };
        if let Some(error) = error {
            self.state.diagnostic = Some(error.into());
            return Err(error.into());
        }
        if self.state.selected != Some(identity) {
            self.state.selected = Some(identity);
            self.state.detail_scroll_y = 0.0;
        }
        self.state.diagnostic = None;
        self.reconcile(console, severities, filter);
        Ok(())
    }
    /// Resolve finite nonnegative pane geometry; invalid measurements retain prior layout.
    pub fn layout(&mut self, remaining: Rect) -> Result<ConsoleDetailLayout, String> {
        let width = remaining.width();
        let height = remaining.height();
        if !remaining.is_finite()
            || !width.is_finite()
            || !height.is_finite()
            || width < 0.0
            || height < 0.0
        {
            return self.invalid_layout();
        }
        let split = remaining.top() + height * (2.0 / 3.0);
        if !split.is_finite() {
            return self.invalid_layout();
        }
        let layout = ConsoleDetailLayout {
            list_rect: Rect::from_min_max(remaining.min, pos2(remaining.right(), split)),
            detail_rect: Rect::from_min_max(pos2(remaining.left(), split), remaining.max),
        };
        self.state.layout = Some(layout);
        if self.state.diagnostic.as_deref() == Some("Invalid console detail layout metrics") {
            self.state.diagnostic = None;
        }
        Ok(layout)
    }
    /// Commit an actual scroll result, clamped to the current finite content range.
    /// Hidden/zero-size rendering does not call this and therefore retains its offset.
    pub fn update_scroll(&mut self, offset_y: f32, max_scroll_y: f32) -> Result<(), String> {
        if !offset_y.is_finite()
            || offset_y < 0.0
            || !max_scroll_y.is_finite()
            || max_scroll_y < 0.0
        {
            return self.invalid_layout();
        }
        self.state.detail_scroll_y = if self.state.selected.is_some() {
            offset_y.min(max_scroll_y)
        } else {
            0.0
        };
        Ok(())
    }
    fn invalid_layout<T>(&mut self) -> Result<T, String> {
        let error = "Invalid console detail layout metrics";
        self.state.diagnostic = Some(error.into());
        Err(error.into())
    }
}
