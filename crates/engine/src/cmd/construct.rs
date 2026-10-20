//! SOLID ▸ CONSTRUCT: construction axes, points and the planes built from model geometry.
//!
//! References are given as `{face: [x,y,z]}`, `{edge: [x,y,z]}`, `{vertex: [x,y,z]}`,
//! `{point: [x,y,z]}`, `{plane: XY|name|{origin, normal}}`, `{axis: X|Y|Z|name}`,
//! `{construct_point: name}`, `{sketch, point}` or `{sketch, curve}`, or as a tagged
//! `GeoRef` (`{"type": "edge", "at": …}`). Picked faces and edges are stored with their
//! persistent names.

use serde_json::Value;
use solidcraft_doc::construct::{AxisDef, GeoRef, PointDef};
use solidcraft_doc::{FeatureKind, PlaneRef};

use super::CommandSpec;
use super::features::{add_feature, check_expr, plane_param, sketch_id};
use crate::params::{bad, req_expr, str_, vec3};
use crate::{Result, Session};

const REF: &str =
    "{face|edge|vertex|point: [x,y,z]} | {plane: XY|name|{origin, normal}} | {axis: X|Y|Z|name} | {construct_point: name} | {sketch, point|curve}";

pub static COMMANDS: &[CommandSpec] = &[
    CommandSpec::new("datum.plane", "Datum Plane", datum_plane)
        .icon("plane")
        .params("type: offset|at_angle|bisector|three_points|two_edges|tangent|along_path|perpendicular; then the parameters of that type's command (construct.plane.offset, construct.plane.at_angle, construct.plane.midplane, …). One datum plane, whichever way it is defined"),
    CommandSpec::new("datum.axis", "Datum Axis", datum_axis)
        .icon("axis")
        .params("type: cylinder|normal_to_face|two_planes|two_points|edge; then the parameters of that type's command (construct.axis.*). One datum axis, whichever way it is defined"),
    CommandSpec::new("datum.point", "Datum Point", datum_point)
        .icon("point")
        .params("type: vertex|two_edges|three_planes|center|edge_plane|along_path; then the parameters of that type's command (construct.point.*). One datum point, whichever way it is defined"),
    CommandSpec::new("datum.csys", "Datum CSYS", datum_csys)
        .icon("origin")
        .params("type: three_points|on_plane|frame; then the parameters of that type's command (construct.csys.*). One datum coordinate system, whichever way it is defined"),
    CommandSpec::new("construct.csys.three_points", "CSYS From Three Points", csys_three_points)
        .icon("origin")
        .params("origin: point ref; x: point ref (on the +X axis); y: point ref (in the XY plane, on the +Y side); name?"),
    CommandSpec::new("construct.csys.on_plane", "CSYS On Plane", csys_on_plane)
        .icon("origin")
        .params("plane: plane or planar face ref; origin?: point ref (moved onto the plane; default: the plane's own origin); name? Its axes are the plane's"),
    CommandSpec::new("construct.csys.frame", "CSYS From Axes", csys_frame)
        .icon("origin")
        .params("origin: [x,y,z]; x_dir: [x,y,z]; y_dir: [x,y,z] (made perpendicular to x_dir, on the side it points); name?"),
    CommandSpec::new("csys.set_work", "Make Work Coordinate System", set_work_csys)
        .icon("origin")
        .params("csys?: datum CSYS name or id (omit or null: the design's own is the work coordinate system again)"),
    CommandSpec::new("construct.axis.through_cylinder", "Axis Through Cylinder/Cone/Torus", axis_face)
        .at("SOLID", "CONSTRUCT")
        .icon("axis_through_cylinder")
        .params("face: ref (a cylinder, cone or torus face, or a circular edge); name?"),
    CommandSpec::new("construct.axis.normal_to_face", "Axis Perpendicular To Face", axis_normal)
        .at("SOLID", "CONSTRUCT")
        .icon("axis_normal_to_face")
        .params("face: ref; point: ref (the axis goes through it, square to the face); name?"),
    CommandSpec::new("construct.axis.two_planes", "Axis Through Two Planes", axis_planes)
        .at("SOLID", "CONSTRUCT")
        .icon("axis_two_planes")
        .params("a, b: plane or planar face refs; name?"),
    CommandSpec::new("construct.axis.two_points", "Axis Through Two Points", axis_points)
        .at("SOLID", "CONSTRUCT")
        .icon("axis_two_points")
        .params("a, b: point refs (vertices, sketch points, points); name?"),
    CommandSpec::new("construct.axis.edge", "Axis Through Edge", axis_edge)
        .at("SOLID", "CONSTRUCT")
        .icon("axis_edge")
        .params("edge: ref (a straight edge or sketch line); name?"),
    CommandSpec::new("construct.point.vertex", "Point At Vertex", point_at)
        .at("SOLID", "CONSTRUCT")
        .icon("point_vertex")
        .params("point: ref (vertex, sketch point, point); name?"),
    CommandSpec::new("construct.point.two_edges", "Point Through Two Edges", point_edges)
        .at("SOLID", "CONSTRUCT")
        .icon("point_two_edges")
        .params("a, b: edge or line refs (where they meet, or come closest); name?"),
    CommandSpec::new("construct.point.three_planes", "Point Through Three Planes", point_planes)
        .at("SOLID", "CONSTRUCT")
        .icon("point_three_planes")
        .params("a, b, c: plane or planar face refs; name?"),
    CommandSpec::new("construct.point.center", "Point At Center Of Circle/Sphere/Torus", point_center)
        .at("SOLID", "CONSTRUCT")
        .icon("point_center")
        .params("of: ref (circular edge, sketch circle or arc, sphere or torus face); name?"),
    CommandSpec::new("construct.point.edge_plane", "Point At Edge And Plane", point_edge_plane)
        .at("SOLID", "CONSTRUCT")
        .icon("point_edge_plane")
        .params("edge: edge or line ref; plane: plane or planar face ref; name?"),
    CommandSpec::new("construct.point.along_path", "Point Along Path", point_along)
        .at("SOLID", "CONSTRUCT")
        .icon("point_along_path")
        .params("path: edge or sketch curve ref; t: expr, 0…1 along it (default 0.5); name?"),
    CommandSpec::new("construct.point.between", "Point Between Two Points", point_between)
        .at("SOLID", "CONSTRUCT")
        .icon("point_between")
        .params("a, b: point refs (vertices, sketch points, points); t: expr, 0…1 of the way from a to b (default 0.5, the middle); name?"),
    CommandSpec::new("construct.plane.midplane", "Midplane", plane_mid)
        .at("SOLID", "CONSTRUCT")
        .icon("plane_midplane")
        .params("a, b: plane or planar face refs (halfway between; bisecting when they meet at an angle); name?"),
    CommandSpec::new("construct.plane.two_edges", "Plane Through Two Edges", plane_edges)
        .at("SOLID", "CONSTRUCT")
        .icon("plane_two_edges")
        .params("a, b: straight edge or line refs in one plane; name?"),
    CommandSpec::new("construct.plane.three_points", "Plane Through Three Points", plane_points)
        .at("SOLID", "CONSTRUCT")
        .icon("plane_three_points")
        .params("a, b, c: point refs; name?"),
    CommandSpec::new("construct.plane.tangent", "Tangent Plane", plane_tangent)
        .at("SOLID", "CONSTRUCT")
        .icon("plane_tangent")
        .params("face: ref (cylinder, cone, sphere or torus); at?: [x,y,z] where it touches (default: the face pick); name?"),
    CommandSpec::new("construct.plane.along_path", "Plane Along Path", plane_along)
        .at("SOLID", "CONSTRUCT")
        .icon("plane_along_path")
        .params("path: edge or sketch curve ref; t: expr, 0…1 along it (default 0.5); name? The plane is square to the path there"),
    CommandSpec::new("construct.plane.perpendicular", "Perpendicular Plane", plane_perpendicular).at("SOLID", "CONSTRUCT").icon("plane_perpendicular").params(
        "plane: plane or planar face ref; line: straight edge, sketch line or axis ref; name? The plane holds the line and is square to the plane",
    ),
];

