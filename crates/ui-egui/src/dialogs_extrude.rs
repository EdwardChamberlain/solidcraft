//! The Extrude dialog of the Advanced approach, laid out the way the feature is described there:
//! **Section** (what to extrude), **Direction** (a vector), **Limits** (a Start and an End, each
//! with its own type and distance), **Offset** (a wall around the section, in place of "thin"),
//! **Draft** and **Boolean**.
//!
//! This is a different arrangement of the parameters the command already has: the Start type and
//! End type together are the start offset, the distance, the other side's distance, symmetric,
//! "from object", "to object" and through all; Offset is the thin wall; the vector is the profile
//! normal, forward or reversed.

use egui::Ui;
use serde_json::json;
use solidcraft_engine::Sel;
use solidcraft_engine::geom::Vec3;

use crate::SolveApp;
use crate::dialogs::{self, Dialog, FROM_OBJECT, Kind, OP_LABELS, TWO_SIDES, field, section};
use crate::ui_kit;

const START_TYPES: [&str; 3] = ["Value", "Value (Other Side)", "Until Selected"];
const END_TYPES: [&str; 5] = ["Value", "Value from Start", "Symmetric Value", "Until Selected", "Through All"];
const OFFSET_TYPES: [&str; 5] = ["None", "Symmetric", "Inside", "Outside", "Two-Sided"];

/// The way the profile faces: the normal of the sketch of the first pick (or of a planar face).
fn profile_normal(app: &SolveApp, d: &Dialog) -> Option<Vec3> {
    match d.inputs.first()?.items.first()? {
        Sel::Profile { sketch, .. } => app.session.world_state().sketch(*sketch).and_then(|s| s.plane.normal().normalized()),
        Sel::Face { body, index, .. } => dialogs::planar_face(&app.session, body, *index).and_then(|(_, n)| n.normalized()),
        _ => None,
    }
}

/// +1 when the specified direction runs along the profile normal, -1 against it; an error when it
/// runs any other way (an extrude goes along its profile's normal).
pub fn direction_sign(app: &SolveApp, d: &Dialog) -> Result<f64, String> {
    let Some(v) = crate::specifier::vector_raw(app, d, 0)? else { return Ok(1.0) };
    let Some(n) = profile_normal(app, d) else { return Ok(1.0) };
    let dot = v.normalized().map_or(0.0, |v| v.dot(n));
    if dot > 1.0 - 1e-6 {
        Ok(1.0)
    } else if dot < -1.0 + 1e-6 {
        Ok(-1.0)
    } else {
        Err("not supported yet: an extrude direction other than along or against the profile normal".into())
    }
}

/// Keep the dialog's one-sided direction and the vector's Reverse the same thing: whichever one
/// changed since the last frame (the picked side, a drag on the canvas, or the checkbox) wins.
fn sync_direction(app: &SolveApp, d: &mut Dialog) {
    let sign = direction_sign(app, d).unwrap_or(1.0);
    let Some(spec) = d.specs.first().cloned() else { return };
    let Kind::Extrude { direction, smart, auto_op, .. } = &mut d.kind else { return };
    if *direction >= 2 {
        return;
    }
    let negative = *direction == 1;
    let wanted = (sign < 0.0) ^ spec.reverse;
    let mut spec = spec;
    if spec.memo == Some(negative) {
        if wanted != negative {
            *direction = usize::from(wanted);
            smart.manual_dir = true;
            *auto_op = false;
        }
    } else {
        spec.reverse = negative ^ (sign < 0.0);
    }
    spec.memo = Some(*direction == 1);
    if let Some(slot) = d.specs.first_mut() {
        *slot = spec;
    }
}

