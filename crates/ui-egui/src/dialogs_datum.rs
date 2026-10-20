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
use crate::selection::{self, AXES, Accept, BODIES, CURVES, EDGES, FACES, PLANAR_FACES, PLANES, POINTS, SelInput};
use solidcraft_engine::doc::construct::{AxisDef, GeoRef, PointDef};
use solidcraft_engine::doc::{FeatureKind, PlaneRef};

/// What a datum dialog makes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    Plane,
    Axis,
    Point,
    Csys,
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
        class: Class::Csys,
        cmd: "construct.csys.three_points",
        label: "Origin, X Point, Y Point",
        inputs: &[("origin", "Origin", POINT_REF), ("x", "X point", POINT_REF), ("y", "Y point", POINT_REF)],
        value: None,
    },
    Method { class: Class::Csys, cmd: "construct.csys.on_plane", label: "On Plane or Face", inputs: &[("plane", "Plane", PLANE_REF)], value: None },
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
        Kind::DatumInferred { class, .. } => Some(*class),
        _ => None,
    }
}

/// The heading of a datum dialog in the Advanced approach (its Type says which method).
pub fn class_title(kind: &Kind) -> Option<&'static str> {
    Some(match class_of(kind)? {
        Class::Plane => "DATUM PLANE",
        Class::Axis => "DATUM AXIS",
        Class::Point => "DATUM POINT",
        Class::Csys => "DATUM CSYS",
    })
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

/// What an inferred datum's single pick accepts: anything a datum method takes.
const INFERRED_ACCEPT: Accept = PLANES | PLANAR_FACES | FACES | EDGES | AXES | CURVES | selection::VERTICES | POINTS;

/// The Inferred dialog: pick what defines the datum and the method follows.
pub fn inferred(class: Class) -> Dialog {
    let value = if class == Class::Plane { "10 mm" } else { "0.5" };
    let mut d = Dialog::new(Kind::DatumInferred { class, value: value.into() }, vec![SelInput::new("Geometry", INFERRED_ACCEPT, true)]);
    d.inputs[0].accept &= !BODIES;
    d
}

/// The Type row (Inferred first, then every method of the class): switching keeps the picks the
/// new type can still use.
pub fn type_row(app: &SolveApp, d: &mut Dialog, ui: &mut Ui) {
    let Some(class) = class_of(&d.kind) else { return };
    let current = command_of(&d.kind);
    let types = types(class);
    let old = current.and_then(|c| types.iter().position(|(cmd, _)| *cmd == c)).map_or(0, |i| i + 1);
    let mut picked = old;
    let mut labels: Vec<&str> = vec!["Inferred"];
    labels.extend(types.iter().map(|(_, label)| *label));
    crate::dialogs::row_label(ui, "Type");
    crate::dialogs::combo(ui, "datum_type", &labels, &mut picked);
    ui.end_row();
    if picked == old {
        return;
    }
    let next = if picked == 0 { Some(inferred(class)) } else { types.get(picked - 1).and_then(|(cmd, _)| Dialog::for_command(app, cmd)) };
    let Some(mut next) = next else { return };
    if picked == 0 {
        // Every pick so far goes into the one input.
        next.inputs[0].items = d.inputs.iter().flat_map(|i| i.items.iter().cloned()).collect();
    } else if d.inputs.len() == 1 && matches!(d.kind, Kind::DatumInferred { .. }) {
        // The inferred picks are dealt out to the inputs of the new type, in order, as they fit.
        let mut picks = d.inputs[0].items.clone().into_iter();
        for input in next.inputs.iter_mut() {
            if let Some(pick) = picks.next().filter(|s| crate::dialogs::fits(input.accept, s)) {
                input.items = vec![pick];
            }
        }
    } else {
        for (input, was) in next.inputs.iter_mut().zip(d.inputs.iter()) {
            input.items = was.items.iter().filter(|s| crate::dialogs::fits(input.accept, s)).cloned().collect();
        }
    }
    next.origin = d.origin.clone();
    *d = next;
}

/// What a pick is, for choosing a method.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Item {
    Plane,
    CurvedFace,
    Line,
    Point,
}

fn classify(app: &SolveApp, sel: &Sel) -> Option<Item> {
    Some(match sel {
        Sel::Plane { .. } => Item::Plane,
        Sel::Face { body, index, .. } => {
            if crate::dialogs::planar_face(&app.session, body, *index).is_some() {
                Item::Plane
            } else {
                Item::CurvedFace
            }
        }
        Sel::Edge { .. } | Sel::Axis { .. } | Sel::SketchCurve { .. } => Item::Line,
        Sel::Vertex { .. } | Sel::SketchPoint { .. } => Item::Point,
        _ => return None,
    })
}