/// A reference from its JSON form.
fn geo(s: &Session, v: Option<&Value>, cmd: &str, what: &str) -> Result<GeoRef> {
    let v = v.ok_or_else(|| bad(cmd, format!("`{what}` is required: {REF}")))?;
    let p3 = |k: &str| v.get(k).and_then(vec3);
    let mut g = if v.get("type").is_some() {
        serde_json::from_value::<GeoRef>(v.clone()).map_err(|e| bad(cmd, format!("`{what}`: {e}")))?
    } else if let Some(at) = p3("face") {
        GeoRef::Face { at, name: None }
    } else if let Some(at) = p3("edge") {
        GeoRef::Edge { at, name: None }
    } else if let Some(at) = p3("vertex") {
        GeoRef::Vertex { at }
    } else if let Some(sk) = v.get("sketch") {
        let sketch = sketch_id(s, Some(sk), cmd, what)?;
        match (str_(v, "point"), str_(v, "curve")) {
            (Some(point), _) => GeoRef::SketchPoint { sketch, point: point.to_string() },
            (None, Some(curve)) => GeoRef::SketchCurve { sketch, curve: curve.to_string() },
            _ => return Err(bad(cmd, format!("`{what}`: a sketch reference needs `point` or `curve`"))),
        }
    } else if let Some(p) = p3("point") {
        GeoRef::Point { p }
    } else if v.get("plane").is_some() {
        GeoRef::Plane { plane: plane_param(s, v.get("plane"), cmd)? }
    } else if let Some(a) = str_(v, "axis") {
        if ["x", "y", "z"].contains(&a.to_ascii_lowercase().as_str()) {
            GeoRef::World { axis: a.to_ascii_uppercase() }
        } else {
            match s.doc.find_feature(a).map(|f| &f.kind) {
                Some(FeatureKind::ConstructionAxis { .. }) => GeoRef::Axis { name: a.to_string() },
                _ if csys_part(s, a, &solidcraft_doc::construct::CSYS_AXES) => GeoRef::Axis { name: a.to_string() },
                _ => return Err(bad(cmd, format!("no axis `{a}`"))),
            }
        }
    } else if let Some(n) = str_(v, "construct_point") {
        match s.doc.find_feature(n).map(|f| &f.kind) {
            Some(FeatureKind::ConstructionPoint { .. }) => GeoRef::ConstructPoint { name: n.to_string() },
            _ if csys_part(s, n, &[solidcraft_doc::construct::CSYS_ORIGIN]) => GeoRef::ConstructPoint { name: n.to_string() },
            _ => return Err(bad(cmd, format!("no construction point `{n}`"))),
        }
    } else if v.as_str().is_some() {
        // A bare string: a plane.
        GeoRef::Plane { plane: plane_param(s, Some(v), cmd)? }
    } else {
        return Err(bad(cmd, format!("`{what}` must be {REF}")));
    };
    g.name_picks(&s.model.state());
    Ok(g)
}

