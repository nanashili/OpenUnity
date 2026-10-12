use super::*;

impl EditorApp {
    pub(super) fn command(&mut self, command: Command) {
        let availability = shortcuts::StateAvailability {
            can_undo: self.can_undo(),
            can_redo: self.can_redo(),
            has_selection: self.imported.as_ref().map_or_else(
                || self.document.selection().primary().is_some(),
                |d| d.primary().is_some(),
            ),
            has_save_path: self.save_path.is_some() || !self.scene_text.trim().is_empty(),
        };
        let result = availability
            .availability(command)
            .map_err(str::to_owned)
            .and_then(|()| match command {
                Command::NewScene => self.request_new(),
                Command::OpenScene => self.request_open(&PathBuf::from(self.scene_text.trim())),
                Command::Save => self.save_current(),
                Command::Undo => self.undo().map(|_| ()),
                Command::Redo => self.redo().map(|_| ()),
                Command::DeleteSelection => self.apply_action(EditorAction::DeleteSelection),
                Command::ClearSelection => {
                    if let Some(document) = &mut self.imported {
                        document.select(None, false).map_err(|e| e.to_string())?;
                    } else {
                        self.document.selection_mut().1.clear();
                    }
                    self.reconcile_inspector();
                    Ok(())
                }
            });
        self.report(result, "Document command completed.");
    }

    pub(super) fn shortcuts(&mut self, ctx: &Context) {
        if self.pending.is_some() || self.path_form.is_some() {
            return;
        }
        let commands = self.shortcut_commands(ctx);
        for command in commands {
            match command {
                Command::OpenScene => self.open_form(PathFormKind::OpenScene),
                Command::Save if self.save_path.is_none() => self.open_form(PathFormKind::SaveAs),
                _ => self.command(command),
            }
        }
    }

    fn shortcut_commands(&self, ctx: &Context) -> Vec<Command> {
        let text_focus = ctx.wants_keyboard_input();
        // Leave Escape available for egui to dismiss this actual open popup, but
        // do not also interpret that dismissal as a global selection command.
        let inspector_menu_open = self.workspace.profile() == AppearanceProfile::Reference
            && self
                .inspector_menu_popup
                .is_some_and(|id| egui::Popup::is_id_open(ctx, id));
        ctx.input(|input| {
            input
                .events
                .iter()
                .filter_map(|event| {
                    let egui::Event::Key {
                        key,
                        pressed: true,
                        repeat: false,
                        modifiers,
                        ..
                    } = event
                    else {
                        return None;
                    };
                    if *key == egui::Key::Escape && inspector_menu_open {
                        return None;
                    }
                    let key = match key {
                        egui::Key::N => Key::N,
                        egui::Key::O => Key::O,
                        egui::Key::S => Key::S,
                        egui::Key::Z => Key::Z,
                        egui::Key::Y => Key::Y,
                        egui::Key::Delete => Key::Delete,
                        egui::Key::Escape => Key::Escape,
                        _ => return None,
                    };
                    shortcuts::resolve(
                        Chord {
                            key,
                            modifiers: Modifiers {
                                ctrl: modifiers.ctrl,
                                command: modifiers.command,
                                shift: modifiers.shift,
                                alt: modifiers.alt,
                            },
                        },
                        text_focus,
                    )
                })
                .collect::<Vec<_>>()
        })
    }
}
