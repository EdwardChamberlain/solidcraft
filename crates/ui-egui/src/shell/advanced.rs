//! The Advanced (NX) layout.
//!
//! Until its own panels land (the resource bar with the Part Navigator, the selection bar, the cue
//! line) it lays out like Classic, so choosing the approach never leaves the window empty.

use crate::SolveApp;

pub fn show(app: &mut SolveApp, ui: &mut egui::Ui) {
    super::classic::show(app, ui);
}