fn t_param(s: &Session, p: &Value, cmd: &str) -> Result<solidcraft_doc::Expr> {
    let t = if p.get("t").is_some() { req_expr(cmd, p, "t")? } else { "0.5".into() };
    check_expr(s, &t, solidcraft_doc::expr::Kind::Unitless, cmd, "t")?;
    Ok(t)
}

fn axis(s: &mut Session, p: &Value, def: AxisDef) -> Result<Value> {
    add_feature(s, p, FeatureKind::ConstructionAxis { def })
}

fn point(s: &mut Session, p: &Value, def: PointDef) -> Result<Value> {
    add_feature(s, p, FeatureKind::ConstructionPoint { def })
}

fn plane(s: &mut Session, p: &Value, plane: PlaneRef) -> Result<Value> {
    add_feature(s, p, FeatureKind::ConstructionPlane { plane })
}

fn axis_face(s: &mut Session, p: &Value) -> Result<Value> {
    let face = geo(s, p.get("face"), "construct.axis.through_cylinder", "face")?;
    axis(s, p, AxisDef::ThroughFace { face })
}

fn axis_normal(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "construct.axis.normal_to_face";
    let face = geo(s, p.get("face"), cmd, "face")?;
    let point = geo(s, p.get("point"), cmd, "point")?;
    axis(s, p, AxisDef::NormalToFace { face, point })
}

