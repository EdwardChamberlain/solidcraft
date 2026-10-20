//! Dialogs of the Advanced approach whose plane is a selector row (a specifier): Mirror and Draft.
//! Each has its inputs and values in groups, the plane a row with its icon.

use egui::Ui;

use crate::SolveApp;
use crate::dialogs::{self, Dialog, Kind, field, row_label, section};

/// The rows of a dialog that has a plane selector. `true` when Enter was pressed in a value.
pub fn rows(app: &SolveApp, d: &mut Dialog, ui: &mut Ui) -> bool {
    let mut enter = false;
    match &d.kind {
        Kind::Mirror => {
            if section(ui, "mirror_objects", "Objects", true) {
                for k in 0..d.inputs.len() {
                    if d.inputs.get(k).is_some_and(|i| i.owner == 0) {
                        dialogs::input_rows_range(d, ui, &app.session, k..k + 1);
                    }
                }
            }
            if section(ui, "mirror_plane", "Mirror Plane", true) {
                enter |= crate::specifier::show(app, d, ui, 0, "Plane", "mirror_plane_selector");
            }
            // Feature replay retains the source operation; Join is for body copies.
            if !d.inputs.first().is_some_and(|i| i.accept == crate::selection::FEATURES) && section(ui, "mirror_settings", "Settings", true) {
                row_label(ui, "Operation");
                let mut op = usize::from(d.extra.get("combine").and_then(serde_json::Value::as_bool).unwrap_or(false));
                dialogs::combo(ui, "mi_op", &["New Body", "Join"], &mut op);
                d.extra.insert("combine".into(), serde_json::json!(op == 1));
                ui.end_row();
            }
        }
        Kind::Draft { .. } => {
            if section(ui, "draft_faces", "Faces", true) {
                for k in 0..d.inputs.len() {
                    if d.inputs.get(k).is_some_and(|i| i.owner == 0) {
                        dialogs::input_rows_range(d, ui, &app.session, k..k + 1);
                    }
                }
            }
            if section(ui, "draft_plane", "Neutral Plane", true) {
                enter |= crate::specifier::show(app, d, ui, 0, "Plane", "draft_plane_selector");
            }
            if section(ui, "draft_angle", "Angle", true)
                && let Kind::Draft { angle } = &mut d.kind
            {
                row_label(ui, "Angle");
                enter |= field(ui, angle);
                ui.end_row();
            }
        }
        _ => {}
    }
    enter
}

/// Does the dialog take these rows (Advanced, with its plane a selector)?
pub fn applies(app: &SolveApp, d: &Dialog) -> bool {
    app.preferences.modelling_approach == crate::prefs::ModellingApproach::Advanced
        && !d.specs.is_empty()
        && matches!(d.kind, Kind::Mirror | Kind::Draft { .. })
}
