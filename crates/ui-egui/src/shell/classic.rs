//! The Classic (Fusion 360) layout: ribbon, parts browser, feature history, palette strip.

use crate::{SolveApp, agent_chat, browser, palette, preview, render_workspace, timeline, timeline_left, toolbar, viewport};

pub fn show(app: &mut SolveApp, ui: &mut egui::Ui) {
    toolbar::toolbar(app, ui);
    let show_timeline = app.ui.show_timeline && !render_workspace::active(app);
    let left_timeline = show_timeline && app.preferences.timeline_left;
    if show_timeline && !left_timeline {
        timeline::timeline(app, ui);
    }
    palette::palette_bar(app, ui);
    if app.ui.show_browser {
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
    preview::update(app, ui.ctx());
    egui::CentralPanel::default().frame(egui::Frame::NONE).show(ui, |ui| {
        if render_workspace::active(app) {
            render_workspace::show(app, ui);
        } else {
            viewport::show(app, ui);
        }
    });
}