fn axis_planes(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "construct.axis.two_planes";
    let (a, b) = (geo(s, p.get("a"), cmd, "a")?, geo(s, p.get("b"), cmd, "b")?);
    axis(s, p, AxisDef::TwoPlanes { a, b })
}

fn axis_points(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "construct.axis.two_points";
    let (a, b) = (geo(s, p.get("a"), cmd, "a")?, geo(s, p.get("b"), cmd, "b")?);
    axis(s, p, AxisDef::TwoPoints { a, b })
}

fn axis_edge(s: &mut Session, p: &Value) -> Result<Value> {
    let edge = geo(s, p.get("edge"), "construct.axis.edge", "edge")?;
    axis(s, p, AxisDef::Edge { edge })
}

fn point_at(s: &mut Session, p: &Value) -> Result<Value> {
    let at = geo(s, p.get("point"), "construct.point.vertex", "point")?;
    point(s, p, PointDef::At { point: at })
}

fn point_edges(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "construct.point.two_edges";
    let (a, b) = (geo(s, p.get("a"), cmd, "a")?, geo(s, p.get("b"), cmd, "b")?);
    point(s, p, PointDef::TwoEdges { a, b })
}

fn point_planes(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "construct.point.three_planes";
    let (a, b, c) = (geo(s, p.get("a"), cmd, "a")?, geo(s, p.get("b"), cmd, "b")?, geo(s, p.get("c"), cmd, "c")?);
    point(s, p, PointDef::ThreePlanes { a, b, c })
}

fn point_center(s: &mut Session, p: &Value) -> Result<Value> {
    let of = geo(s, p.get("of"), "construct.point.center", "of")?;
    point(s, p, PointDef::Center { of })
}

fn point_edge_plane(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "construct.point.edge_plane";
    let (edge, pl) = (geo(s, p.get("edge"), cmd, "edge")?, geo(s, p.get("plane"), cmd, "plane")?);
    point(s, p, PointDef::EdgeAndPlane { edge, plane: pl })
}

fn point_along(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "construct.point.along_path";
    let path = geo(s, p.get("path"), cmd, "path")?;
    let t = t_param(s, p, cmd)?;
    point(s, p, PointDef::AlongPath { path, t })
}

fn point_between(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "construct.point.between";
    let (a, b) = (geo(s, p.get("a"), cmd, "a")?, geo(s, p.get("b"), cmd, "b")?);
    let t = t_param(s, p, cmd)?;
    point(s, p, PointDef::Between { a, b, t })
}

fn plane_mid(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "construct.plane.midplane";
    let (a, b) = (geo(s, p.get("a"), cmd, "a")?, geo(s, p.get("b"), cmd, "b")?);
    plane(s, p, PlaneRef::Midplane { a: Box::new(a), b: Box::new(b) })
}

fn plane_edges(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "construct.plane.two_edges";
    let (a, b) = (geo(s, p.get("a"), cmd, "a")?, geo(s, p.get("b"), cmd, "b")?);
    plane(s, p, PlaneRef::TwoEdges { a: Box::new(a), b: Box::new(b) })
}

fn plane_points(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "construct.plane.three_points";
    let (a, b, c) = (geo(s, p.get("a"), cmd, "a")?, geo(s, p.get("b"), cmd, "b")?, geo(s, p.get("c"), cmd, "c")?);
    plane(s, p, PlaneRef::ThreePoints { a: Box::new(a), b: Box::new(b), c: Box::new(c) })
}

fn plane_tangent(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "construct.plane.tangent";
    let face = geo(s, p.get("face"), cmd, "face")?;
    let at = match (p.get("at").and_then(vec3), &face) {
        (Some(a), _) => a,
        (None, GeoRef::Face { at, .. }) => *at,
        _ => return Err(bad(cmd, "`at` is required: where the plane touches the face")),
    };
    plane(s, p, PlaneRef::Tangent { face: Box::new(face), at })
}

