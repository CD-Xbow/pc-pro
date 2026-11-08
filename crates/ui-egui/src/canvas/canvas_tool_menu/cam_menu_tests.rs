use super::*;
use egui::{Modifiers, PointerButton, Pos2, vec2};
use egui_kittest::Harness;

fn app() -> PhotocraftApp {
    let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), crate::Services::default());
    app.run("file.new", json!({"width": 32, "height": 32})).unwrap();
    app.ui.tool = Tool::ContentAwareMove;
    app
}

fn harness(mut app: PhotocraftApp) -> Harness<'static, PhotocraftApp> {
    app.sync_views();
    let mut h = Harness::builder().with_size(vec2(1000.0, 700.0)).build_ui_state(
        |ui, app: &mut PhotocraftApp| {
            let ctx = ui.ctx().clone();
            if !ctx.fonts(|f| f.families().contains(&egui::FontFamily::Name("medium".into()))) {
                return;
            }
            egui::CentralPanel::default().show(ui, |ui| crate::canvas::document_area(app, ui));
        },
        app,
    );
    PhotocraftApp::setup_context(&h.ctx, crate::theme::ThemeKind::default());
    h.run_steps(4);
    h
}

fn right_click(h: &mut Harness<'static, PhotocraftApp>, p: Pos2) {
    h.event(egui::Event::PointerMoved(p));
    h.run_steps(1);
    h.event(egui::Event::PointerButton { pos: p, button: PointerButton::Secondary, pressed: true, modifiers: Modifiers::NONE });
    h.run_steps(1);
    h.event(egui::Event::PointerButton { pos: p, button: PointerButton::Secondary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
}

#[test]
fn content_aware_move_outside_and_without_selection_matches_lasso_menus() {
    let mut app = app();
    let ctx = Context::default();
    assert!(applies(Tool::ContentAwareMove));

    // With no selection, right-clicking opens NO_SELECTION_MENU, matching Tool::Lasso.
    assert!(open_at(&mut app, &ctx, Tool::ContentAwareMove, [10.0, 10.0], Some([16.0, 16.0])));
    let menu = app.ui.canvas_tool_menu.clone().expect("no-selection menu opened");
    assert_eq!(rows(&menu), NO_SELECTION_MENU);
    choose(&mut app, &ctx, "select.all");
    assert!(app.session.active().unwrap().doc.selection.is_some());

    // Outside an active selection, right-clicking opens SELECTION_MENU, matching Tool::Lasso.
    app.run("select.rect", json!({"x": 2, "y": 2, "width": 8, "height": 8})).unwrap();
    assert!(open_at(&mut app, &ctx, Tool::ContentAwareMove, [10.0, 10.0], Some([24.0, 24.0])));
    let menu = app.ui.canvas_tool_menu.clone().expect("outside-selection menu opened");
    assert!(menu.has_selection && !inside_selection(&menu));
    assert_eq!(rows(&menu), SELECTION_MENU);
    choose(&mut app, &ctx, "select.deselect");
    assert!(app.session.active().unwrap().doc.selection.is_none());
}

#[test]
fn content_aware_move_inside_selection_opens_color_range_and_load_selection_only() {
    let mut app = app();
    app.run("select.rect", json!({"x": 4, "y": 4, "width": 24, "height": 24})).unwrap();
    let mut h = harness(app);
    let center = h.state().last_canvas_rect.center();
    right_click(&mut h, center);

    let menu = h.state().ui.canvas_tool_menu.clone().expect("inside-selection menu opened");
    assert!(menu.has_selection && inside_selection(&menu));
    assert_eq!(rows(&menu), CAM_INSIDE_SELECTION_MENU);
    let labels: Vec<_> = rows(&menu).iter().map(|r| r.map(|(l, _)| l)).collect();
    assert_eq!(labels, vec![Some("Color Range…"), Some("Load Selection…")]);

    // Color Range… is enabled; Load Selection… is greyed out when no selection channel is saved.
    assert!(entry_enabled(h.state(), &menu, "select.colorRange"));
    assert!(!entry_enabled(h.state(), &menu, "select.loadSelection"));
    assert!(!available(h.state(), &menu, "select.deselect"), "full selection menu rows are not available inside selection");

    // Once a selection channel is saved, Load Selection… becomes enabled.
    let ctx = h.ctx.clone();
    h.state_mut().run("select.saveSelection", json!({"name": "Alpha 1"})).unwrap();
    assert!(open_at(h.state_mut(), &ctx, Tool::ContentAwareMove, [10.0, 10.0], Some([16.0, 16.0])));
    let menu = h.state().ui.canvas_tool_menu.clone().unwrap();
    assert_eq!(rows(&menu), CAM_INSIDE_SELECTION_MENU);
    assert!(entry_enabled(h.state(), &menu, "select.loadSelection"));
}
