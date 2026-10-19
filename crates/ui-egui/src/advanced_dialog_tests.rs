//! Advanced approach: dialogs dock at the left and offer Apply and Reset beside OK and Cancel.
use crate::prefs::ModellingApproach;
use crate::scenario::Harness;
use serde_json::json;

fn sketch(h: &mut Harness) {
    h.app.run("sketch.create", json!({"plane":"XY","name":"Base"})).unwrap();
    h.app.run("sketch.rectangle.two_point", json!({"p0":[0,0],"p1":[10,10]})).unwrap();
    h.app.run("sketch.finish", json!({})).unwrap();
}

fn features(h: &Harness) -> usize {
    h.app.session.doc.features.len()
}

#[test]
fn apply_adds_the_feature_and_keeps_a_fresh_dialog_open() {
    let mut h = Harness::new();
    h.app.preferences.modelling_approach = ModellingApproach::Advanced;
    sketch(&mut h);
    let before = features(&h);
    h.step(&json!({"call":"ui.start","params":{"command":"solid.extrude"}})).unwrap();
    h.frames(6);
    assert_eq!(h.app.dialog.as_ref().and_then(|d| d.origin.as_deref()), Some("solid.extrude"));
    h.step(&json!({"widget":"Apply"})).unwrap();
    h.frames(6);
    assert_eq!(features(&h), before + 1, "Apply ran the command");
    let d = h.app.dialog.as_ref().expect("the dialog stays open after Apply");
    assert!(!d.reopen && d.editing.is_none());
}

#[test]
fn classic_dialogs_have_no_apply_and_ok_closes_them() {
    let mut h = Harness::new();
    sketch(&mut h);
    h.step(&json!({"call":"ui.start","params":{"command":"solid.extrude"}})).unwrap();
    h.frames(6);
    assert!(h.step(&json!({"widget":"Apply"})).is_err());
    h.step(&json!({"widget":"OK"})).unwrap();
    h.frames(6);
    assert!(h.app.dialog.is_none());
}

fn extruded(h: &mut Harness) {
    sketch(h);
    h.app.run("solid.extrude", json!({"sketch":"Base","distance":"10 mm","operation":"new","body_name":"Main"})).unwrap();
    h.frames(2);
}

#[test]
fn datum_dialogs_exist_only_in_advanced_and_list_every_method_per_class() {
    use crate::dialogs::Kind;
    let mut classic = Harness::new();
    classic.app.start("construct.axis.two_points");
    assert!(classic.app.dialog.is_none(), "Classic still has no axis dialog");

    let mut h = Harness::new();
    h.app.preferences.modelling_approach = ModellingApproach::Advanced;
    extruded(&mut h);
    h.app.start("construct.axis.two_points");
    h.frames(3);
    let d = h.app.dialog.as_ref().expect("an axis dialog");
    assert!(matches!(d.kind, Kind::Datum { .. }));
    assert_eq!(d.inputs.len(), 2);

    // Type switches to another axis method; the plane methods have their own list.
    let types = crate::dialogs_datum::types(crate::dialogs_datum::Class::Axis);
    assert_eq!(types.len(), 5);
    assert_eq!(crate::dialogs_datum::types(crate::dialogs_datum::Class::Plane).len(), 8);
    assert_eq!(crate::dialogs_datum::types(crate::dialogs_datum::Class::Point).len(), 6);
}

#[test]
fn a_datum_dialog_runs_its_command_with_references_built_from_the_picks() {
    use solidcraft_engine::Sel;
    use solidcraft_engine::geom::Vec3;
    let mut h = Harness::new();
    h.app.preferences.modelling_approach = ModellingApproach::Advanced;
    extruded(&mut h);
    h.app.start("construct.axis.two_points");
    h.frames(2);
    let mut d = h.app.dialog.take().expect("dialog");
    d.inputs[0].items = vec![Sel::Vertex { body: "Main".into(), point: Vec3::new(0.0, 0.0, 0.0) }];
    d.inputs[1].items = vec![Sel::Vertex { body: "Main".into(), point: Vec3::new(10.0, 0.0, 0.0) }];
    let (cmd, params) = crate::dialogs_datum::command(&h.app, &d).unwrap();
    assert_eq!(cmd, "construct.axis.two_points");
    assert_eq!(params["a"], json!({"vertex":[0.0,0.0,0.0]}));
    h.app.run(cmd, params).unwrap();
    assert!(h.app.session.doc.features.iter().any(|f| matches!(f.kind, solidcraft_engine::doc::FeatureKind::ConstructionAxis { .. })));
    d.inputs[1].items.clear();
    assert!(crate::dialogs_datum::command(&h.app, &d).unwrap_err().contains("second point"));
}

