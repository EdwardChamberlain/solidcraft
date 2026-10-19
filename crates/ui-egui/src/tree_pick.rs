//! Picking from the feature tree: while a dialog waits for a pick, a click on a feature of the
//! tree answers it, wherever the input takes what the feature stands for: a pattern's features, a
//! sketch's profiles or curves, a datum plane, axis or point, the bodies a feature made.

use solidcraft_engine::Sel;
use solidcraft_engine::doc::FeatureKind;

use crate::SolveApp;
use crate::selection::{Accept, BODIES, FEATURES, PLANES, POINTS, PROFILES, VERTICES};

/// What feature `id` stands for in an input that takes `accept`. With `all` false only the
/// feature itself (a pattern's features) counts, as in the timeline strip of the Classic layout.
pub fn sels_for(app: &SolveApp, accept: Accept, id: u64, all: bool) -> Vec<Sel> {
    let Some(feature) = app.session.doc.feature(id) else { return Vec::new() };
    if accept & FEATURES != 0 {
        return vec![Sel::Feature { id }];
    }
    if !all {
        return Vec::new();
    }
    let st = app.session.world_state();
    match &feature.kind {
        FeatureKind::Sketch { .. } => {
            let Some(sketch) = st.sketch(id) else { return Vec::new() };
            if accept & PROFILES != 0 {
                return (0..sketch.profiles.len()).map(|index| Sel::Profile { sketch: id, index }).collect();
            }
            Vec::new()
        }
        FeatureKind::ConstructionPlane { .. } if accept & PLANES != 0 => vec![Sel::Plane { name: feature.name.clone() }],
        FeatureKind::ConstructionAxis { .. } if accept & crate::selection::AXES != 0 => vec![Sel::Axis { name: feature.name.clone() }],
        FeatureKind::ConstructionPoint { .. } if accept & (VERTICES | POINTS) != 0 => solidcraft_engine::view::construction_points(&app.session)
            .into_iter()
            .find(|(fid, ..)| *fid == id)
            .map(|(_, _, point)| vec![Sel::Vertex { body: String::new(), point }])
            .unwrap_or_default(),
        _ if accept & BODIES != 0 => crate::shell::part_nav_tree::bodies_of(app, id).into_iter().map(|name| Sel::Body { name }).collect(),
        _ => Vec::new(),
    }
}

/// A click on feature `id` of the tree: into the open dialog's active input when it takes what the
/// feature stands for. `true` when it did, so the click is not also a selection.
pub fn pick_feature(app: &mut SolveApp, id: u64, all: bool) -> bool {
    if app.applying.is_some() {
        return false;
    }
    let Some(accept) = app.dialog.as_ref().and_then(|d| d.active_input()).map(|i| i.accept) else { return false };
    let sels = sels_for(app, accept, id, all);
    if sels.is_empty() {
        return false;
    }
    let Some(mut d) = app.dialog.take() else { return false };
    for sel in sels {
        d.pick(&app.session, sel);
    }
    app.dialog = Some(d);
    true
}
