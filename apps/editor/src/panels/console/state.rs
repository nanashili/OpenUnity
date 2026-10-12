//! Console ownership, selection reconciliation and filter state.

use super::*;

impl Default for ConsolePanel {
    fn default() -> Self {
        Self {
            info: true,
            warning: true,
            error: true,
            filter: String::new(),
            detail: ConsoleDetailController::default(),
            controls: Vec::new(),
            rows: Vec::new(),
            severity_icons: Vec::new(),
            visible_sequences: Vec::new(),
            paint_clip: None,
            detail_scroll: None,
            scroll_identity: None,
        }
    }
}

impl ConsolePanel {
    pub(super) fn severities(&self) -> Vec<Severity> {
        [
            (self.info, Severity::Info),
            (self.warning, Severity::Warning),
            (self.error, Severity::Error),
        ]
        .into_iter()
        .filter_map(|(enabled, severity)| enabled.then_some(severity))
        .collect()
    }

    /// Current explicit owner scope, independent of records and storage addresses.
    pub fn lifetime(&self) -> Option<ConsoleLifetime> {
        self.detail.lifetime()
    }

    /// Capture a current owner with an existing record sequence for later validation.
    pub fn identity(&self, sequence: u64) -> Option<ConsoleRecordIdentity> {
        self.detail.identity(sequence)
    }

    /// Rebind before supplying a replaced Console, invalidating every old-scope event.
    pub fn reset_owner(&mut self) -> Result<(), String> {
        self.detail.reset_owner()?;
        self.controls.clear();
        self.severity_icons.clear();
        self.rows.clear();
        self.visible_sequences.clear();
        self.detail_scroll = None;
        self.scroll_identity = None;
        self.paint_clip = None;
        Ok(())
    }

    /// Reconcile current retention and filters without selecting any fallback record.
    pub fn reconcile(&mut self, console: &Console) {
        let severities = self.severities();
        self.detail.reconcile(console, &severities, &self.filter);
        self.visible_sequences = console
            .filtered(&severities, &self.filter)
            .map(|r| r.sequence)
            .collect();
    }

    /// Resolve a captured visible record against the current Console and filters.
    pub fn select(
        &mut self,
        identity: ConsoleRecordIdentity,
        console: &Console,
    ) -> Result<(), String> {
        let severities = self.severities();
        self.detail
            .select(identity, console, &severities, &self.filter)
    }

    /// Replace the existing literal, case-sensitive message filter and reconcile detail.
    pub fn set_filter(&mut self, filter: String, console: &Console) {
        self.reconcile(console);
        self.filter = filter;
        self.reconcile(console);
    }

    /// Replace enabled severity filters using the existing severity-intersection semantics.
    pub fn set_severities(&mut self, severities: &[Severity], console: &Console) {
        self.reconcile(console);
        self.info = severities.contains(&Severity::Info);
        self.warning = severities.contains(&Severity::Warning);
        self.error = severities.contains(&Severity::Error);
        self.reconcile(console);
    }
}