#[test]
fn a_planar_face_is_a_direction_in_advanced_and_not_in_classic() {
    use solidcraft_engine::Sel;
    use solidcraft_engine::geom::Vec3;
    let mut h = Harness::new();
    extruded(&mut h);
    h.app.start("solid.pattern.rectangular");
    h.frames(2);
    let classic_accepts = h.app.dialog.as_ref().unwrap().inputs[1].accept;
    assert_eq!(classic_accepts & crate::selection::PLANAR_FACES, 0);

    h.app.dialog = None;
    h.app.preferences.modelling_approach = ModellingApproach::Advanced;
    h.app.start("solid.pattern.rectangular");
    h.frames(2);
    let mut d = h.app.dialog.take().unwrap();
    assert_ne!(d.inputs[1].accept & crate::selection::PLANAR_FACES, 0);
    let top = (0..40)
        .find_map(|i| crate::dialogs::planar_face(&h.app.session, "Main", i).filter(|(_, n)| n.z > 0.9).map(|_| i))
        .expect("a face looking up");
    d.inputs[0].items = vec![Sel::Body { name: "Main".into() }];
    d.inputs[1].items = vec![Sel::Face { body: "Main".into(), index: top, point: Vec3::new(5.0, 5.0, 10.0) }];
    let (cmd, p) = crate::dialogs::apply_commands(&h.app, &d).unwrap().remove(0);
    assert_eq!(cmd, "solid.pattern.rectangular");
    assert!((p["dir1"][2].as_f64().unwrap() - 1.0).abs() < 1e-9, "{p}");
}

#[test]
fn a_middle_click_moves_to_the_next_pick_then_confirms_in_advanced_only() {
    use solidcraft_engine::Sel;
    let mut h = Harness::new();
    h.app.preferences.modelling_approach = ModellingApproach::Advanced;
    extruded(&mut h);
    let before = features(&h);
    h.app.start("construct.plane.midplane");
    h.frames(3);
    // Nothing picked: the click moves on to the first empty pick and creates nothing.
    h.middle_click();
    h.frames(2);
    assert_eq!(features(&h), before);
    let mut d = h.app.dialog.take().expect("still open");
    d.inputs[0].items = vec![Sel::Plane { name: "XY".into() }];
    d.inputs[1].items = vec![Sel::Plane { name: "XZ".into() }];
    h.app.dialog = Some(d);
    h.frames(3);
    h.middle_click();
    h.frames(4);
    assert!(h.app.dialog.is_none(), "every pick made: the click confirmed");
    assert_eq!(features(&h), before + 1);

    // Classic ignores it.
    let mut c = Harness::new();
    extruded(&mut c);
    c.app.start("construct.plane.midplane");
    c.frames(3);
    let mut d = c.app.dialog.take().unwrap();
    d.inputs[0].items = vec![Sel::Plane { name: "XY".into() }];
    d.inputs[1].items = vec![Sel::Plane { name: "XZ".into() }];
    c.app.dialog = Some(d);
    c.frames(3);
    c.middle_click();
    c.frames(3);
    assert!(c.app.dialog.is_some(), "Classic does not take a middle click as OK");
}

fn vertex(x: f64, y: f64, z: f64) -> solidcraft_engine::Sel {
    solidcraft_engine::Sel::Vertex { body: "Main".into(), point: solidcraft_engine::geom::Vec3::new(x, y, z) }
}

