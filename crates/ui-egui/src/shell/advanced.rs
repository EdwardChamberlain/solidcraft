//! The Advanced (NX) layout, top to bottom:
//!
//! - the ribbon (HOME, ASSEMBLY, ANALYSIS, VIEW, TOOLS; SKETCH while a sketch is open);
//! - the selection bar: a pane switch for the navigators, the selection filter and the scope;
//! - the navigator pane on the left (the Part Navigator, or the Assembly Navigator);
//! - the viewport;
//! - the cue line at the bottom, which says what to pick or do next.
//!
//! Everything it draws is shared with the Classic layout; only the arrangement differs.

use egui::RichText;

use crate::theme::Tokens;
use crate::{SolveApp, agent_chat, palette, preview, render_workspace, toolbar, viewport};

/// Which navigator the left pane shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Pane {
    /// The feature tree: every feature in the order it replays, with the rollback marker.
    #[default]
    Part,
    /// The components of the design (placements, bodies and joints).
    Assembly,
}

fn pane_id() -> egui::Id {
    egui::Id::new("sc_advanced_pane")
}

/// The navigator pane now showing.
pub fn pane(ctx: &egui::Context) -> Pane {
    ctx.data(|d| d.get_temp::<bool>(pane_id())).map_or(Pane::Part, |assembly| if assembly { Pane::Assembly } else { Pane::Part })
}

pub fn set_pane(ctx: &egui::Context, pane: Pane) {
    ctx.data_mut(|d| d.insert_temp(pane_id(), pane == Pane::Assembly));
}

pub fn show(app: &mut SolveApp, ui: &mut egui::Ui) {
    toolbar::toolbar(app, ui);
    selection_bar(app, ui);
    cue_line(app, ui);
    palette::palette_bar(app, ui);
    if app.ui.show_browser {
        match pane(ui.ctx()) {
            Pane::Part => super::part_nav::show(app, ui),
            Pane::Assembly => super::assembly_nav::show(app, ui),
        }
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

/// The strip under the ribbon: which navigator is open, and what a click in the viewport picks.
fn selection_bar(app: &mut SolveApp, ui: &mut egui::Ui) {
    let t = Tokens::get();
    egui::Panel::top("sc_selection_bar").exact_size(28.0).frame(egui::Frame::NONE.fill(t.panel).inner_margin(egui::Margin::symmetric(8, 3))).show(
        ui,
        |ui| {
            ui.horizontal_centered(|ui| {
                let current = pane(ui.ctx());
                for (which, label) in [(Pane::Part, "Part Navigator"), (Pane::Assembly, "Assembly Navigator")] {
                    let shown = app.ui.show_browser && current == which;
                    let response = ui.selectable_label(shown, app.tr_label(label));
                    crate::scenario::publish_rect(&format!("navigator:{}", if which == Pane::Part { "part" } else { "assembly" }), response.rect);
                    if response.clicked() {
                        // Clicking the open pane's tab hides the pane, as a resource bar does.
                        if shown {
                            app.ui.show_browser = false;
                        } else {
                            app.ui.show_browser = true;
                            set_pane(ui.ctx(), which);
                        }
                    }
                }
                ui.separator();
                ui.menu_button(app.tr_label("Selection filter"), |ui| toolbar::selection_filter(app, ui));
            });
        },
    );
}

/// What to do next: the dialog's open pick, the sketch's state, or the last thing that happened.
pub fn cue_text(app: &SolveApp) -> String {
    if let Some(d) = &app.dialog {
        return match d.inputs.iter().position(|i| i.items.is_empty()) {
            Some(i) => {
                let input = d.inputs.get(i).map_or("Objects", |input| input.label);
                crate::i18n::format(
                    "dialog.selection_step",
                    &[("step", &(i + 1).to_string()), ("total", &d.inputs.len().to_string()), ("input", app.tr_label(input))],
                )
            }
            None => app.tr("ui.cue_ready").to_string(),
        };
    }
    if let Some(id) = app.session.active_sketch
        && let Some(ss) = app.session.world_state().sketch(id)
    {
        let name = ss.name.to_string();
        return if !ss.report.ok() {
            crate::i18n::format("ui.cue_sketch_conflict", &[("name", &name)])
        } else if ss.report.dof == 0 {
            crate::i18n::format("ui.cue_sketch_full", &[("name", &name)])
        } else {
            crate::i18n::format("ui.cue_sketch_free", &[("name", &name), ("dof", &ss.report.dof.to_string())])
        };
    }
    if let Some((text, _, _)) = &app.status {
        return text.clone();
    }
    String::new()
}

fn cue_line(app: &mut SolveApp, ui: &mut egui::Ui) {
    let t = Tokens::get();
    let text = cue_text(app);
    egui::Panel::bottom("sc_cue_line").exact_size(24.0).frame(egui::Frame::NONE.fill(t.panel).inner_margin(egui::Margin::symmetric(8, 3))).show(
        ui,
        |ui| {
            ui.horizontal_centered(|ui| {
                let response = ui.label(RichText::new(text).size(12.0).color(t.text));
                crate::scenario::publish_rect("cue_line", response.rect);
            });
        },
    );
}
