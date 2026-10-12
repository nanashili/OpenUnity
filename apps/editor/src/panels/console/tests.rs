//! Existing Console input and retention assertions.

use eframe::egui::{
    CentralPanel, Context, Event, FullOutput, Modifiers, PointerButton, Pos2, RawInput, Rect, Vec2,
    epaint::Shape,
};

use super::*;

fn frame(
    ctx: &Context,
    panel: &mut ConsolePanel,
    console: &mut Console,
    events: Vec<Event>,
) -> FullOutput {
    ctx.run(
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
            events,
            ..Default::default()
        },
        |ctx| {
            CentralPanel::default().show(ctx, |ui| panel.show(ui, console));
        },
    )
}

fn text_rows(output: &FullOutput) -> Vec<(String, Pos2)> {
    output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            Shape::Text(text) => Some((
                text.galley.text().to_owned(),
                text.pos + text.galley.size() * 0.5,
            )),
            _ => None,
        })
        .collect()
}

fn click(
    ctx: &Context,
    panel: &mut ConsolePanel,
    console: &mut Console,
    output: &FullOutput,
    label: &str,
) -> FullOutput {
    let pos = text_rows(output)
        .into_iter()
        .find_map(|(text, pos)| (text == label).then_some(pos))
        .unwrap_or_else(|| panic!("missing visible control: {label}"));
    frame(
        ctx,
        panel,
        console,
        vec![
            Event::PointerMoved(pos),
            Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
            Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed: false,
                modifiers: Modifiers::NONE,
            },
        ],
    )
}

fn visible_records(output: &FullOutput) -> Vec<String> {
    text_rows(output)
        .into_iter()
        .map(|(text, _)| text)
        .filter(|text| text.starts_with('#'))
        .collect()
}

#[test]
fn pointer_severity_and_typed_message_filters_preserve_live_record_order() {
    let mut console = Console::new(8).unwrap();
    console.push(Severity::Info, "asset opened").unwrap();
    console.push(Severity::Warning, "scene changed").unwrap();
    console.push(Severity::Error, "asset unavailable").unwrap();
    console.push(Severity::Warning, "asset stale").unwrap();
    let mut panel = ConsolePanel::default();
    let ctx = Context::default();
    let output = frame(&ctx, &mut panel, &mut console, Vec::new());
    assert_eq!(
        visible_records(&output),
        [
            "#1 [Info] asset opened",
            "#2 [Warning] scene changed",
            "#3 [Error] asset unavailable",
            "#4 [Warning] asset stale",
        ]
    );
    let output = click(&ctx, &mut panel, &mut console, &output, "Info");
    assert_eq!(visible_records(&output).len(), 3);
    let _ = click(
        &ctx,
        &mut panel,
        &mut console,
        &output,
        "Search message text",
    );
    let output = frame(
        &ctx,
        &mut panel,
        &mut console,
        vec![Event::Text("asset".to_owned())],
    );
    assert_eq!(
        visible_records(&output),
        ["#3 [Error] asset unavailable", "#4 [Warning] asset stale"]
    );
    let output = click(&ctx, &mut panel, &mut console, &output, "Warning");
    assert_eq!(visible_records(&output), ["#3 [Error] asset unavailable"]);
    let output = click(&ctx, &mut panel, &mut console, &output, "Error");
    assert!(visible_records(&output).is_empty());
    assert!(
        text_rows(&output)
            .iter()
            .any(|(text, _)| text == "No records match the current filters.")
    );
    assert_eq!(console.records().len(), 4);
    assert_eq!(console.dropped_count(), 0);
}

#[test]
fn clear_button_removes_actual_records_and_preserves_visible_overflow() {
    let mut console = Console::new(1).unwrap();
    console.push(Severity::Info, "evicted").unwrap();
    console.push(Severity::Error, "retained").unwrap();
    let mut panel = ConsolePanel::default();
    let ctx = Context::default();
    let output = frame(&ctx, &mut panel, &mut console, Vec::new());
    assert_eq!(visible_records(&output), ["#2 [Error] retained"]);
    assert!(
        text_rows(&output)
            .iter()
            .any(|(text, _)| text.contains("Dropped records: 1"))
    );
    let output = click(&ctx, &mut panel, &mut console, &output, "Error");
    assert!(visible_records(&output).is_empty());
    assert_eq!(console.records().len(), 1);
    let output = click(&ctx, &mut panel, &mut console, &output, "Clear");
    assert_eq!(console.records().len(), 0);
    assert_eq!(console.dropped_count(), 1);
    assert!(visible_records(&output).is_empty());
    assert!(
        text_rows(&output)
            .iter()
            .any(|(text, _)| text == "No editor records.")
    );
    assert!(
        text_rows(&output)
            .iter()
            .any(|(text, _)| text.contains("Dropped records: 1"))
    );
    assert_eq!(console.push(Severity::Warning, "after clear"), Ok(3));
    let output = frame(&ctx, &mut panel, &mut console, Vec::new());
    assert_eq!(visible_records(&output), ["#3 [Warning] after clear"]);
}
