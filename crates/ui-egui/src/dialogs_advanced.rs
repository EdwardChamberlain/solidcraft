//! The Advanced approach's dialog rows. Its dialogs name a plane, a direction or a point through
//! selector rows (specifiers), group their rows, and keep the datum Type row; this is what the
//! dialog body draws for them in place of (or before) the common forms of `dialogs_dense`.

use super::*;
use crate::ui_kit;

/// What the Advanced layout drew of a dialog.
pub(super) enum Rows {
    /// All of it: nothing else is drawn (`true`: Enter was pressed in a value).
    Done(bool),
    /// Its picks: the common form goes on with the values below them.
    Values(bool),
    /// Nothing of its own: the common form draws it all.
    Not,
}

pub(super) fn rows(app: &SolveApp, d: &mut Dialog, ui: &mut egui::Ui) -> Rows {
    if app.preferences.modelling_approach != crate::prefs::ModellingApproach::Advanced {
        return Rows::Not;
    }
    if matches!(d.kind, Kind::Specify { .. }) {
        return Rows::Done(crate::specifier::sub_rows(app, d, ui));
    }
    if crate::specifier::is_sketch(d) {
        return Rows::Done(sketch_specifier_rows(app, d, ui));
    }
    if !d.specs.is_empty() {
        match d.kind {
            Kind::Extrude { .. } => return Rows::Done(crate::dialogs_extrude::rows(app, d, ui)),
            Kind::Revolve { .. } => return Rows::Done(crate::dialogs_revolve::rows(app, d, ui)),
            Kind::Datum { .. } => return Rows::Done(crate::dialogs_datum::frame_rows(app, d, ui)),
            _ => {}
        }
    }
    if crate::dialogs_selectors::applies(app, d) {
        // The Layout and Object Type rows of the patterns and the mirror come first.
        if matches!(d.kind, Kind::Mirror | Kind::PatternRect { .. } | Kind::PatternCirc { .. }) {
            placement_row(app, d, ui, crate::dialogs_specify::accept(app));
        }
        let enter = crate::dialogs_selectors::rows(app, d, ui);
        // Mirror and Draft draw their own values; the others' values follow in a Settings group.
        if matches!(d.kind, Kind::Mirror | Kind::Draft { .. }) {
            return Rows::Done(enter);
        }
        return if section(ui, "settings", "Settings", true) { Rows::Values(enter) } else { Rows::Done(enter) };
    }
    if matches!(d.kind, Kind::Datum { .. } | Kind::DatumInferred { .. }) {
        crate::dialogs_datum::type_row(app, d, ui);
        input_rows(d, ui, &app.session);
        let picked = d.inputs.first().map_or(0, |i| i.items.len());
        return Rows::Done(match &mut d.kind {
            Kind::Datum { method, value } => crate::dialogs_datum::value_row(ui, *method, value),
            Kind::DatumInferred { class, value } => crate::dialogs_datum::inferred_value_row(ui, *class, picked, value),
            _ => false,
        });
    }
    if let Kind::NewPart { name } = &mut d.kind {
        return Rows::Done(ui_kit::text_row(ui, "Part name", name, "").0);
    }
    // The plane dialogs keep their Type row at the top.
    crate::dialogs_datum::type_row(app, d, ui);
    if grouped_kind(d) {
        // What to pick, then the values: two groups, as the dialogs with their own layout have.
        if section(ui, "selection", "Selection", true) {
            input_rows(d, ui, &app.session);
        }
        return if section(ui, "settings", "Settings", true) { Rows::Values(false) } else { Rows::Done(false) };
    }
    Rows::Not
}