fn plane_along(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "construct.plane.along_path";
    let path = geo(s, p.get("path"), cmd, "path")?;
    let t = t_param(s, p, cmd)?;
    plane(s, p, PlaneRef::AlongPath { path: Box::new(path), t })
}

fn plane_perpendicular(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "construct.plane.perpendicular";
    let (base, line) = (geo(s, p.get("plane"), cmd, "plane")?, geo(s, p.get("line"), cmd, "line")?);
    plane(s, p, PlaneRef::Perpendicular { plane: Box::new(base), line: Box::new(line) })
}

/// `datum.plane`, `datum.axis`, `datum.point`: one datum of a kind, the way it is defined being the
/// `type` parameter; the rest goes to that way's own command.
fn datum(s: &mut Session, p: &Value, cmd: &str, kind: &str, types: &[(&str, &str)]) -> Result<Value> {
    let ty = str_(p, "type").ok_or_else(|| bad(cmd, format!("`type` is required: {}", types.iter().map(|t| t.0).collect::<Vec<_>>().join(", "))))?;
    let (_, target) = types
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(ty))
        .ok_or_else(|| bad(cmd, format!("unknown {kind} type `{ty}` ({})", types.iter().map(|t| t.0).collect::<Vec<_>>().join(", "))))?;
    let mut q = p.clone();
    if let Value::Object(m) = &mut q {
        m.remove("type");
    }
    super::find_command(target).ok_or_else(|| bad(cmd, target.to_string()))?.run_nested(s, &q)
}

fn datum_plane(s: &mut Session, p: &Value) -> Result<Value> {
    datum(
        s,
        p,
        "datum.plane",
        "plane",
        &[
            ("offset", "construct.plane.offset"),
            ("at_angle", "construct.plane.at_angle"),
            ("bisector", "construct.plane.midplane"),
            ("midplane", "construct.plane.midplane"),
            ("three_points", "construct.plane.three_points"),
            ("two_edges", "construct.plane.two_edges"),
            ("tangent", "construct.plane.tangent"),
            ("along_path", "construct.plane.along_path"),
            ("perpendicular", "construct.plane.perpendicular"),
        ],
    )
}

fn datum_axis(s: &mut Session, p: &Value) -> Result<Value> {
    datum(
        s,
        p,
        "datum.axis",
        "axis",
        &[
            ("cylinder", "construct.axis.through_cylinder"),
            ("normal_to_face", "construct.axis.normal_to_face"),
            ("two_planes", "construct.axis.two_planes"),
            ("two_points", "construct.axis.two_points"),
            ("edge", "construct.axis.edge"),
        ],
    )
}

fn datum_point(s: &mut Session, p: &Value) -> Result<Value> {
    datum(
        s,
        p,
        "datum.point",
        "point",
        &[
            ("vertex", "construct.point.vertex"),
            ("two_edges", "construct.point.two_edges"),
            ("three_planes", "construct.point.three_planes"),
            ("center", "construct.point.center"),
            ("edge_plane", "construct.point.edge_plane"),
            ("along_path", "construct.point.along_path"),
            ("between", "construct.point.between"),
        ],
    )
}

#[cfg(test)]
mod datum_tests {
    use serde_json::{Value, json};

    use crate::Session;

    fn run(s: &mut Session, id: &str, p: Value) -> Value {
        match s.execute(id, &p) {
            Ok(v) => v,
            Err(e) => panic!("{id} {p}: {e}"),
        }
    }

