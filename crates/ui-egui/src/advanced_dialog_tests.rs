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