#[test]
fn the_inferred_type_chooses_the_method_from_the_picks_and_says_when_it_cannot() {
    use crate::dialogs_datum::{Class, infer, inferred};
    use solidcraft_engine::Sel;
    let mut h = Harness::new();
    h.app.preferences.modelling_approach = ModellingApproach::Advanced;
    extruded(&mut h);
    let plane = |n: &str| Sel::Plane { name: n.into() };
    let method = |class: Class, picks: Vec<Sel>| {
        let mut d = inferred(class);
        d.inputs[0].items = picks;
        infer(&h.app, &d)
    };
    assert_eq!(method(Class::Plane, vec![plane("XY")]).unwrap().0, "construct.plane.offset");
    assert_eq!(method(Class::Plane, vec![plane("XY"), plane("XZ")]).unwrap().0, "construct.plane.midplane");
    assert_eq!(method(Class::Plane, vec![vertex(0., 0., 0.), vertex(10., 0., 0.), vertex(0., 10., 0.)]).unwrap().0, "construct.plane.three_points");
    assert_eq!(method(Class::Axis, vec![vertex(0., 0., 0.), vertex(10., 0., 0.)]).unwrap().0, "construct.axis.two_points");
    assert_eq!(method(Class::Axis, vec![plane("XY"), plane("XZ")]).unwrap().0, "construct.axis.two_planes");
    assert_eq!(method(Class::Point, vec![vertex(0., 0., 0.)]).unwrap().0, "construct.point.vertex");
    assert_eq!(method(Class::Point, vec![plane("XY"), plane("XZ"), plane("YZ")]).unwrap().0, "construct.point.three_planes");
    // Too few, too many or unrelated picks are explained rather than guessed.
    assert!(method(Class::Plane, vec![]).unwrap_err().contains("select"));
    let err = method(Class::Axis, vec![plane("XY"), plane("XZ"), plane("YZ")]).unwrap_err();
    assert!(err.contains("axis"), "{err}");
    // What it chose runs.
    let (cmd, params) = method(Class::Axis, vec![vertex(0., 0., 0.), vertex(10., 0., 0.)]).unwrap();
    h.app.run(cmd, params).unwrap();
    assert!(h.app.session.doc.features.iter().any(|f| matches!(f.kind, solidcraft_engine::doc::FeatureKind::ConstructionAxis { .. })));
}

#[test]
fn editing_a_datum_made_by_a_shared_method_refills_its_picks_and_keeps_fixed_points() {
    use solidcraft_engine::doc::FeatureKind;
    let mut h = Harness::new();
    h.app.preferences.modelling_approach = ModellingApproach::Advanced;
    extruded(&mut h);
    h.app.run("construct.axis.two_points", json!({"a": {"vertex": [0, 0, 0]}, "b": {"vertex": [10, 0, 0]}, "name": "Rail"})).unwrap();
    let id = h.app.session.doc.find_feature("Rail").unwrap().id;
    let d = crate::dialogs::for_feature(&h.app, id, None).expect("an edit dialog");
    assert!(matches!(d.kind, crate::dialogs::Kind::Datum { .. }));
    assert_eq!(d.editing.map(|e| e.0), Some(id));
    assert!(d.inputs.iter().all(|i| i.items.len() == 1), "{:?}", d.inputs);
    // A fixed point has no pick: it is kept as stored and applied again unchanged.
    h.app.run("construct.axis.two_points", json!({"a": {"point": [1, 2, 3]}, "b": {"vertex": [10, 0, 0]}, "name": "Free"})).unwrap();
    let free = h.app.session.doc.find_feature("Free").unwrap().id;
    let d = crate::dialogs::for_feature(&h.app, free, None).unwrap();
    assert!(d.inputs[0].items.is_empty() && d.extra.contains_key("a"));
    let (cmd, params) = crate::dialogs_datum::command(&h.app, &d).unwrap();
    assert_eq!((cmd, &params["a"]["type"]), ("construct.axis.two_points", &json!("point")));
    assert!(matches!(h.app.session.doc.feature(free).map(|f| &f.kind), Some(FeatureKind::ConstructionAxis { .. })));
    // Classic has no dialog for it.
    h.app.preferences.modelling_approach = ModellingApproach::Classic;
    assert!(crate::dialogs::for_feature(&h.app, id, None).is_none());
}

