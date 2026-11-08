use egui::Modifiers;
use serde_json::json;

use super::*;
use crate::canvas::tool_event;

fn type_app_with_selection() -> PhotocraftApp {
    let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), Default::default());
    app.run("file.new", json!({"width": 120, "height": 80})).unwrap();
    app.run("type.create", json!({"x": 10, "y": 40, "text": "Hello"})).unwrap();
    app.run("select.rect", json!({"x": 8, "y": 16, "width": 40, "height": 30})).unwrap();
    app.ui.tool = Tool::ContentAwareMove;
    app.ui.tool_options.sample_all.content_aware_move = false;
    app.ui.tool_options.cam_transform_on_drop = false;
    app
}

fn active_content(app: &PhotocraftApp) -> &'static str {
    let st = app.session.active().unwrap();
    st.doc.layer(st.active_layer.unwrap()).unwrap().content.kind_name()
}

fn past(app: &PhotocraftApp) -> usize {
    app.session.active().unwrap().history.past_len()
}

fn prompt(app: &PhotocraftApp) -> Option<&crate::state::Dialog> {
    app.ui.dialogs.iter().find(|d| owns(&d.fields))
}

fn drag(app: &mut PhotocraftApp, from: [f64; 2], to: [f64; 2]) {
    tool_event(app, ToolEvent::Down { x: from[0], y: from[1], pressure: 1.0 }, Modifiers::NONE);
    tool_event(app, ToolEvent::Move { x: to[0], y: to[1], pressure: 1.0 }, Modifiers::NONE);
    tool_event(app, ToolEvent::Up { x: to[0], y: to[1] }, Modifiers::NONE);
}

#[test]
fn content_aware_move_on_type_layer_prompts_to_rasterize_and_ok_runs_move() {
    let mut app = type_app_with_selection();
    let before = past(&app);

    // Pressing inside the selection does not prompt until the non-zero drag is released.
    tool_event(&mut app, ToolEvent::Down { x: 20.0, y: 30.0, pressure: 1.0 }, Modifiers::NONE);
    assert!(prompt(&app).is_none(), "mouse-down alone must not ask to rasterize");
    tool_event(&mut app, ToolEvent::Move { x: 45.0, y: 35.0, pressure: 1.0 }, Modifiers::NONE);
    tool_event(&mut app, ToolEvent::Up { x: 45.0, y: 35.0 }, Modifiers::NONE);

    let d = prompt(&app).expect("releasing a move drag on a type layer opens the rasterize prompt");
    assert_eq!(d.fields["message"], json!(Kind::Type.message()));
    assert_eq!(past(&app), before, "nothing happens before answering the prompt");
    assert_eq!(active_content(&app), "Type");

    let id = d.id;
    let r = crate::dialogs::confirm(&mut app, id).unwrap();
    assert_eq!(r["painted"], json!(true));
    assert_eq!(active_content(&app), "Pixel");
    assert_eq!(past(&app), before + 2, "rasterize type and content-aware move are two history states");
    assert_eq!(app.session.journal.last().map(|(cmd, _)| cmd.as_str()), Some("paint.contentAwareMove"));
}

#[test]
fn content_aware_move_on_type_layer_with_transform_on_drop_opens_box_after_rasterize() {
    let mut app = type_app_with_selection();
    app.ui.tool_options.cam_transform_on_drop = true;
    let before = past(&app);

    drag(&mut app, [20.0, 30.0], [44.0, 36.0]);
    assert!(app.ui.cam_transform.is_none(), "Transform On Drop waits for the rasterize prompt");
    let id = prompt(&app).expect("rasterize prompt opened").id;

    crate::dialogs::confirm(&mut app, id).unwrap();
    assert_eq!(active_content(&app), "Pixel");
    assert_eq!(past(&app), before + 1, "rasterized first");
    assert!(app.ui.cam_transform.is_some(), "Transform On Drop box opened on the rasterized layer");

    crate::cam_transform::commit(&mut app);
    assert!(app.ui.cam_transform.is_none());
    assert_eq!(past(&app), before + 2, "committing Transform On Drop records the second history state");
}

#[test]
fn content_aware_move_cancel_leaves_type_layer_and_selection_untouched() {
    let mut app = type_app_with_selection();
    let before = (past(&app), app.session.active().unwrap().doc.clone());

    drag(&mut app, [20.0, 30.0], [50.0, 40.0]);
    let id = prompt(&app).expect("rasterize prompt opened").id;
    crate::dialogs::cancel(&mut app, id).unwrap();

    assert!(app.ui.dialogs.is_empty());
    assert_eq!(past(&app), before.0);
    assert_eq!(active_content(&app), "Type");
    assert_eq!(*app.session.active().unwrap().doc, *before.1, "document and selection are untouched after Cancel");
    assert!(app.session.active().unwrap().doc.selection.as_ref().is_some_and(|s| s.sample_channel(20, 30, 0) > 0.0));
}

#[test]
fn drawing_new_selection_or_sample_all_layers_on_type_layer_does_not_prompt() {
    // Drawing a new selection with Content-Aware Move on a type layer does not ask to rasterize.
    let mut app = type_app_with_selection();
    app.run("select.deselect", json!({})).unwrap();
    tool_event(&mut app, ToolEvent::Down { x: 10.0, y: 10.0, pressure: 1.0 }, Modifiers::NONE);
    tool_event(&mut app, ToolEvent::Move { x: 30.0, y: 10.0, pressure: 1.0 }, Modifiers::NONE);
    tool_event(&mut app, ToolEvent::Move { x: 30.0, y: 30.0, pressure: 1.0 }, Modifiers::NONE);
    tool_event(&mut app, ToolEvent::Up { x: 10.0, y: 30.0 }, Modifiers::NONE);
    assert!(prompt(&app).is_none(), "drawing a lasso selection on a type layer must not ask to rasterize");
    assert!(app.session.active().unwrap().doc.selection.is_some());
    assert_eq!(active_content(&app), "Type");

    // With Sample All Layers on, moving refuses a type layer without asking to rasterize.
    let mut app = type_app_with_selection();
    app.ui.tool_options.sample_all.content_aware_move = true;
    drag(&mut app, [20.0, 30.0], [45.0, 35.0]);
    assert!(prompt(&app).is_none(), "Sample All Layers on must not ask to rasterize");
    assert!(app.ui.status_error);
    assert_eq!(active_content(&app), "Type");
}
