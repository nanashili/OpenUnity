//! Independent editor shortcut resolution and state-dependent availability.
//!
//! The frontend supplies explicit keys and modifier state. Text input has first
//! refusal: while it has focus, every global shortcut in this module is ignored,
//! including Save and Undo. This preserves the text widget's editing behavior.

/// Keys recognized by the independent editor's global shortcut resolver.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Key {
    /// Letter N.
    N,
    /// Letter O.
    O,
    /// Letter S.
    S,
    /// Letter Z.
    Z,
    /// Letter Y.
    Y,
    /// Delete key.
    Delete,
    /// Escape key.
    Escape,
}

/// Modifier state provided by the frontend, independent of its UI toolkit.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Modifiers {
    /// Control is pressed.
    pub ctrl: bool,
    /// Platform command modifier is pressed.
    pub command: bool,
    /// Shift is pressed.
    pub shift: bool,
    /// Alt is pressed.
    pub alt: bool,
}

/// One key press with its complete modifier state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Chord {
    /// Pressed key.
    pub key: Key,
    /// Complete modifier state.
    pub modifiers: Modifiers,
}

/// Actions supported by the independent editor shortcut contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Command {
    /// Begin a new independent scene.
    NewScene,
    /// Choose an independent scene to open.
    OpenScene,
    /// Save to the selected independent path.
    Save,
    /// Restore the previous transaction.
    Undo,
    /// Restore the next transaction.
    Redo,
    /// Remove selected subtrees.
    DeleteSelection,
    /// Clear document selection.
    ClearSelection,
}

/// Resolve a global shortcut without executing it or checking document state.
///
/// Either Control or Command enables letter shortcuts. Both may be true because
/// a frontend can report Command as its platform's logical shortcut modifier.
/// Alt is never accepted; Shift is accepted only for Control/Command+Shift+Z.
/// Delete and Escape require no modifiers. Text focus suppresses all commands.
pub fn resolve(chord: Chord, text_input: bool) -> Option<Command> {
    if text_input || chord.modifiers.alt {
        return None;
    }
    let modifiers = chord.modifiers;
    if modifiers.ctrl || modifiers.command {
        return match (chord.key, modifiers.shift) {
            (Key::N, false) => Some(Command::NewScene),
            (Key::O, false) => Some(Command::OpenScene),
            (Key::S, false) => Some(Command::Save),
            (Key::Z, false) => Some(Command::Undo),
            (Key::Z, true) | (Key::Y, false) => Some(Command::Redo),
            _ => None,
        };
    }
    if modifiers.shift {
        return None;
    }
    match chord.key {
        Key::Delete => Some(Command::DeleteSelection),
        Key::Escape => Some(Command::ClearSelection),
        _ => None,
    }
}

/// A caller-owned snapshot of state needed to enable supported commands.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StateAvailability {
    /// An undo snapshot is available.
    pub can_undo: bool,
    /// A redo snapshot is available.
    pub can_redo: bool,
    /// At least one entity is selected.
    pub has_selection: bool,
    /// An explicit save path has been selected.
    pub has_save_path: bool,
}