#[test]
fn editing_a_datum_through_its_dialog_redefines_it_in_place() {
    use solidcraft_engine::doc::FeatureKind;
    use solidcraft_engine::doc::construct::{AxisDef, GeoRef};
    let mut h = Harness::new();
    h.app.preferences.modelling_approach = ModellingApproach::Advanced;
    extruded(&mut h);
    h.app.run("construct.axis.two_points", json!({"a": {"vertex": [0, 0, 0]}, "b": {"vertex": [10, 0, 0]}, "name": "Rail"})).unwrap();
    let id = h.app.session.doc.find_feature("Rail").unwrap().id;
    let count = h.app.session.doc.features.len();
    h.step(&json!({"call":"ui.editFeature","params":{"feature":"Rail"}})).unwrap();
    h.frames(4);
    let d = h.app.dialog.as_mut().expect("the edit dialog is open");
    assert_eq!(d.editing.map(|e| e.0), Some(id));
    d.inputs[1].items = vec![vertex(10.0, 10.0, 0.0)];
    h.frames(2);
    h.step(&json!({"widget":"OK"})).unwrap();
    h.frames(4);
    assert!(h.app.dialog.is_none());
    assert_eq!(h.app.session.doc.features.len(), count, "redefined in place, not added");
    let FeatureKind::ConstructionAxis { def: AxisDef::TwoPoints { b, .. } } = &h.app.session.doc.feature(id).unwrap().kind else {
        panic!("an axis through two points")
    };
    assert!(matches!(b, GeoRef::Vertex { at } if (at.y - 10.0).abs() < 1e-6), "{b:?}");
}

#[test]
fn the_preview_group_switches_the_live_result_off_and_on() {
    let mut h = Harness::new();
    h.app.preferences.modelling_approach = ModellingApproach::Advanced;
    sketch(&mut h);
    h.step(&json!({"call":"ui.start","params":{"command":"solid.extrude"}})).unwrap();
    h.frames(30);
    assert!(h.app.preview.active, "the result is previewed by default");
    h.step(&json!({"widget":"Show result"})).unwrap();
    h.frames(6);
    assert!(h.app.preview.hidden && !h.app.preview.active, "Show result off clears the preview");
    let before = features(&h);
    h.step(&json!({"widget":"OK"})).unwrap();
    h.frames(6);
    assert_eq!(features(&h), before + 1, "OK still makes the feature");
    h.app.preview.hidden = false;
}

#[test]
fn classic_dialogs_have_no_preview_group() {
    let mut h = Harness::new();
    sketch(&mut h);
    h.step(&json!({"call":"ui.start","params":{"command":"solid.extrude"}})).unwrap();
    h.frames(6);
    assert!(h.step(&json!({"widget":"Show result"})).is_err());
}

fn to_object_accept(approach: ModellingApproach) -> crate::selection::Accept {
    use crate::selection::{FACES, SelInput, VERTICES};
    let mut h = Harness::new();
    h.app.preferences.modelling_approach = approach;
    sketch(&mut h);
    h.step(&json!({"call":"ui.start","params":{"command":"solid.extrude"}})).unwrap();
    h.frames(4);
    let d = h.app.dialog.as_mut().unwrap();
    d.inputs.push(SelInput::new("To Object", FACES | VERTICES, false));
    h.frames(4);
    h.app.dialog.as_ref().unwrap().inputs.iter().find(|i| i.label == "To Object").unwrap().accept
}

#[test]
fn specify_point_takes_an_edge_in_advanced_only() {
    use crate::selection::EDGES;
    assert_ne!(to_object_accept(ModellingApproach::Advanced) & EDGES, 0);
    assert_eq!(to_object_accept(ModellingApproach::Classic) & EDGES, 0);
}

fn groups_of(approach: ModellingApproach, command: &str, names: &[&str]) -> Vec<bool> {
    let mut h = Harness::new();
    h.app.preferences.modelling_approach = approach;
    extruded(&mut h);
    h.app.start(command);
    h.frames(6);
    names.iter().map(|n| crate::scenario::handle_at(&format!("dialog:group:{n}")).is_some()).collect()
}

#[test]
fn hole_and_blend_dialogs_have_groups_in_advanced_only() {
    let hole = ["hole_form", "hole_limits"];
    assert_eq!(groups_of(ModellingApproach::Advanced, "solid.hole", &hole), [true, true]);
    assert_eq!(groups_of(ModellingApproach::Classic, "solid.hole", &hole), [false, false]);
    let blend = ["blend_shape", "blend_settings"];
    assert_eq!(groups_of(ModellingApproach::Advanced, "solid.fillet", &blend), [true, true]);
    assert_eq!(groups_of(ModellingApproach::Classic, "solid.fillet", &blend), [false, false]);
}
