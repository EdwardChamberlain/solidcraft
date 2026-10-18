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
