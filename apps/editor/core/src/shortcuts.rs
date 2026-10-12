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
mod tests;
