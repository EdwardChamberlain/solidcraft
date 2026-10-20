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
use crate::{SolveApp, agent_chat, preview, render_workspace, theme, toolbar, viewport};

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
    // What picking an edge takes with it follows the selection bar.
    crate::selection::set_intent(app.ui.selection_intent);
    crate::selection::set_snaps(app.ui.snap_kinds);
    // A part just made the work part: its Part Navigator (and the banner) comes forward.
    if let Some(work) = app.work.as_mut().filter(|w| w.fresh) {
        work.fresh = false;
        set_pane(ui.ctx(), Pane::Part);
        app.ui.show_browser = true;
    }
    toolbar::toolbar(app, ui);
    theme::TabAccent::publish(app, ui.ctx());
    super::selection_bar::show(app, ui);
    cue_line(app, ui);
    resource_bar(app, ui);
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

/// The resource bar at the left edge: one vertical tab for each navigator. Clicking the open
/// navigator's tab hides it, as a resource bar does.
fn resource_bar(app: &mut SolveApp, ui: &mut egui::Ui) {
    let t = Tokens::get();
    let current = pane(ui.ctx());
    egui::Panel::left("sc_resource_bar")
        .exact_size(30.0)
        .resizable(false)
        .frame(egui::Frame::NONE.fill(t.toolbar).inner_margin(egui::Margin::symmetric(2, 6)))
        .show(ui, |ui| {
            for (which, label, key) in [(Pane::Part, "Part Navigator", "part"), (Pane::Assembly, "Assembly Navigator", "assembly")] {
                let text = app.tr_label(label).to_string();
                let galley = ui.painter().layout_no_wrap(text.clone(), egui::FontId::proportional(t.font_label), t.text);
                let height = galley.size().x + 20.0;
                let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), height), egui::Sense::click());
                response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &text));
                crate::scenario::publish_rect(&format!("navigator:{key}"), rect);
                let shown = app.ui.show_browser && current == which;
                if shown {
                    ui.painter().rect_filled(rect, 3.0, t.accent.gamma_multiply(0.35));
                } else if response.hovered() {
                    ui.painter().rect_filled(rect, 3.0, t.hover);
                }
                // Text reads bottom to top, as on a resource bar.
                let colour = if shown { t.text } else { t.text_dim };
                let galley = ui.painter().layout_no_wrap(text, egui::FontId::proportional(t.font_label), colour);
                let at = egui::pos2(rect.center().x - galley.size().y / 2.0, rect.bottom() - 10.0);
                ui.painter().add(egui::epaint::TextShape::new(at, galley, colour).with_angle(-std::f32::consts::FRAC_PI_2));
                if response.clicked() {
                    if shown {
                        app.ui.show_browser = false;
                    } else {
                        app.ui.show_browser = true;
                        set_pane(ui.ctx(), which);
                    }
                }
            }
        });
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
    app.tr("ui.cue_idle").to_string()
}

fn cue_line(app: &mut SolveApp, ui: &mut egui::Ui) {
    let t = Tokens::get();
    let text = cue_text(app);
    egui::Panel::bottom("sc_cue_line").exact_size(24.0).frame(egui::Frame::NONE.fill(t.panel).inner_margin(egui::Margin::symmetric(8, 3))).show(
        ui,
        |ui| {
            ui.horizontal_centered(|ui| {
                let response = ui.label(RichText::new(text).size(t.font_label).color(t.text));
                crate::scenario::publish_rect("cue_line", response.rect);
            });
        },
    );
}
