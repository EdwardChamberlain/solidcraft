//! Specify Vector for the Advanced approach: a direction input that takes whatever names one,
//! an axis, a straight edge or sketch line (along it) or a planar face (its normal), and turns it
//! into the plain direction the commands already take. Flip stays the dialogs' own Flip toggle.
//!
//! Specify Point: the place a To Object or From Object extent goes to or starts from. It takes a
//! vertex or construction point, a face (where it was picked) or an edge (where it was picked),
//! and the commands get the plain point.

use solidcraft_engine::Sel;
use solidcraft_engine::geom::Vec3;

use crate::SolveApp;
use crate::selection::SelInput;
use crate::selection::{AXES, Accept, EDGES, FACES, PLANAR_FACES, VERTICES};

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

/// What a point input accepts in the Advanced approach.
pub const POINT: Accept = FACES | VERTICES | EDGES | crate::selection::SKETCH_POINTS;

/// Let the point inputs (To Object, From Object) take what Specify Point names, in the Advanced
/// approach; Classic keeps faces and vertices.
pub fn widen_points(app: &SolveApp, inputs: &mut [SelInput], labels: &[&str]) {
    if app.preferences.modelling_approach != crate::prefs::ModellingApproach::Advanced {
        return;
    }
    for input in inputs.iter_mut().filter(|i| labels.contains(&i.label)) {
        input.accept |= POINT;
    }
}