    #[test]
    fn a_datum_is_made_the_way_its_type_says() {
        let mut s = Session::default();
        run(&mut s, "datum.plane", json!({"type":"offset","base":"XY","offset":"12 mm","name":"Above"}));
        run(&mut s, "datum.plane", json!({"type":"bisector","a":{"plane":"XZ"},"b":{"plane":"YZ"},"name":"Between"}));
        run(&mut s, "datum.axis", json!({"type":"two_points","a":{"point":[0,0,0]},"b":{"point":[0,0,5]},"name":"Up"}));
        run(&mut s, "datum.point", json!({"type":"vertex","point":{"point":[1,2,3]},"name":"Spot"}));
        let names: Vec<&str> = s.doc.features.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["Above", "Between", "Up", "Spot"]);
        let direct = {
            let mut d = Session::default();
            run(&mut d, "construct.plane.offset", json!({"base":"XY","offset":"12 mm","name":"Above"}));
            d.doc.to_json()
        };
        let mut one = Session::default();
        run(&mut one, "datum.plane", json!({"type":"offset","base":"XY","offset":"12 mm","name":"Above"}));
        assert_eq!(one.doc.to_json(), direct, "the same feature as the command of its type");
    }

    #[test]
    fn a_missing_or_unknown_type_is_refused_with_the_choices() {
        let mut s = Session::default();
        for (cmd, p, word) in [
            ("datum.plane", json!({"base":"XY"}), "`type` is required"),
            ("datum.plane", json!({"type":"spiral"}), "unknown plane type"),
            ("datum.axis", json!({"type":"x"}), "unknown axis type"),
            ("datum.point", json!({"type":3}), "`type` is required"),
        ] {
            let e = s.execute(cmd, &p).unwrap_err().to_string();
            assert!(e.contains(word), "{cmd}: {e}");
        }
        assert!(s.doc.features.is_empty());
    }
}

/// Is `name` one of a datum coordinate system's parts (`Csys1:X`), of the kinds in `parts`?
pub(super) fn csys_part(s: &Session, name: &str, parts: &[&str]) -> bool {
    let Some((csys, part)) = name.rsplit_once(':') else { return false };
    parts.contains(&part) && s.doc.find_feature(csys).is_some_and(|f| matches!(f.kind, FeatureKind::ConstructionCsys { .. }))
}

fn csys_three_points(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "construct.csys.three_points";
    let (origin, x, y) = (geo(s, p.get("origin"), cmd, "origin")?, geo(s, p.get("x"), cmd, "x")?, geo(s, p.get("y"), cmd, "y")?);
    let r = add_feature(s, p, FeatureKind::ConstructionCsys { def: solidcraft_doc::construct::CsysDef::ThreePoints { origin, x, y } })?;
    Ok(csys_added(s, r))
}

fn csys_on_plane(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "construct.csys.on_plane";
    let plane = geo(s, p.get("plane"), cmd, "plane")?;
    let origin = match p.get("origin") {
        None | Some(Value::Null) => None,
        Some(v) => Some(geo(s, Some(v), cmd, "origin")?),
    };
    let r = add_feature(s, p, FeatureKind::ConstructionCsys { def: solidcraft_doc::construct::CsysDef::OnPlane { plane, origin } })?;
    Ok(csys_added(s, r))
}

fn csys_frame(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "construct.csys.frame";
    let v = |key: &str| -> Result<solidcraft_geom::Vec3> {
        p.get(key)
            .and_then(vec3)
            .filter(|v| v.x.is_finite() && v.y.is_finite() && v.z.is_finite())
            .ok_or_else(|| bad(cmd, format!("`{key}` must be [x, y, z]")))
    };
    let (origin, x_dir, y_dir) = (v("origin")?, v("x_dir")?, v("y_dir")?);
    // Refused now when it makes no frame, not at the next evaluation.
    let x = x_dir.normalized().ok_or_else(|| bad(cmd, "`x_dir` has no length"))?;
    (y_dir - x * y_dir.dot(x)).normalized().ok_or_else(|| bad(cmd, "`y_dir` lies along `x_dir`"))?;
    let r = add_feature(s, p, FeatureKind::ConstructionCsys { def: solidcraft_doc::construct::CsysDef::Frame { origin, x_dir, y_dir } })?;
    Ok(csys_added(s, r))
}

/// A new coordinate system shows as its triad: its three planes start hidden (they are still its
/// parts, and can be shown).
fn csys_added(s: &mut Session, r: Value) -> Value {
    if let Some(name) = r.get("name").and_then(Value::as_str) {
        for part in solidcraft_doc::construct::CSYS_PLANES {
            let plane = format!("{name}:{part}");
            if !s.visibility.hidden_origin.contains(&plane) {
                s.visibility.hidden_origin.push(plane);
            }
        }
    }
    r
}

