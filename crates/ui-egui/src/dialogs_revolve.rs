//! The Revolve dialog of the Advanced approach, laid out as the feature is described there:
//! **Section** (what to revolve), **Axis** (a Specify Vector with Reverse), **Limits** (a Start
//! and an End), and **Boolean**. The same parameters as the command's: the Start type and End
//! type together are the other side's angle, symmetric, the angle and "to object".

use egui::Ui;
use serde_json::{Value, json};
use solidcraft_engine::Sel;
use solidcraft_engine::geom::Vec3;

use crate::SolveApp;
use crate::dialogs::{self, Dialog, Kind, OP_LABELS, TO_OBJECT, field, section};
use crate::ui_kit;

const START_TYPES: [&str; 3] = ["Profile Plane", "Value (Other Side)", "Symmetric Value"];
const END_TYPES: [&str; 2] = ["Value", "Until Selected"];

/// The axis as the dialog stands: a point on it and its direction (the specifier's, with the
/// sketch's own axes and the world's resolved against the profile's frame).
pub fn axis_line(app: &SolveApp, d: &Dialog) -> Option<(Vec3, Vec3)> {
    let spec = d.specs.first()?;
    if let Some(Sel::Axis { name }) = d.inputs.iter().find(|i| i.owner == 1).and_then(|i| i.items.first())
        && matches!(name.as_str(), "x" | "y")
        && !spec.reverse
        && let Some(Sel::Profile { sketch, .. }) = d.inputs.first().and_then(|i| i.items.first())
    {
        let plane = crate::canvas::profile_plane(app, *sketch)?;
        return Some((plane.origin, if name == "x" { plane.x } else { plane.y }));
    }
    crate::specifier::line(app, d, 0).ok().flatten()
}

fn point(v: Vec3) -> Value {
    json!([v.x, v.y, v.z])
}

/// Has the axis been given through the specifier: a pick in its inputs, or a way that needs none
/// (a coordinate axis)? Otherwise an axis stored with the feature (an edit) stands.
pub fn specified(d: &Dialog) -> bool {
    d.specs.first().is_some_and(|s| s.count == 0 || d.inputs.iter().any(|i| i.owner == 1 && !i.items.is_empty()))
}

/// The `axis` parameter: a name or a sketch line's id as picked (so the revolve follows it), or
/// the line as a point and a direction when reversed or specified another way.
pub fn axis_param(app: &SolveApp, d: &Dialog) -> Result<Value, String> {
    let spec = d.specs.first().ok_or("no axis input")?;
    let picked = d.inputs.iter().find(|i| i.owner == 1).and_then(|i| i.items.first());
    if matches!(spec.id(), "inferred" | "edge") && !spec.reverse {
        match picked {
            Some(Sel::Axis { name }) => return Ok(json!(name)),
            Some(Sel::SketchCurve { id }) => return Ok(json!(id)),
            _ => {}
        }
    }
    match axis_line(app, d) {
        Some((origin, dir)) => Ok(json!({"origin": point(origin), "dir": point(dir)})),
        None => Err("select an axis".into()),
    }
}

/// The Advanced Revolve rows. `true` when Enter was pressed in a value.
pub fn rows(app: &SolveApp, d: &mut Dialog, ui: &mut Ui) -> bool {
    let mut enter = false;
    if section(ui, "revolve_section", "Section", true) {
        for k in 0..d.inputs.len() {
            if d.inputs.get(k).is_some_and(|i| i.owner == 0 && i.label != TO_OBJECT) {
                dialogs::input_rows_range(d, ui, &app.session, k..k + 1);
            }
        }
    }
    enter |= crate::specifier::show(app, d, ui, 0, "Axis", "revolve_axis");
    if section(ui, "revolve_limits", "Limits", true) {
        enter |= limits(app, d, ui);
    }
    let Kind::Revolve { operation, .. } = &mut d.kind else { return enter };
    if section(ui, "revolve_boolean", "Boolean", true) {
        ui_kit::row(ui, "Operation", |ui| {
            dialogs::combo(ui, "rv_op", &OP_LABELS, operation);
        });
    }
    enter
}

fn limits(app: &SolveApp, d: &mut Dialog, ui: &mut Ui) -> bool {
    let mut enter = false;
    let to_object = dialogs::to_object_on(&d.inputs);
    let mut end = usize::from(to_object);
    let mut start_type = if d.extra.contains_key("angle2") {
        1
    } else if d.extra.get("direction").is_some() {
        2
    } else {
        0
    };
    // Until Selected has no sides: the Start is the profile plane then.
    if !to_object {
        let current = start_type;
        ui_kit::row(ui, "Start", |ui| {
            dialogs::combo(ui, "rv_start_type", &START_TYPES, &mut start_type);
        });
        if start_type != current {
            d.extra.remove("angle2");
            d.extra.remove("direction");
            match start_type {
                1 => {
                    d.extra.insert("angle2".into(), json!("90 deg"));
                }
                2 => {
                    d.extra.insert("direction".into(), json!("symmetric"));
                }
                _ => {}
            }
        }
        if start_type == 1 {
            ui_kit::row(ui, "Angle", |ui| {
                enter |= dialogs::extra_field(ui, &mut d.extra, "angle2", "90 deg");
            });
        }
    }
    ui_kit::row(ui, "End", |ui| {
        dialogs::combo(ui, "rv_end_type", &END_TYPES, &mut end);
    });
    if end == 0 {
        if let Kind::Revolve { angle, .. } = &mut d.kind {
            ui_kit::row(ui, if start_type == 2 { "Angle (total)" } else { "Angle" }, |ui| {
                enter |= field(ui, angle);
            });
        }
    } else {
        d.extra.remove("angle2");
        d.extra.remove("direction");
    }
    dialogs::set_to_object(&mut d.inputs, &mut d.active, end == 1);
    if end == 1
        && let Some(at) = d.inputs.iter().position(|i| i.label == TO_OBJECT)
    {
        dialogs::input_rows_range(d, ui, &app.session, at..at + 1);
    }
    enter
}
