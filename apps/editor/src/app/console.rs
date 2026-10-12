use super::*;

impl EditorApp {
    /// Actual Console view state, existing retained records and current core counters.
    pub fn console_snapshot(&self) -> serde_json::Value {
        let mut value = self.console_panel.snapshot();
        value["profile"] = self.workspace.profile().name().into();
        value["capacity"] = self.console.capacity().into();
        value["dropped_count"] = self.console.dropped_count().into();
        value["records"] = self
            .console
            .records()
            .map(|record| {
                serde_json::json!({
                    "sequence": record.sequence,
                    "severity": crate::console_detail::severity_label(record.severity),
                    "message": record.message
                })
            })
            .collect::<Vec<_>>()
            .into();
        value
    }

    /// Capture an existing record sequence with the actual current Console owner.
    pub fn console_identity(
        &self,
        sequence: u64,
    ) -> Option<crate::console_detail::ConsoleRecordIdentity> {
        self.console_panel.identity(sequence)
    }

    /// Select a retained currently visible Console record without touching editor documents.
    pub fn select_console_record(
        &mut self,
        identity: crate::console_detail::ConsoleRecordIdentity,
    ) -> Result<(), String> {
        self.check_idle()?;
        if self.workspace.profile() != AppearanceProfile::Reference {
            return Err("Console detail selection requires Reference appearance".into());
        }
        let result = self.console_panel.select(identity, &self.console);
        if let Err(error) = &result {
            self.status.clone_from(error);
        }
        result
    }

    /// Replace only the existing Console literal message filter.
    pub fn set_console_filter(&mut self, filter: String) {
        self.console_panel.set_filter(filter, &self.console);
    }

    /// Replace only the existing Console severity filter values.
    pub fn set_console_severities(&mut self, severities: &[Severity]) {
        self.console_panel.set_severities(severities, &self.console);
    }

    /// Append an independent editor record through the unchanged bounded Console API.
    pub fn append_console_record(
        &mut self,
        severity: Severity,
        message: &str,
    ) -> Result<u64, String> {
        let sequence = self
            .console
            .push(severity, message)
            .map_err(|error| error.to_string())?;
        if self.workspace.profile() == AppearanceProfile::Reference {
            self.console_panel.reconcile(&self.console);
        }
        Ok(sequence)
    }

    /// Clear retained Console records using existing counter-preserving semantics.
    pub fn clear_console_records(&mut self) {
        self.console.clear();
        if self.workspace.profile() == AppearanceProfile::Reference {
            self.console_panel.reconcile(&self.console);
        }
    }

    /// Explicitly bind a new empty Console lifetime before any new record selection events.
    /// Capacity validation and lifetime allocation both precede replacement; failure is atomic.
    pub fn replace_console_owner(&mut self, capacity: usize) -> Result<(), String> {
        let console = Console::new(capacity).map_err(|error| error.to_string())?;
        self.console_panel.reset_owner()?;
        self.console = console;
        Ok(())
    }

    pub(super) fn report(&mut self, result: Result<(), String>, success: &str) {
        let (severity, message) = match result {
            Ok(()) => (Severity::Info, success.to_owned()),
            Err(error) => (Severity::Error, error),
        };
        self.status = message.clone();
        if let Err(error) = self.console.push(severity, &message) {
            self.status = format!("{message}; console recording failed: {error}");
        }
    }
}