/// The Advanced Extrude rows. `true` when Enter was pressed in a value.
pub fn rows(app: &SolveApp, d: &mut Dialog, ui: &mut Ui) -> bool {
    let mut enter = false;
    sync_direction(app, d);

    if section(ui, "extrude_section", "Section", true) {
        for k in 0..d.inputs.len() {
            if d.inputs.get(k).is_some_and(|i| i.owner == 0 && i.label != dialogs::TO_OBJECT && i.label != FROM_OBJECT) {
                dialogs::input_rows_range(d, ui, &app.session, k..k + 1);
            }
        }
    }

    enter |= crate::specifier::show(app, d, ui, 0, "Direction", "extrude_direction");
    if let Err(why) = direction_sign(app, d) {
        ui_kit::row(ui, "", |ui| {
            ui.label(egui::RichText::new(why).color(crate::theme::Tokens::get().error));
        });
    }
    sync_direction(app, d);

    if section(ui, "extrude_limits", "Limits", true) {
        enter |= limits(app, d, ui);
    }
    if section(ui, "extrude_offset", "Offset", false) {
        enter |= offset(d, ui);
    }
    let Kind::Extrude { direction, taper, taper_present, operation, auto_op, .. } = &mut d.kind else { return enter };
    if *direction < 2 && section(ui, "extrude_draft", "Draft", *taper_present) {
        ui_kit::row(ui, "Taper Angle", |ui| {
            let before = taper.clone();
            enter |= field(ui, taper);
            if *taper != before {
                *taper_present = !taper.trim().is_empty();
            }
        });
    }
    if section(ui, "extrude_boolean", "Boolean", true) {
        ui_kit::row(ui, "Operation", |ui| {
            let before = *operation;
            dialogs::combo(ui, "ex_op", &OP_LABELS, operation);
            // A choice made by hand sticks.
            if *operation != before {
                *auto_op = false;
            }
        });
    }
    enter
}

/// A length as the dialog writes one it worked out.
fn mm(v: f64) -> String {
    format!("{} mm", format!("{v:.6}").trim_end_matches('0').trim_end_matches('.'))
}

/// The distance from the start that reaches `end` (measured from the profile plane) when the start
/// is `start` along the normal.
fn from_start(app: &SolveApp, end: &str, start: &str) -> String {
    if start.trim().is_empty() {
        return end.to_string();
    }
    let length = solidcraft_engine::doc::expr::Kind::Length;
    match (app.session.doc.eval(end, length), app.session.doc.eval(start, length)) {
        (Ok(e), Ok(s)) => mm(e - s),
        _ => format!("({end}) - ({start})"),
    }
}

/// The end, measured from the profile plane, that a distance from the start reaches.
fn from_plane(app: &SolveApp, distance: &str, start: &str) -> String {
    if start.trim().is_empty() {
        return distance.to_string();
    }
    let length = solidcraft_engine::doc::expr::Kind::Length;
    match (app.session.doc.eval(distance, length), app.session.doc.eval(start, length)) {
        (Ok(d), Ok(s)) => mm(d + s),
        _ => format!("({distance}) + ({start})"),
    }
}

