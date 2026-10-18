//! The window layouts, one per modelling approach (Preferences › General › Modelling approach).
//!
//! Everything below the layout (commands, dialog bodies, the viewport, previews) is shared; a shell
//! only decides which panels surround the viewport and in what order. `SolveApp::cad_ui` runs the
//! parts every layout needs, then hands the panels to [`show`].

pub mod advanced;
pub mod assembly_nav;
pub mod classic;
pub mod part_nav;

use crate::SolveApp;
use crate::prefs::ModellingApproach;

/// Lay out the panels around the viewport for the chosen approach.
pub fn show(app: &mut SolveApp, ui: &mut egui::Ui) {
    match app.preferences.modelling_approach {
        ModellingApproach::Classic => classic::show(app, ui),
        ModellingApproach::Advanced => advanced::show(app, ui),
    }
}
