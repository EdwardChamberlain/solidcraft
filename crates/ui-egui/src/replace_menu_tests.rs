//! Replace With, in a feature's context menu.

use crate::prefs::ModellingApproach;
use crate::scenario::Harness;
use serde_json::json;

fn rect(h: &mut Harness, name: &str, w: f64) {
    h.app.run("sketch.create", json!({"plane": "XY", "name": name})).unwrap();
    h.app.run("sketch.rectangle.two_point", json!({"p0": [0, 0], "p1": [w, 10]})).unwrap();
    h.app.run("sketch.finish", json!({})).unwrap();
}

#[test]
fn replace_with_in_the_feature_menu_swaps_the_sketch_and_reading_the_choices_changes_nothing() {
    let mut h = Harness::new();
    h.app.preferences.modelling_approach = ModellingApproach::Advanced;
    rect(&mut h, "Narrow", 10.0);
    rect(&mut h, "Wide", 30.0);
    h.app.run("solid.extrude", json!({"sketch": "Narrow", "distance": 5})).unwrap();
    h.frames(6);
    h.step(&json!({"click": {"handle": "timeline:Narrow"}, "button": "right"})).unwrap();
    h.frames(4);
    assert!(crate::scenario::handle_at("replace").is_some(), "the menu offers Replace With");
    // The selection the right-click made is settled; listing the choices each frame is not an edit.
    let revision = h.app.session.revision;
    h.frames(5);
    assert_eq!(h.app.session.revision, revision, "listing the choices is not an edit");
    h.step(&json!({"click": {"handle": "replace"}})).unwrap();
    h.frames(4);
    h.step(&json!({"click": {"handle": "replace:Wide"}})).unwrap();
    h.frames(8);
    let v = h.app.run("inspect.measure", json!({})).unwrap()["total"]["volume_mm3"].as_f64().unwrap();
    assert!((v - 1500.0).abs() < 1e-6, "the extrude now uses Wide: {v}");
}
