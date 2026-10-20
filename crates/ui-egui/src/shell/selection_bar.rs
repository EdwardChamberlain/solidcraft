//! The Advanced selection bar: what a click may pick. A **type filter** (all, bodies, faces,
//! edges, vertices, sketch geometry; the Classic checkbox filter is under Customize), a **scope**
//! (the whole design, only the work part, or the work part and its components) and an **intent**
//! (one edge, or the edges that continue it smoothly). They apply to clicks with no dialog and to
//! picks into a dialog's inputs alike.

use egui::RichText;

use crate::selection::{Intent, SNAP_KINDS, Scope};
use crate::theme::Tokens;
use crate::{SolveApp, UiState, toolbar};

/// The type filters: label, whether a face picks its body, and the kinds a click does not pick.
pub const FILTERS: &[(&str, bool, &[&str])] = &[
    ("All", false, &[]),
    ("Bodies", true, &[]),
    ("Faces", false, &["edges", "vertices", "sketch", "sketch_curves", "sketch_points"]),
    ("Edges", false, &["faces", "vertices", "sketch", "sketch_curves", "sketch_points"]),
    ("Vertices", false, &["faces", "edges", "sketch", "sketch_curves", "sketch_points"]),
    ("Sketch Geometry", false, &["faces", "edges", "vertices"]),
];

pub const SCOPES: &[(&str, Scope)] =
    &[("Entire Design", Scope::Entire), ("Work Part Only", Scope::WorkPart), ("Work Part and Components", Scope::WorkPartAndComponents)];

pub const INTENTS: &[(&str, Intent)] = &[
    ("Single", Intent::Single),
    ("Tangent Chain", Intent::TangentChain),
    ("Face Edges", Intent::FaceEdges),
    ("Connected Curves", Intent::Connected),
];

/// The filter the UI state stands for; `None` when it is a mix of its own (Customize).
pub fn current_filter(ui: &UiState) -> Option<usize> {
    FILTERS.iter().position(|(_, bodies, off)| {
        *bodies == ui.pick_bodies && off.len() == ui.pick_off.len() && off.iter().all(|k| ui.pick_off.iter().any(|x| x == k))
    })
}

pub fn set_filter(ui: &mut UiState, index: usize) {
    if let Some((_, bodies, off)) = FILTERS.get(index) {
        ui.pick_bodies = *bodies;
        ui.pick_off = off.iter().map(|k| (*k).to_string()).collect();
    }
}

pub fn show(app: &mut SolveApp, ui: &mut egui::Ui) {
    let t = Tokens::get();
    egui::Panel::top("sc_selection_bar").exact_size(28.0).frame(egui::Frame::NONE.fill(t.panel).inner_margin(egui::Margin::symmetric(8, 3))).show(
        ui,
        |ui| {
            ui.horizontal_centered(|ui| {
                // Type filter.
                let filter = current_filter(&app.ui);
                let label = filter.map_or("Custom", |i| FILTERS[i].0);
                let r = ui.menu_button(RichText::new(app.tr_label(label)), |ui| {
                    for (i, (name, ..)) in FILTERS.iter().enumerate() {
                        let r = ui.selectable_label(filter == Some(i), app.tr_label(name));
                        crate::scenario::publish_handle(&format!("selection:filter:{name}"), r.rect.center());
                        if r.clicked() {
                            set_filter(&mut app.ui, i);
                            ui.close();
                        }
                    }
                    ui.separator();
                    ui.menu_button(app.tr_label("Customize"), |ui| toolbar::selection_filter(app, ui));
                });
                crate::scenario::publish_handle("selection:filter", r.response.rect.center());
                r.response.on_hover_text(app.tr_label("Type filter"));
                // Scope.
                let label = SCOPES.iter().find(|(_, s)| *s == app.ui.selection_scope).map_or("", |(n, _)| *n);
                let r = ui.menu_button(RichText::new(app.tr_label(label)), |ui| {
                    for (name, scope) in SCOPES {
                        let r = ui.selectable_label(app.ui.selection_scope == *scope, app.tr_label(name));
                        crate::scenario::publish_handle(&format!("selection:scope:{name}"), r.rect.center());
                        if r.clicked() {
                            app.ui.selection_scope = *scope;
                            ui.close();
                        }
                    }
                });
                crate::scenario::publish_handle("selection:scope", r.response.rect.center());
                r.response.on_hover_text(app.tr_label("Scope"));
                // Intent.
                let label = INTENTS.iter().find(|(_, i)| *i == app.ui.selection_intent).map_or("", |(n, _)| *n);
                let r = ui.menu_button(RichText::new(app.tr_label(label)), |ui| {
                    for (name, intent) in INTENTS {
                        let r = ui.selectable_label(app.ui.selection_intent == *intent, app.tr_label(name));
                        crate::scenario::publish_handle(&format!("selection:intent:{name}"), r.rect.center());
                        if r.clicked() {
                            app.ui.selection_intent = *intent;
                            ui.close();
                        }
                    }
                });
                crate::scenario::publish_handle("selection:intent", r.response.rect.center());
                r.response.on_hover_text(app.tr_label("Intent"));
                // Snap points: which inferences the sketch tools make.
                let off = SNAP_KINDS.iter().filter(|(_, bit)| app.ui.snap_kinds & bit == 0).count();
                let label = if off == 0 { "Snap Points".to_string() } else { format!("Snap Points ({} off)", off) };
                let r = ui.menu_button(RichText::new(app.tr_label(&label)), |ui| {
                    for (name, bit) in SNAP_KINDS {
                        let mut on = app.ui.snap_kinds & bit != 0;
                        let r = ui.checkbox(&mut on, app.tr_label(name));
                        crate::scenario::publish_handle(&format!("selection:snap:{name}"), r.rect.center());
                        if r.changed() {
                            app.ui.snap_kinds = if on { app.ui.snap_kinds | bit } else { app.ui.snap_kinds & !bit };
                        }
                    }
                });
                crate::scenario::publish_handle("selection:snap", r.response.rect.center());
                r.response.on_hover_text(app.tr_label("Snap points"));
            });
        },
    );
}
