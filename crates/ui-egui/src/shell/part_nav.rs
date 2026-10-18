//! The Part Navigator (Advanced approach): the feature tree on top, in the order it replays, with
//! the rollback marker; below it, the selected feature's Dependencies and Details.
//!
//! The list is the same one the Classic layout can show at the left (`timeline_left`): every
//! feature in creation order, sketches where they were made, never nested under what uses them.

use egui::{RichText, vec2};
use serde_json::json;

use crate::theme::Tokens;
use crate::{SolveApp, timeline, timeline_left};

/// What the lower panes say about one feature.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Facts {
    pub id: u64,
    pub name: String,
    pub kind: String,
    pub suppressed: bool,
    pub error: Option<String>,
    /// (label, expression) of each value the feature takes.
    pub inputs: Vec<(String, String)>,
    /// The sketches the feature is made from.
    pub parents: Vec<String>,
    /// Features deleted along with it, and those that would fail without it.
    pub children: Vec<String>,
    pub would_fail: Vec<String>,
}

/// The facts of a feature: parents and values are cheap; children need the design evaluated
/// without the feature, so they are asked for separately ([`children_of`]).
pub fn facts(app: &SolveApp, id: u64) -> Option<Facts> {
    let doc = &app.session.doc;
    let f = doc.feature(id)?;
    Some(Facts {
        id,
        name: f.name.clone(),
        kind: f.kind.type_name().to_string(),
        suppressed: f.suppressed,
        error: app.session.model.result(id).and_then(|r| r.error.clone()),
        inputs: f.kind.inputs().into_iter().map(|(label, e, _)| (label.to_string(), e.to_string())).collect(),
        parents: f.kind.input_sketches().into_iter().filter_map(|sid| doc.feature(sid).map(|p| p.name.clone())).collect(),
        children: Vec::new(),
        would_fail: Vec::new(),
    })
}

/// What deleting the feature takes with it and breaks (`timeline.dependents`).
pub fn children_of(app: &SolveApp, id: u64) -> (Vec<String>, Vec<String>) {
    let mut scratch = app.session.scratch();
    let Ok(r) = scratch.execute("timeline.dependents", &json!({"feature": id})) else { return (Vec::new(), Vec::new()) };
    let names = |key: &str| -> Vec<String> {
        r.get(key).and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default()
    };
    (names("deleted_with_it"), names("would_fail"))
}

fn cache_id() -> egui::Id {
    egui::Id::new("sc_part_nav_children")
}

pub fn show(app: &mut SolveApp, ui: &mut egui::Ui) {
    let t = Tokens::get();
    egui::Panel::left("sc_part_navigator")
        .exact_size(270.0)
        .resizable(false)
        .frame(egui::Frame::NONE.fill(t.panel).inner_margin(egui::Margin::symmetric(6, 6)))
        .show(ui, |ui| {
            let total = ui.available_height();
            ui.allocate_ui(vec2(ui.available_width(), (total * 0.58).max(120.0)), |ui| timeline_left::left_timeline(app, ui));
            ui.separator();
            panes(app, ui);
        });
}

fn panes(app: &mut SolveApp, ui: &mut egui::Ui) {
    let t = Tokens::get();
    let Some(id) = timeline::selected_feature_ids(app).first().copied() else {
        ui.label(RichText::new(app.tr("ui.part_nav_pick")).color(t.text_dim));
        return;
    };
    let Some(mut facts) = facts(app, id) else { return };
    // Children cost an evaluation: asked once per design revision and feature, not per frame.
    let key = (app.session.revision, id);
    let cached: Option<((u64, u64), (Vec<String>, Vec<String>))> = ui.data(|d| d.get_temp(cache_id()));
    let (children, would_fail) = match cached {
        Some((k, v)) if k == key => v,
        _ => {
            let v = children_of(app, id);
            ui.data_mut(|d| d.insert_temp(cache_id(), (key, v.clone())));
            v
        }
    };
    facts.children = children;
    facts.would_fail = would_fail;
    egui::ScrollArea::vertical().id_salt("sc_part_nav_panes").auto_shrink([false, false]).show(ui, |ui| {
        egui::CollapsingHeader::new(RichText::new(app.tr("ui.dependencies")).strong().size(12.0)).default_open(true).show(ui, |ui| {
            list(app, ui, "ui.parents", &facts.parents);
            list(app, ui, "ui.children", &facts.children);
            list(app, ui, "ui.would_fail", &facts.would_fail);
        });
        egui::CollapsingHeader::new(RichText::new(app.tr("ui.details")).strong().size(12.0)).default_open(true).show(ui, |ui| {
            ui.label(RichText::new(format!("{} — {}", facts.name, facts.kind)).strong());
            if facts.suppressed {
                ui.label(RichText::new(app.tr("ui.suppressed")).color(t.text_dim));
            }
            if let Some(e) = &facts.error {
                ui.label(RichText::new(e).color(t.error));
            }
            egui::Grid::new("sc_part_nav_inputs").num_columns(2).spacing(vec2(8.0, 2.0)).show(ui, |ui| {
                for (label, value) in &facts.inputs {
                    ui.label(RichText::new(app.tr_label(label)).color(t.text_dim));
                    ui.label(value);
                    ui.end_row();
                }
            });
            let edit = ui.button(app.tr("ui.edit_feature"));
            crate::scenario::publish_rect("part_nav:edit", edit.rect);
            if edit.clicked() {
                app.edit_feature(id);
            }
        });
    });
}

fn list(app: &SolveApp, ui: &mut egui::Ui, heading: &str, names: &[String]) {
    if names.is_empty() {
        return;
    }
    let t = Tokens::get();
    ui.label(RichText::new(app.tr(heading)).size(11.0).color(t.text_dim));
    for name in names {
        ui.label(format!("  {name}"));
    }
}
