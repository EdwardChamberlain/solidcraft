//! Datum dialogs for the Advanced approach: one dialog per kind of datum (plane, axis, point)
//! with a Type selector, so every construction method the engine has can be started, and changed
//! after the fact, from one place.
//!
//! The four plane methods that already had dialogs keep their own [`Kind`]s; the rest share
//! [`Kind::Datum`], driven by the [`METHODS`] table (the inputs are the command's own reference
//! parameters, so nothing here knows more than the `construct.*` commands do).

use egui::Ui;
use serde_json::{Value, json};
use solidcraft_engine::Sel;

use crate::SolveApp;
use crate::dialogs::{Dialog, Kind};
use crate::selection::{self, AXES, Accept, CURVES, EDGES, FACES, PLANAR_FACES, PLANES, POINTS, SelInput};

/// What a datum dialog makes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    Plane,
    Axis,
    Point,
}

/// One construction method of the shared dialog: the command, and its inputs as
/// (parameter, label, what it accepts).
pub struct Method {
    pub class: Class,
    pub cmd: &'static str,
    pub label: &'static str,
    pub inputs: &'static [(&'static str, &'static str, Accept)],
    /// A typed value: (parameter, label, default).
    pub value: Option<(&'static str, &'static str, &'static str)>,
}

const PLANE_REF: Accept = PLANES | PLANAR_FACES;
const POINT_REF: Accept = selection::VERTICES | POINTS;
const LINE_REF: Accept = EDGES | AXES;

pub static METHODS: &[Method] = &[
    Method {
        class: Class::Axis,
        cmd: "construct.axis.through_cylinder",
        label: "Cylinder, Cone or Torus",
        inputs: &[("face", "Face", FACES | EDGES)],
        value: None,
    },
    Method {
        class: Class::Axis,
        cmd: "construct.axis.normal_to_face",
        label: "Normal to Face",
        inputs: &[("face", "Face", FACES | PLANAR_FACES), ("point", "Point", POINT_REF)],
        value: None,
    },
    Method {
        class: Class::Axis,
        cmd: "construct.axis.two_planes",
        label: "Intersection of Planes",
        inputs: &[("a", "First plane", PLANE_REF), ("b", "Second plane", PLANE_REF)],
        value: None,
    },
    Method {
        class: Class::Axis,
        cmd: "construct.axis.two_points",
        label: "Two Points",
        inputs: &[("a", "First point", POINT_REF), ("b", "Second point", POINT_REF)],
        value: None,
    },
    Method { class: Class::Axis, cmd: "construct.axis.edge", label: "Edge or Line", inputs: &[("edge", "Edge or line", LINE_REF)], value: None },
    Method { class: Class::Point, cmd: "construct.point.vertex", label: "At Vertex or Point", inputs: &[("point", "Point", POINT_REF)], value: None },
    Method {
        class: Class::Point,
        cmd: "construct.point.two_edges",
        label: "Two Edges",
        inputs: &[("a", "First edge", LINE_REF), ("b", "Second edge", LINE_REF)],
        value: None,
    },
    Method {
        class: Class::Point,
        cmd: "construct.point.three_planes",
        label: "Three Planes",
        inputs: &[("a", "First plane", PLANE_REF), ("b", "Second plane", PLANE_REF), ("c", "Third plane", PLANE_REF)],
        value: None,
    },
    Method {
        class: Class::Point,
        cmd: "construct.point.center",
        label: "Circle, Sphere or Torus Centre",
        inputs: &[("of", "Circle or sphere", EDGES | FACES | CURVES)],
        value: None,
    },
    Method {
        class: Class::Point,
        cmd: "construct.point.edge_plane",
        label: "Edge and Plane",
        inputs: &[("edge", "Edge or line", LINE_REF), ("plane", "Plane", PLANE_REF)],
        value: None,
    },
    Method {
        class: Class::Point,
        cmd: "construct.point.along_path",
        label: "Along Path",
        inputs: &[("path", "Path", CURVES | EDGES)],
        value: Some(("t", "Position (0 to 1)", "0.5")),
    },
    Method {
        class: Class::Plane,
        cmd: "construct.plane.two_edges",
        label: "Two Edges",
        inputs: &[("a", "First edge", LINE_REF), ("b", "Second edge", LINE_REF)],
        value: None,
    },
    Method {
        class: Class::Plane,
        cmd: "construct.plane.three_points",
        label: "Three Points",
        inputs: &[("a", "First point", POINT_REF), ("b", "Second point", POINT_REF), ("c", "Third point", POINT_REF)],
        value: None,
    },
    Method { class: Class::Plane, cmd: "construct.plane.tangent", label: "Tangent to Face", inputs: &[("face", "Curved face", FACES)], value: None },
    Method {
        class: Class::Plane,
        cmd: "construct.plane.perpendicular",
        label: "Perpendicular to Plane",
        inputs: &[("plane", "Plane", PLANE_REF), ("line", "Line or edge", LINE_REF)],
        value: None,
    },
];

/// The Type choices of each class, in the order the dialog lists them: (command, label). The
/// plane methods that have their own dialog kinds are here too, so Type switches between all.
pub fn types(class: Class) -> Vec<(&'static str, &'static str)> {
    let mut v: Vec<(&'static str, &'static str)> = match class {
        Class::Plane => vec![
            ("construct.plane.offset", "Offset from Plane"),
            ("construct.plane.at_angle", "At Angle to Plane"),
            ("construct.plane.midplane", "Midplane"),
            ("construct.plane.along_path", "Along Path"),
        ],
        _ => Vec::new(),
    };
    v.extend(METHODS.iter().filter(|m| m.class == class).map(|m| (m.cmd, m.label)));
    v
}

/// The class of datum a dialog kind makes, if it is one.
pub fn class_of(kind: &Kind) -> Option<Class> {
    match kind {
        Kind::OffsetPlane { .. } | Kind::AnglePlane { .. } | Kind::Midplane | Kind::PathPlane { .. } => Some(Class::Plane),
        Kind::Datum { method, .. } => METHODS.get(*method).map(|m| m.class),
        _ => None,
    }
}

/// The command a dialog kind runs.
fn command_of(kind: &Kind) -> Option<&'static str> {
    Some(match kind {
        Kind::OffsetPlane { .. } => "construct.plane.offset",
        Kind::AnglePlane { .. } => "construct.plane.at_angle",
        Kind::Midplane => "construct.plane.midplane",
        Kind::PathPlane { .. } => "construct.plane.along_path",
        Kind::Datum { method, .. } => METHODS.get(*method)?.cmd,
        _ => return None,
    })
}

