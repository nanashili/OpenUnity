use super::record_project_warnings;
use openunity_editor_core::console::{Console, MAX_MESSAGE_BYTES, Severity};
use openunity_editor_core::project::{ProjectDiagnostic, ProjectDiagnosticKind};
use std::path::PathBuf;

#[test]
fn rejected_project_warning_reports_context_and_stops_without_rollback() {
    let diagnostics = [
        ProjectDiagnostic {
            relative_path: PathBuf::from("first.asset"),
            kind: ProjectDiagnosticKind::UnsupportedEntry,
            message: "authored earlier warning".into(),
        },
        ProjectDiagnostic {
            relative_path: PathBuf::from("oversized.asset"),
            kind: ProjectDiagnosticKind::UnsupportedEntry,
            message: "x".repeat(MAX_MESSAGE_BYTES),
        },
        ProjectDiagnostic {
            relative_path: PathBuf::from("last.asset"),
            kind: ProjectDiagnosticKind::UnsupportedEntry,
            message: "must not be attempted".into(),
        },
    ];
    let mut console = Console::new(1).unwrap();
    console.push(Severity::Info, "older record").unwrap();

    let error = record_project_warnings(&mut console, &diagnostics).unwrap_err();

    assert!(error.contains("project was not opened"));
    assert!(error.contains("oversized.asset"));
    assert!(error.contains(&diagnostics[1].message));
    assert!(error.contains("console message exceeds 65536 UTF-8 bytes"));
    let retained = console.records().next().unwrap();
    assert_eq!(retained.sequence, 2);
    assert_eq!(retained.severity, Severity::Warning);
    assert_eq!(retained.message, "first.asset: authored earlier warning");
    assert_eq!(console.dropped_count(), 1);
    assert_eq!(
        console.push(Severity::Info, "subsequent record").unwrap(),
        3
    );
}
