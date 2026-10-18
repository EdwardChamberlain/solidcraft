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