fn datum_csys(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "datum.csys";
    let types = [("three_points", "construct.csys.three_points"), ("on_plane", "construct.csys.on_plane"), ("frame", "construct.csys.frame")];
    let names = || types.iter().map(|t| t.0).collect::<Vec<_>>().join(", ");
    let ty = str_(p, "type").ok_or_else(|| bad(cmd, format!("`type` is required: {}", names())))?;
    let (_, target) =
        types.iter().find(|(name, _)| name.eq_ignore_ascii_case(ty)).ok_or_else(|| bad(cmd, format!("unknown csys type `{ty}` ({})", names())))?;
    let mut q = p.clone();
    if let Value::Object(m) = &mut q {
        m.remove("type");
    }
    super::find_command(target).ok_or_else(|| bad(cmd, target.to_string()))?.run_nested(s, &q)
}

fn set_work_csys(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "csys.set_work";
    let id = match p.get("csys") {
        None | Some(Value::Null) => None,
        Some(v) => {
            let key = match v {
                Value::Number(n) => n.to_string(),
                Value::String(x) => x.clone(),
                _ => return Err(bad(cmd, "`csys` must be a coordinate system's name or id")),
            };
            match s.doc.find_feature(&key) {
                Some(f) if matches!(f.kind, FeatureKind::ConstructionCsys { .. }) => Some(f.id),
                _ => return Err(bad(cmd, format!("no datum coordinate system `{key}`"))),
            }
        }
    };
    s.doc_mut().work_csys = id;
    Ok(serde_json::json!({"work_csys": id}))
}

#[cfg(test)]
mod csys_tests {
    use serde_json::{Value, json};
    use solidcraft_geom::Vec3;

    use crate::Session;

    fn run(s: &mut Session, id: &str, p: Value) -> Value {
        match s.execute(id, &p) {
            Ok(v) => v,
            Err(e) => panic!("{id} {p}: {e}"),
        }
    }

    fn near(a: Vec3, b: Vec3) -> bool {
        a.dist(b) < 1e-9
    }

    fn csys(s: &mut Session) -> String {
        let r = run(
            s,
            "datum.csys",
            json!({"type":"three_points","origin":{"point":[10,0,0]},"x":{"point":[20,0,0]},"y":{"point":[10,5,0]},"name":"Work"}),
        );
        r["name"].as_str().unwrap_or("Work").to_string()
    }

    #[test]
    fn a_coordinate_system_offers_its_planes_axes_and_origin_by_name() {
        let mut s = Session::default();
        let name = csys(&mut s);
        let planes = crate::view::construction_planes(&s);
        let plane = |n: &str| planes.iter().find(|p| p.1 == format!("{name}:{n}")).map(|p| p.2).unwrap_or_else(|| panic!("{n}: {planes:?}"));
        assert!(near(plane("XY").origin, Vec3::new(10.0, 0.0, 0.0)) && near(plane("XY").normal(), Vec3::Z));
        assert!(near(plane("XZ").normal(), Vec3::Y) && near(plane("YZ").normal(), Vec3::X));
        let axes = crate::view::construction_axes(&s);
        let axis = |n: &str| axes.iter().find(|a| a.1 == format!("{name}:{n}")).map(|a| (a.2, a.3)).unwrap();
        assert!(near(axis("X").1, Vec3::X) && near(axis("Y").1, Vec3::Y) && near(axis("Z").1, Vec3::Z));
        assert!(crate::view::construction_points(&s).iter().any(|p| p.1 == format!("{name}:O") && near(p.2, Vec3::new(10.0, 0.0, 0.0))));
        assert_eq!(crate::view::coordinate_systems(&s).len(), 1);
    }

