//! `solid.pattern.feature` and `solid.pattern.geometry`: the layout is a parameter.

use serde_json::{Value, json};

use crate::Session;

fn run(s: &mut Session, id: &str, p: Value) -> Value {
    match s.execute(id, &p) {
        Ok(v) => v,
        Err(e) => panic!("{id} {p}: {e}"),
    }
}

fn part() -> Session {
    let mut s = Session::default();
    run(&mut s, "sketch.create", json!({"plane":"XY","name":"Base"}));
    run(&mut s, "sketch.rectangle.two_point", json!({"p0":[0,0],"p1":[10,10]}));
    run(&mut s, "sketch.finish", json!({}));
    run(&mut s, "solid.extrude", json!({"sketch":"Base","distance":"5 mm","operation":"new","body_name":"Block"}));
    s
}

fn volume(s: &Session) -> f64 {
    s.world_state().bodies.iter().filter_map(|b| b.mesh().mass_properties()).map(|m| m.volume).sum()
}

#[test]
fn a_feature_pattern_is_the_same_as_the_command_of_its_layout() {
    let mut direct = part();
    run(&mut direct, "solid.pattern.rectangular", json!({"features":["Extrude1"],"dir1":[1,0,0],"count1":3,"spacing1":"20 mm"}));
    let mut layout = part();
    run(&mut layout, "solid.pattern.feature", json!({"layout":"linear","features":["Extrude1"],"dir1":[1,0,0],"count1":3,"spacing1":"20 mm"}));
    assert!((volume(&direct) - volume(&layout)).abs() < 1e-6, "{} {}", volume(&direct), volume(&layout));
    assert_eq!(direct.doc.features.len(), layout.doc.features.len());
    // The layout defaults to linear.
    let mut dflt = part();
    run(&mut dflt, "solid.pattern.feature", json!({"features":["Extrude1"],"dir1":[1,0,0],"count1":3,"spacing1":"20 mm"}));
    assert!((volume(&dflt) - volume(&direct)).abs() < 1e-6);
}

#[test]
fn a_geometry_pattern_copies_bodies_in_a_circle() {
    let mut s = part();
    run(&mut s, "solid.pattern.geometry", json!({"layout":"circular","bodies":["Block"],"axis":"Z","count":3}));
    assert_eq!(s.doc.features.len(), 3, "one pattern feature was made");
    assert!(s.doc.features.iter().any(|f| f.kind.type_name().contains("Pattern")));
    let names: Vec<String> = s.world_state().bodies.iter().map(|b| b.name.clone()).collect();
    assert!(names.len() > 1 || s.world_state().bodies.first().is_some_and(|b| b.body.size() > 15.0), "the copies are there: {names:?}");
}

#[test]
fn features_and_geometry_are_kept_apart_and_bad_layouts_are_refused() {
    let mut s = part();
    let err = |s: &mut Session, id: &str, p: Value| s.execute(id, &p).unwrap_err().to_string();
    assert!(
        err(&mut s, "solid.pattern.geometry", json!({"features":["Extrude1"],"dir1":[1,0,0],"count1":2,"spacing1":10}))
            .contains("solid.pattern.feature")
    );
    assert!(
        err(&mut s, "solid.pattern.feature", json!({"bodies":["Block"],"dir1":[1,0,0],"count1":2,"spacing1":10})).contains("solid.pattern.geometry")
    );
    assert!(err(&mut s, "solid.pattern.feature", json!({"layout":"spiral","features":["Extrude1"]})).contains("unknown layout"));
    assert!(err(&mut s, "solid.pattern.feature", json!({"layout":"linear"})).contains("give features"));
    assert_eq!(s.doc.features.len(), 2, "nothing was made by the refused calls");
}
