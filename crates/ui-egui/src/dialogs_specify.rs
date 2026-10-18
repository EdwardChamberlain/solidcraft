//! Specify Vector for the Advanced approach: a direction input that takes whatever names one,
//! an axis, a straight edge or sketch line (along it) or a planar face (its normal), and turns it
//! into the plain direction the commands already take. Flip stays the dialogs' own Flip toggle.

use solidcraft_engine::Sel;
use solidcraft_engine::geom::Vec3;

use crate::SolveApp;
use crate::selection::{AXES, Accept, EDGES, PLANAR_FACES};

/// What a vector input accepts in the Advanced approach.
pub const VECTOR: Accept = AXES | EDGES | PLANAR_FACES;

/// What a vector input accepts in the current approach.
pub fn accept(app: &SolveApp) -> Accept {
    if app.preferences.modelling_approach == crate::prefs::ModellingApproach::Advanced { VECTOR } else { AXES | EDGES }
}

/// The (point, direction) a pick stands for.
pub fn vector_of(app: &SolveApp, sel: &Sel) -> Option<(Vec3, Vec3)> {
    match sel {
        Sel::Face { body, index, .. } => crate::dialogs::planar_face(&app.session, body, *index),
        other => crate::dialogs::axis_of(app, other),
    }
}