/// The shared dialog for one of [`METHODS`].
pub fn dialog(cmd: &str) -> Option<Dialog> {
    let (method, m) = METHODS.iter().enumerate().find(|(_, m)| m.cmd == cmd)?;
    let inputs = m.inputs.iter().map(|(_, label, accept)| SelInput::new(label, *accept, false)).collect();
    let value = m.value.map(|(_, _, default)| default.to_string()).unwrap_or_default();
    Some(Dialog::new(Kind::Datum { method, value }, inputs))
}

/// The Type row: switching keeps the picks the new type can still use.
pub fn type_row(app: &SolveApp, d: &mut Dialog, ui: &mut Ui) {
    let (Some(class), Some(current)) = (class_of(&d.kind), command_of(&d.kind)) else { return };
    let types = types(class);
    let old = types.iter().position(|(cmd, _)| *cmd == current).unwrap_or(0);
    let mut picked = old;
    let labels: Vec<&str> = types.iter().map(|(_, label)| *label).collect();
    crate::dialogs::row_label(ui, "Type");
    crate::dialogs::combo(ui, "datum_type", &labels, &mut picked);
    ui.end_row();
    if picked == old {
        return;
    }
    if let Some((cmd, _)) = types.get(picked)
        && let Some(mut next) = Dialog::for_command(app, cmd)
    {
        for (input, was) in next.inputs.iter_mut().zip(d.inputs.iter()) {
            input.items = was.items.iter().filter(|s| crate::dialogs::fits(input.accept, s)).cloned().collect();
        }
        next.origin = d.origin.clone();
        *d = next;
    }
}

/// The typed value row of a shared-dialog method, if it has one.
pub fn value_row(ui: &mut Ui, method: usize, value: &mut String) -> bool {
    let Some((_, label, _)) = METHODS.get(method).and_then(|m| m.value) else { return false };
    crate::dialogs::row_label(ui, label);
    let enter = crate::dialogs::field(ui, value);
    ui.end_row();
    enter
}

/// A pick as the reference the `construct.*` commands take.
fn reference(app: &SolveApp, sel: &Sel) -> Result<Value, String> {
    let at = |p: solidcraft_engine::geom::Vec3| json!([p.x, p.y, p.z]);
    Ok(match sel {
        Sel::Plane { name } => json!({"plane": name}),
        Sel::Axis { name } => json!({"axis": name}),
        Sel::Face { point, .. } => json!({"face": at(*point)}),
        Sel::Edge { point, .. } => json!({"edge": at(*point)}),
        Sel::Vertex { point, .. } => json!({"vertex": at(*point)}),
        Sel::SketchCurve { id } => {
            let sketch = crate::dialogs::curves_sketch(app, std::slice::from_ref(id), None).ok_or("the curve must be in a sketch")?;
            json!({"sketch": sketch, "curve": id})
        }
        Sel::SketchPoint { id } => match id.split_once(':') {
            Some((sketch, point)) => json!({"sketch": sketch.parse::<u64>().map_err(|_| "pick the point again")?, "point": point}),
            None => return Err("pick a vertex or a point of a sketch".into()),
        },
        _ => return Err("that cannot be used here".into()),
    })
}

/// The command and parameters a shared dialog applies.
pub fn command(app: &SolveApp, d: &Dialog) -> Result<(&'static str, Value), String> {
    let Kind::Datum { method, value } = &d.kind else { return Err("not a datum dialog".into()) };
    let m = METHODS.get(*method).ok_or("unknown datum type")?;
    let mut p = serde_json::Map::new();
    for ((param, label, _), input) in m.inputs.iter().zip(&d.inputs) {
        let sel = input.items.first().ok_or_else(|| format!("select {} first", label.to_lowercase()))?;
        p.insert((*param).into(), reference(app, sel)?);
    }
    if let Some((param, _, _)) = m.value {
        p.insert(param.into(), json!(value));
    }
    Ok((m.cmd, Value::Object(p)))
}