    #[test]
    fn things_are_built_on_its_parts_by_name() {
        let mut s = Session::default();
        let name = csys(&mut s);
        // A sketch on its plane, a plane offset from it, an axis between its origin and a point.
        run(&mut s, "sketch.create", json!({"plane": format!("{name}:XY"), "name":"On"}));
        let plane = s.world_state().sketch(2).unwrap().plane;
        assert!(near(plane.origin, Vec3::new(10.0, 0.0, 0.0)), "{plane:?}");
        run(&mut s, "sketch.finish", json!({}));
        run(&mut s, "construct.plane.offset", json!({"base": format!("{name}:XY"), "offset":"4 mm", "name":"Up"}));
        let up = crate::view::construction_planes(&s).into_iter().find(|p| p.1 == "Up").unwrap().2;
        assert!(near(up.origin, Vec3::new(10.0, 0.0, 4.0)), "{up:?}");
        run(&mut s, "construct.axis.edge", json!({"edge": {"axis": format!("{name}:Z")}, "name":"Along"}));
    }

    #[test]
    fn the_other_ways_make_the_same_kind_of_system() {
        let mut s = Session::default();
        // A frame whose y is not perpendicular: it is made so, on the side it points.
        run(&mut s, "construct.csys.frame", json!({"origin":[0,0,0],"x_dir":[1,0,0],"y_dir":[1,2,0],"name":"A"}));
        let a = crate::view::coordinate_systems(&s).into_iter().find(|c| c.1 == "A").unwrap().2;
        assert!(near(a.x, Vec3::X) && near(a.y, Vec3::Y), "{a:?}");
        // On a plane: its axes, with an origin of its own moved onto it.
        run(&mut s, "construct.csys.on_plane", json!({"plane":"XZ","origin":{"point":[3,9,5]},"name":"B"}));
        let b = crate::view::coordinate_systems(&s).into_iter().find(|c| c.1 == "B").unwrap().2;
        assert!(near(b.origin, Vec3::new(3.0, 0.0, 5.0)) && near(b.normal(), Vec3::Y), "{b:?}");
    }

    #[test]
    fn the_work_coordinate_system_is_the_worlds_until_one_is_named() {
        let mut s = Session::default();
        assert!(near(crate::view::work_frame(&s).origin, Vec3::ZERO));
        let name = csys(&mut s);
        run(&mut s, "csys.set_work", json!({"csys": name}));
        assert!(near(crate::view::work_frame(&s).origin, Vec3::new(10.0, 0.0, 0.0)));
        // The design records it; undo takes it back.
        let saved = s.doc.to_json();
        let back = solidcraft_doc::Document::from_json(&saved).unwrap();
        assert!(back.work_csys.is_some(), "{saved}");
        run(&mut s, "csys.set_work", json!({}));
        assert!(near(crate::view::work_frame(&s).origin, Vec3::ZERO), "cleared");
        run(&mut s, "csys.set_work", json!({"csys": name}));
        run(&mut s, "timeline.delete", json!({"features":[name]}));
        assert!(near(crate::view::work_frame(&s).origin, Vec3::ZERO), "a deleted system is no longer the work one");
    }

    #[test]
    fn hostile_input_is_refused_not_panicked_on() {
        let mut s = Session::default();
        for (id, p) in [
            ("construct.csys.frame", json!({"origin":[0,0,0],"x_dir":[0,0,0],"y_dir":[0,1,0]})),
            ("construct.csys.frame", json!({"origin":[0,0,0],"x_dir":[1,0,0],"y_dir":[2,0,0]})),
            ("construct.csys.frame", json!({"origin":[0,0],"x_dir":[1,0,0],"y_dir":[0,1,0]})),
            ("construct.csys.three_points", json!({"origin":{"point":[0,0,0]},"x":{"point":[0,0,0]},"y":{"point":[0,1,0]}})),
            ("construct.csys.three_points", json!({"origin":{"point":[0,0,0]}})),
            ("construct.csys.on_plane", json!({})),
            ("csys.set_work", json!({"csys":"nothing"})),
            ("csys.set_work", json!({"csys":[1]})),
            ("datum.csys", json!({"type":"spiral"})),
            ("datum.csys", json!({})),
        ] {
            let before = s.doc.features.len();
            assert!(s.execute(id, &p).is_err(), "{id} {p}");
            assert_eq!(s.doc.features.len(), before, "{id} left a feature behind");
        }
    }
}