/// Start and End, each a type and a distance.
fn limits(app: &SolveApp, d: &mut Dialog, ui: &mut Ui) -> bool {
    let mut enter = false;
    let from_object = dialogs::object_on(&d.inputs, FROM_OBJECT);
    let to_object = dialogs::to_object_on(&d.inputs);
    let Kind::Extrude { direction, start, distance, distance2, to_offset, all, plane_end, smart, auto_op, .. } = &mut d.kind else { return false };

    // The End as the dialog stands.
    let end_now = if to_object {
        3
    } else if *all {
        4
    } else if *direction == 2 {
        2
    } else if plane_end.is_some() {
        0
    } else {
        1
    };

    // Start: where the extrude begins. Not for a symmetric one, which has no start of its own.
    let mut to_from_object = from_object;
    let mut start_type = if from_object {
        2
    } else if *direction == TWO_SIDES {
        1
    } else {
        0
    };
    if end_now != 2 {
        let current = start_type;
        ui_kit::row(ui, "Start", |ui| {
            dialogs::combo(ui, "ex_start_type", &START_TYPES, &mut start_type);
        });
        if start_type != current {
            let reverse = *direction == 1;
            *direction = if start_type == 1 { TWO_SIDES } else { usize::from(reverse) };
            to_from_object = start_type == 2;
            smart.manual_dir = true;
            *auto_op = false;
        }
        match start_type {
            0 => {
                ui_kit::row(ui, "Distance", |ui| {
                    enter |= field(ui, start);
                });
            }
            1 => {
                ui_kit::row(ui, "Distance", |ui| {
                    enter |= field(ui, distance2);
                });
            }
            _ => {}
        }
    }

    // End.
    let mut end = end_now;
    ui_kit::row(ui, "End", |ui| {
        dialogs::combo(ui, "ex_end_type", &END_TYPES, &mut end);
    });
    if end != end_now {
        *all = end == 4;
        match end {
            0 => *plane_end = Some((from_plane(app, distance, start), distance.clone())),
            _ => *plane_end = None,
        }
        if end == 2 {
            *direction = 2;
            start.clear();
            to_from_object = false;
            d.extra.insert("convention".into(), json!(solidcraft_engine::doc::ExtrudeConvention::TotalDistance));
        } else if *direction == 2 {
            *direction = 0;
        }
        smart.manual_dir = true;
        *auto_op = false;
    }
    match end {
        0 => {
            // Measured from the profile plane: what was typed, and the distance from the start it
            // comes to. The distance changing by itself (a drag on the canvas) is read back.
            if let Some((typed, made)) = plane_end.as_mut() {
                if made != distance {
                    *typed = from_plane(app, distance, start);
                    made.clone_from(distance);
                } else {
                    let want = from_start(app, typed, start);
                    if want != *distance {
                        distance.clone_from(&want);
                        *made = want;
                    }
                }
                ui_kit::row(ui, "Distance", |ui| {
                    if field(ui, typed) {
                        enter = true;
                    }
                    let want = from_start(app, typed, start);
                    if want != *distance {
                        distance.clone_from(&want);
                        *made = want;
                    }
                });
            }
        }
        1 | 2 => {
            ui_kit::row(ui, if end == 2 { "Distance (total)" } else { "Distance" }, |ui| {
                enter |= field(ui, distance);
            });
        }
        3 => {
            ui_kit::row(ui, "Offset", |ui| {
                enter |= field(ui, to_offset);
            });
        }
        _ => {}
    }
    dialogs::set_object(&mut d.inputs, &mut d.active, FROM_OBJECT, to_from_object);
    dialogs::set_to_object(&mut d.inputs, &mut d.active, end == 3);
    // The picks of an Until Selected sit with their limit.
    for (label, wanted) in [(FROM_OBJECT, to_from_object), (dialogs::TO_OBJECT, end == 3)] {
        if wanted && let Some(at) = d.inputs.iter().position(|i| i.label == label) {
            dialogs::input_rows_range(d, ui, &app.session, at..at + 1);
        }
    }
    enter
}

/// The wall around the section: what "thin" was, as an offset from it.
fn offset(d: &mut Dialog, ui: &mut Ui) -> bool {
    let mut enter = false;
    let Kind::Extrude { thin, wall, wall_side, wall2, .. } = &mut d.kind else { return false };
    // None, Symmetric (centre), Inside, Outside, Two-Sided: the wall sides the command has.
    let current = if !*thin {
        0
    } else {
        match *wall_side {
            0 => 1,
            1 => 2,
            2 => 3,
            _ => 4,
        }
    };
    let mut picked = current;
    ui_kit::row(ui, "Type", |ui| {
        dialogs::combo(ui, "ex_offset_type", &OFFSET_TYPES, &mut picked);
    });
    if picked != current {
        *thin = picked != 0;
        *wall_side = match picked {
            1 => 0,
            2 => 1,
            3 => 2,
            4 => 3,
            _ => *wall_side,
        };
        if *thin && wall.trim().is_empty() {
            *wall = "2 mm".into();
        }
        dialogs::set_thin(&mut d.inputs, *thin);
    }
    if *thin {
        let two = *wall_side == 3;
        ui_kit::row(
            ui,
            if two {
                "Start Offset"
            } else if *wall_side == 0 {
                "Offset (total)"
            } else {
                "Offset"
            },
            |ui| {
                enter |= field(ui, wall);
            },
        );
        if two {
            ui_kit::row(ui, "End Offset", |ui| {
                enter |= field(ui, wall2);
            });
        }
    }
    enter
}