/// The method the picks make and its parameters: what the Inferred type applies.
pub fn infer(app: &SolveApp, d: &Dialog) -> Result<(&'static str, Value), String> {
    let Kind::DatumInferred { class, value } = &d.kind else { return Err("not a datum dialog".into()) };
    let picks = d.inputs.first().map(|i| i.items.as_slice()).unwrap_or_default();
    if picks.is_empty() {
        return Err("select what defines the datum first".into());
    }
    let mut kinds = Vec::new();
    for sel in picks {
        kinds.push((sel, classify(app, sel).ok_or("that cannot define a datum")?));
    }
    let of = |item: Item| -> Vec<&Sel> { kinds.iter().filter(|(_, k)| *k == item).map(|(s, _)| *s).collect() };
    let (planes, curved, lines, points) = (of(Item::Plane), of(Item::CurvedFace), of(Item::Line), of(Item::Point));
    let r = |sel: &Sel| reference(app, sel);
    let refs = |items: &[&Sel], keys: &[&str]| -> Result<Value, String> {
        let mut p = serde_json::Map::new();
        for (key, sel) in keys.iter().zip(items) {
            p.insert((*key).into(), r(sel)?);
        }
        Ok(Value::Object(p))
    };
    let count = (planes.len(), curved.len(), lines.len(), points.len());
    let found = match (class, count) {
        (Class::Plane, (1, 0, 0, 0)) => {
            ("construct.plane.offset", json!({"base": crate::dialogs::plane_arg(app, planes.first().copied())?, "offset": value}))
        }
        (Class::Plane, (2, 0, 0, 0)) => ("construct.plane.midplane", refs(&planes, &["a", "b"])?),
        (Class::Plane, (0, 0, 0, 3)) => ("construct.plane.three_points", refs(&points, &["a", "b", "c"])?),
        (Class::Plane, (0, 0, 2, 0)) => ("construct.plane.two_edges", refs(&lines, &["a", "b"])?),
        (Class::Plane, (0, 1, 0, 0)) => ("construct.plane.tangent", refs(&curved, &["face"])?),
        (Class::Plane, (1, 0, 1, 0)) => {
            let mut p = refs(&planes, &["plane"])?;
            p["line"] = r(lines[0])?;
            ("construct.plane.perpendicular", p)
        }
        (Class::Plane, (0, 0, 1, 0)) => {
            let mut p = refs(&lines, &["path"])?;
            p["t"] = json!(value);
            ("construct.plane.along_path", p)
        }
        (Class::Csys, (1, 0, 0, 0)) => ("construct.csys.on_plane", refs(&planes, &["plane"])?),
        (Class::Csys, (0, 0, 0, 3)) => ("construct.csys.three_points", refs(&points, &["origin", "x", "y"])?),
        (Class::Axis, (0, 1, 0, 0)) => ("construct.axis.through_cylinder", refs(&curved, &["face"])?),
        (Class::Axis, (0, 0, 1, 0)) => ("construct.axis.edge", refs(&lines, &["edge"])?),
        (Class::Axis, (0, 0, 0, 2)) => ("construct.axis.two_points", refs(&points, &["a", "b"])?),
        (Class::Axis, (2, 0, 0, 0)) => ("construct.axis.two_planes", refs(&planes, &["a", "b"])?),
        (Class::Axis, (1, 0, 0, 1)) => {
            let mut p = refs(&planes, &["face"])?;
            p["point"] = r(points[0])?;
            ("construct.axis.normal_to_face", p)
        }
        (Class::Point, (0, 0, 0, 1)) => ("construct.point.vertex", refs(&points, &["point"])?),
        (Class::Point, (0, 0, 2, 0)) => ("construct.point.two_edges", refs(&lines, &["a", "b"])?),
        (Class::Point, (3, 0, 0, 0)) => ("construct.point.three_planes", refs(&planes, &["a", "b", "c"])?),
        (Class::Point, (0, 1, 0, 0)) => ("construct.point.center", refs(&curved, &["of"])?),
        (Class::Point, (1, 0, 1, 0)) => {
            let mut p = refs(&lines, &["edge"])?;
            p["plane"] = r(planes[0])?;
            ("construct.point.edge_plane", p)
        }
        (Class::Point, (0, 0, 1, 0)) => {
            let mut p = refs(&lines, &["path"])?;
            p["t"] = json!(value);
            ("construct.point.along_path", p)
        }
        _ => return Err(format!("these picks do not define a {} yet; see Type for the methods", class_name(*class))),
    };
    Ok(found)
}

fn class_name(class: Class) -> &'static str {
    match class {
        Class::Plane => "plane",
        Class::Axis => "axis",
        Class::Point => "point",
        Class::Csys => "coordinate system",
    }
}

