//! The Classic (Fusion 360) layout: ribbon, parts browser, feature history, and the floating shell
//! cards (or the docked panels, by `ui.set {layout}`).

use crate::{SolveApp, agent_chat, browser, preview, render_workspace, shell, theme, timeline, timeline_left, toolbar, viewport};

pub fn show(app: &mut SolveApp, ui: &mut egui::Ui) {
    // The selection bar's intent is the Advanced layout's.
    crate::selection::set_intent(crate::selection::Intent::Single);
    crate::selection::set_snaps(crate::selection::SNAPS_ALL);
    let floating = app.ui.layout == shell::LayoutMode::Floating;
    let show_timeline = app.ui.show_timeline && !render_workspace::active(app);
    let left_timeline = show_timeline && app.preferences.timeline_left;
    if !floating {
        toolbar::toolbar(app, ui);
    }
    theme::TabAccent::publish(app, ui.ctx());
    if !floating && show_timeline && !left_timeline {
        timeline::timeline(app, ui);
    }
    // A floating browser keeps the workspace the whole width and covers viewport input.
    let browser_float = app.ui.show_browser && (app.preferences.browser_float || (floating && !left_timeline));
    if !browser_float {
        app.shell.browser = None;
    }
    app.shell.history = None;
    let full = ui.available_rect_before_wrap();
    if floating {
        let cards = shell::Cards::new(full, app.ui.layout);
        app.shell.cards = vec![cards.workspace, cards.ribbon];
        app.shell.history = (show_timeline && !left_timeline).then(|| timeline::floating_rect(full));
        // Draw the scene first, then let the unchanged panel layouts cover it. The
        // viewport keeps its full projection even underneath the browser and cards.
        preview::update(app, ui.ctx());
        // A camera drag that began on the model keeps capture across cards. New
        // gestures over chrome are blocked, as are selection drags into the ribbon.
        let continuing_navigation = ui.input(|i| {
            (i.pointer.middle_down() || i.pointer.secondary_down())
                && i.pointer.press_origin().is_some_and(|p| full.contains(p) && !shell::covered(app, p))
        });
        let blocked = !continuing_navigation && ui.input(|i| i.pointer.latest_pos()).is_some_and(|p| shell::covered(app, p));
        let mut viewport_ui = ui.new_child(egui::UiBuilder::new().max_rect(full).id_salt("floating_viewport"));
        if blocked {
            // Disabled only to shield the scene from input under a card or menu; it must
            // not fade, or the whole viewport dims whenever the pointer is over the shell.
            viewport_ui.visuals_mut().disabled_alpha = 1.0;
            viewport_ui.disable();
        }
        app.show_workspace(&mut viewport_ui);
        shell::paint_top_corners(ui.painter(), full, theme::Tokens::get().app_bar);
        toolbar::toolbar(app, ui);
        theme::TabAccent::publish(app, ui.ctx());
    }
    if floating && show_timeline && !left_timeline {
        timeline::timeline(app, ui);
    }
    if app.ui.show_browser && !browser_float {
        if left_timeline {
            browser::browser_below_timeline(app, ui);
        } else {
            browser::browser(app, ui);
        }
    } else if left_timeline {
        timeline_left::left_panel(app, ui);
    }
    if render_workspace::active(app) {
        render_workspace::panel(app, ui);
    }
    agent_chat::show(app, ui);
    if browser_float {
        // The remaining content starts below the ribbon in either shell layout.
        let viewport = ui.available_rect_before_wrap();
        browser::floating(app, ui, viewport);
    }
    egui::CentralPanel::default().frame(egui::Frame::NONE).show(ui, |ui| {
        if floating {
            let panel_content = ui.max_rect();
            let mut content = panel_content;
            if let Some(history) = app.shell.history {
                content.max.y = (history.top() - shell::SHELL_GAP).max(content.top());
            }
            if let Some(browser) = app.shell.browser {
                content.min.x = (browser.right() + shell::SHELL_GAP).min(content.right());
            }
            app.shell.content = Some(content);
            app.shell.panels = [
                egui::Rect::from_min_max(egui::pos2(full.left(), panel_content.top()), egui::pos2(panel_content.left(), full.bottom())),
                egui::Rect::from_min_max(egui::pos2(panel_content.right(), panel_content.top()), full.max),
            ]
            .into_iter()
            .filter(|r| r.is_positive())
            .collect();
            viewport::refresh_layout(app, ui.ctx().pixels_per_point());
        } else {
            preview::update(app, ui.ctx());
            app.show_workspace(ui);
        }
    });
}