impl StateAvailability {
    /// Check availability and return a user-facing reason when disabled.
    ///
    /// New/Open remain available so the shell can handle unsaved work before
    /// replacing a document. Save requires an explicitly selected save path;
    /// selecting that path is a separate frontend operation. Unsupported actions
    /// are deliberately absent from [`Command`].
    pub fn availability(self, command: Command) -> Result<(), &'static str> {
        match command {
            Command::Undo if !self.can_undo => Err("Nothing to undo."),
            Command::Redo if !self.can_redo => Err("Nothing to redo."),
            Command::DeleteSelection | Command::ClearSelection if !self.has_selection => {
                Err("No entities selected.")
            }
            Command::Save if !self.has_save_path => {
                Err("Select an independent OpenUnity scene save path first.")
            }
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chord(key: Key, modifiers: Modifiers) -> Chord {
        Chord { key, modifiers }
    }

    #[test]
    fn control_and_command_resolve_all_letter_shortcuts() {
        for modifiers in [
            Modifiers {
                ctrl: true,
                ..Modifiers::default()
            },
            Modifiers {
                command: true,
                ..Modifiers::default()
            },
            Modifiers {
                ctrl: true,
                command: true,
                ..Modifiers::default()
            },
        ] {
            for (key, command) in [
                (Key::N, Command::NewScene),
                (Key::O, Command::OpenScene),
                (Key::S, Command::Save),
                (Key::Z, Command::Undo),
                (Key::Y, Command::Redo),
            ] {
                assert_eq!(resolve(chord(key, modifiers), false), Some(command));
            }
            assert_eq!(
                resolve(
                    chord(
                        Key::Z,
                        Modifiers {
                            shift: true,
                            ..modifiers
                        }
                    ),
                    false
                ),
                Some(Command::Redo)
            );
        }
    }

    #[test]
    fn only_exact_supported_modifiers_resolve() {
        for ctrl in [false, true] {
            for command in [false, true] {
                for shift in [false, true] {
                    for alt in [false, true] {
                        let modifiers = Modifiers {
                            ctrl,
                            command,
                            shift,
                            alt,
                        };
                        for key in [
                            Key::N,
                            Key::O,
                            Key::S,
                            Key::Z,
                            Key::Y,
                            Key::Delete,
                            Key::Escape,
                        ] {
                            let resolved = resolve(chord(key, modifiers), false);
                            if alt || (shift && key != Key::Z) {
                                assert_eq!(resolved, None, "{key:?} {modifiers:?}");
                            } else if matches!(key, Key::Delete | Key::Escape) {
                                assert_eq!(resolved.is_some(), !ctrl && !command && !shift);
                            } else {
                                assert_eq!(resolved.is_some(), ctrl || command);
                            }
                        }
                    }
                }
            }
        }
        assert_eq!(
            resolve(chord(Key::Delete, Modifiers::default()), false),
            Some(Command::DeleteSelection)
        );
        assert_eq!(
            resolve(chord(Key::Escape, Modifiers::default()), false),
            Some(Command::ClearSelection)
        );
    }

    #[test]
    fn text_input_consumes_every_global_shortcut() {
        for key in [
            Key::N,
            Key::O,
            Key::S,
            Key::Z,
            Key::Y,
            Key::Delete,
            Key::Escape,
        ] {
            for modifiers in [
                Modifiers::default(),
                Modifiers {
                    ctrl: true,
                    ..Modifiers::default()
                },
                Modifiers {
                    command: true,
                    shift: true,
                    ..Modifiers::default()
                },
            ] {
                assert_eq!(resolve(chord(key, modifiers), true), None);
            }
        }
    }

    #[test]
    fn availability_reports_reasons_and_enables_valid_state() {
        let disabled = StateAvailability::default();
        assert_eq!(disabled.availability(Command::NewScene), Ok(()));
        assert_eq!(disabled.availability(Command::OpenScene), Ok(()));
        assert_eq!(
            disabled.availability(Command::Undo),
            Err("Nothing to undo.")
        );
        assert_eq!(
            disabled.availability(Command::Redo),
            Err("Nothing to redo.")
        );
        assert_eq!(
            disabled.availability(Command::DeleteSelection),
            Err("No entities selected.")
        );
        assert_eq!(
            disabled.availability(Command::ClearSelection),
            Err("No entities selected.")
        );
        assert_eq!(
            disabled.availability(Command::Save),
            Err("Select an independent OpenUnity scene save path first.")
        );
        let enabled = StateAvailability {
            can_undo: true,
            can_redo: true,
            has_selection: true,
            has_save_path: true,
        };
        for command in [
            Command::NewScene,
            Command::OpenScene,
            Command::Save,
            Command::Undo,
            Command::Redo,
            Command::DeleteSelection,
            Command::ClearSelection,
        ] {
            assert_eq!(enabled.availability(command), Ok(()));
        }
    }
}