/// The inferred dialog's typed value: a distance for one plane, a position along one path.
pub fn inferred_value_row(ui: &mut Ui, class: Class, picked: usize, value: &mut String) -> bool {
    let label = match (class, picked) {
        (Class::Plane, 1) => "Distance",
        (Class::Plane | Class::Point, _) => "Position (0 to 1)",
        _ => return false,
    };
    crate::dialogs::row_label(ui, label);
    let enter = crate::dialogs::field(ui, value);
    ui.end_row();
    enter
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
        match (input.items.first(), d.extra.get(*param)) {
            (Some(sel), _) => p.insert((*param).into(), reference(app, sel)?),
            // A reference the dialog was filled with that has no pick of its own (a fixed point,
            // a construction point): kept as it was until another is picked.
            (None, Some(kept)) => p.insert((*param).into(), kept.clone()),
            (None, None) => return Err(format!("select {} first", label.to_lowercase())),
        };
    }
    if let Some((param, _, _)) = m.value {
        p.insert(param.into(), json!(value));
    }
    Ok((m.cmd, Value::Object(p)))
}

/// A pick standing for a stored reference, when the model still has it.
fn sel_of(app: &SolveApp, g: &GeoRef) -> Option<Sel> {
    let s = &app.session;
    match g {
        GeoRef::Plane { plane } => crate::dialogs::plane_sel(s, plane),
        GeoRef::Face { at, .. } => crate::dialogs::face_sel(s, *at),
        GeoRef::Edge { at, .. } => crate::dialogs::edge_sel(s, *at),
        GeoRef::Vertex { at } => crate::dialogs::vertex_sel(s, *at),
        GeoRef::World { axis } | GeoRef::Axis { name: axis } => Some(Sel::Axis { name: axis.clone() }),
        GeoRef::SketchCurve { curve, .. } => Some(Sel::SketchCurve { id: curve.clone() }),
        GeoRef::SketchPoint { sketch, point } => Some(Sel::SketchPoint { id: format!("{sketch}:{point}") }),
        GeoRef::Point { .. } | GeoRef::Origin | GeoRef::ConstructPoint { .. } => None,
    }
}

/// The dialog that edits a datum feature made by one of [`METHODS`], filled from it.
pub fn refill(app: &SolveApp, kind: &FeatureKind) -> Option<Dialog> {
    let (cmd, refs, value): (&str, Vec<&GeoRef>, Option<String>) = match kind {
        FeatureKind::ConstructionAxis { def } => match def {
            AxisDef::ThroughFace { face } => ("construct.axis.through_cylinder", vec![face], None),
            AxisDef::NormalToFace { face, point } => ("construct.axis.normal_to_face", vec![face, point], None),
            AxisDef::TwoPlanes { a, b } => ("construct.axis.two_planes", vec![a, b], None),
            AxisDef::TwoPoints { a, b } => ("construct.axis.two_points", vec![a, b], None),
            AxisDef::Edge { edge } => ("construct.axis.edge", vec![edge], None),
        },
        FeatureKind::ConstructionPoint { def } => match def {
            PointDef::At { point } => ("construct.point.vertex", vec![point], None),
            PointDef::TwoEdges { a, b } => ("construct.point.two_edges", vec![a, b], None),
            PointDef::ThreePlanes { a, b, c } => ("construct.point.three_planes", vec![a, b, c], None),
            PointDef::Center { of } => ("construct.point.center", vec![of], None),
            PointDef::EdgeAndPlane { edge, plane } => ("construct.point.edge_plane", vec![edge, plane], None),
            PointDef::AlongPath { path, t } => ("construct.point.along_path", vec![path], Some(t.to_string())),
        },
        FeatureKind::ConstructionCsys { def } => match def {
            solidcraft_engine::doc::construct::CsysDef::ThreePoints { origin, x, y } => ("construct.csys.three_points", vec![origin, x, y], None),
            solidcraft_engine::doc::construct::CsysDef::OnPlane { plane, .. } => ("construct.csys.on_plane", vec![plane], None),
            solidcraft_engine::doc::construct::CsysDef::Frame { .. } => return None,
        },
        FeatureKind::ConstructionPlane { plane } => match plane {
            PlaneRef::TwoEdges { a, b } => ("construct.plane.two_edges", vec![a, b], None),
            PlaneRef::ThreePoints { a, b, c } => ("construct.plane.three_points", vec![a, b, c], None),
            PlaneRef::Tangent { face, .. } => ("construct.plane.tangent", vec![face], None),
            PlaneRef::Perpendicular { plane, line } => ("construct.plane.perpendicular", vec![plane, line], None),
            _ => return None,
        },
        _ => return None,
    };
    let mut d = dialog(cmd)?;
    let method = METHODS.iter().find(|m| m.cmd == cmd)?;
    for (((param, _, _), input), g) in method.inputs.iter().zip(d.inputs.iter_mut()).zip(refs) {
        match sel_of(app, g) {
            Some(sel) => input.items = vec![sel],
            None => {
                d.extra.insert((*param).into(), serde_json::to_value(g).ok()?);
            }
        }
    }
    if let (Some(v), Kind::Datum { value, .. }) = (value, &mut d.kind) {
        *value = v;
    }
    Some(d)
}
