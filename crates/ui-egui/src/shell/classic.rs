//! The Classic (Fusion 360) layout: ribbon, parts browser, feature history, and the floating shell
//! cards (or the docked panels, by `ui.set {layout}`).

use crate::ui_kit::{Container, Surface};
use crate::{SolveApp, agent_chat, browser, preview, render_workspace, shell, theme, timeline, timeline_left, toolbar, ui_kit, viewport};

pub fn show(app: &mut SolveApp, ui: &mut egui::Ui) {
    // The selection bar's intent is the Advanced layout's.
    crate::selection::set_intent(crate::selection::Intent::Single);
    crate::selection::set_snaps(crate::selection::SNAPS_ALL);
    let floating = ui_kit::Container::new(ui.ctx(), ui_kit::Surface::Panel).floating();
    let show_timeline = app.ui.show_timeline && !render_workspace::active(app);
    let left_timeline = show_timeline && app.preferences.timeline_left;
    timeline_left::animate_folds(app, ui.ctx());
    if !floating {
        toolbar::toolbar(app, ui);
    }
    theme::TabAccent::publish(app, ui.ctx());
    if !floating {
        agent_chat::show(app, ui);
    }
    if !floating && show_timeline && !left_timeline {
        timeline::timeline(app, ui);
    }
    // Browser hugs its contents in either layout; it never becomes a full-height blank column.
    if !app.ui.show_browser {
        app.shell.browser = None;
    }
    if floating || left_timeline || !show_timeline {
        app.shell.history = None;
    }
    let full = ui.available_rect_before_wrap();
    if floating {
        let cards = shell::Cards::new(full, app.ui.layout, toolbar::ribbon_height(app.preferences.ribbon_labels));
        app.shell.cards = vec![cards.workspace, cards.ribbon];
        app.shell.history = if left_timeline {
            let below_ribbon = egui::Rect::from_min_max(egui::pos2(full.left(), cards.bottom), full.max);
            Some(timeline_left::floating_rect(app, below_ribbon))
        } else {
            show_timeline.then(|| timeline::floating_rect(full))
        };
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
    if left_timeline {
        timeline_left::left_panel(app, ui);
    }
    if render_workspace::active(app) {
        render_workspace::panel(app, ui);
    }
    if app.ui.show_browser {
        browser::browser(app, ui);
    }
    if floating {
        agent_chat::show(app, ui);
    }
    Container::new(ui.ctx(), Surface::Viewport)
        .central()
        .frame(Container::new(ui.ctx(), Surface::Viewport).edge_frame(egui::Color32::TRANSPARENT))
        .show(ui, |ui| {
            let panel_content = ui.max_rect();
            app.shell.content = Some(shell::workspace_content(app, panel_content));
            if floating {
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
