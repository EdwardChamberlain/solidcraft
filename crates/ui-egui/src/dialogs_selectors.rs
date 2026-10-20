//! Dialogs of the Advanced approach whose plane, direction or axis is a selector row (a
//! specifier): Mirror, Draft, Section, and the linear and circular patterns. Each has its inputs
//! and values in groups, the plane, direction or axis a row with its icon.

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
        Kind::PatternRect { .. } => {
            if section(ui, "pattern_objects", "Objects", true) {
                for k in 0..d.inputs.len() {
                    if d.inputs.get(k).is_some_and(|i| i.owner == 0) {
                        dialogs::input_rows_range(d, ui, &app.session, k..k + 1);
                    }
                }
            }
            if section(ui, "pattern_direction", "Direction", true) {
                enter |= crate::specifier::show(app, d, ui, 0, "Direction", "pattern_direction_selector");
            }
            if section(ui, "pattern_direction2", "Second Direction", true) {
                enter |= crate::specifier::show(app, d, ui, 1, "Direction 2", "pattern_direction2_selector");
            }
        }
        Kind::PatternCirc { .. } => {
            if section(ui, "pattern_objects", "Objects", true) {
                for k in 0..d.inputs.len() {
                    if d.inputs.get(k).is_some_and(|i| i.owner == 0) {
                        dialogs::input_rows_range(d, ui, &app.session, k..k + 1);
                    }
                }
            }
            if section(ui, "pattern_axis", "Axis", true) {
                enter |= crate::specifier::show(app, d, ui, 0, "Axis", "pattern_axis_selector");
            }
        }
        Kind::Section { .. } if section(ui, "section_plane", "Section Plane", true) => {
            enter |= crate::specifier::show(app, d, ui, 0, "Plane", "section_plane_selector");
        }
        _ => {}
    }
    enter
}

/// The linear and circular patterns' direction and axis are selector rows in the Advanced
/// approach, for patterns of bodies: after the objects, one Vector specifier each (two for a
/// linear pattern, the second optional). Called wherever such a dialog gets its inputs afresh; what
/// was picked stays. A pattern of features keeps plain inputs ([`plain_pattern`]).
pub fn convert_pattern(app: &SolveApp, d: &mut Dialog) {
    use crate::specifier::{Class, Specifier, add};
    if app.preferences.modelling_approach != crate::prefs::ModellingApproach::Advanced {
        return;
    }
    let kept: Vec<Vec<solidcraft_engine::Sel>> = d.inputs.iter().skip(1).map(|i| i.items.clone()).collect();
    match d.kind {
        Kind::PatternRect { .. } => {
            d.inputs.truncate(1);
            d.specs.clear();
            add(app, d, Specifier::new(Class::Vector, 0));
            add(app, d, Specifier::new(Class::Vector, 0).optional());
        }
        Kind::PatternCirc { .. } => {
            d.inputs.truncate(1);
            d.specs.clear();
            add(app, d, Specifier::new(Class::Vector, 0));
        }
        _ => return,
    }
    // Each specifier's first input takes what its plain input held, as far as it fits.
    for (k, items) in kept.into_iter().enumerate() {
        if let Some(input) = d.inputs.iter_mut().find(|i| i.owner == k + 1) {
            input.items = items.into_iter().filter(|s| dialogs::fits(input.accept, s)).collect();
        }
    }
}

/// A pattern of features keeps its own layout, with plain direction and axis inputs: the
/// selector rows go, what was picked stays.
pub fn plain_pattern(app: &SolveApp, d: &mut Dialog) {
    use crate::selection::{AXES, SelInput};
    if d.specs.is_empty() {
        return;
    }
    let kept: Vec<Vec<solidcraft_engine::Sel>> =
        (1..=d.specs.len()).map(|owner| d.inputs.iter().find(|i| i.owner == owner).map(|i| i.items.clone()).unwrap_or_default()).collect();
    let vector = crate::dialogs_specify::accept(app);
    let plain = match d.kind {
        Kind::PatternRect { .. } => vec![SelInput::new("Direction", vector, false), SelInput::new("Direction 2", vector, false)],
        Kind::PatternCirc { .. } => vec![SelInput::new("Axis", AXES, false)],
        _ => return,
    };
    d.inputs.truncate(1);
    d.specs.clear();
    d.inputs.extend(plain);
    for (k, items) in kept.into_iter().enumerate() {
        if let Some(input) = d.inputs.get_mut(k + 1) {
            input.items = items.into_iter().filter(|s| dialogs::fits(input.accept, s)).collect();
        }
    }
}

/// Does the dialog take these rows (Advanced, with its plane, direction or axis a selector)?
pub fn applies(app: &SolveApp, d: &Dialog) -> bool {
    app.preferences.modelling_approach == crate::prefs::ModellingApproach::Advanced
        && !d.specs.is_empty()
        && matches!(d.kind, Kind::Mirror | Kind::Draft { .. } | Kind::Section { .. } | Kind::PatternRect { .. } | Kind::PatternCirc { .. })
}
