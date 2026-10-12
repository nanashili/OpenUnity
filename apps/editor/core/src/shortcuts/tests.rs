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
