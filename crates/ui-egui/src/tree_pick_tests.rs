//! Picking from the feature tree into a dialog's inputs.
use crate::prefs::ModellingApproach;
use crate::scenario::Harness;
use serde_json::json;
use solidcraft_engine::Sel;

fn part(approach: ModellingApproach) -> Harness {
    let mut h = Harness::new();
    h.app.preferences.modelling_approach = approach;
    h.app.run("sketch.create", json!({"plane":"XY","name":"Base"})).unwrap();
    h.app.run("sketch.rectangle.two_point", json!({"p0":[0,0],"p1":[20,10]})).unwrap();
    h.app.run("sketch.finish", json!({})).unwrap();
    h.app.run("solid.extrude", json!({"sketch":"Base","distance":"10 mm","operation":"new","body_name":"Main"})).unwrap();
    h.app.run("construct.plane.offset", json!({"base":"XY","offset":"25 mm","name":"Datum Top"})).unwrap();
    h.app.run("select.clear", json!({})).unwrap();
    h.frames(6);
    h
}

fn id_of(h: &Harness, name: &str) -> u64 {
    h.app.session.doc.find_feature(name).map(|f| f.id).unwrap()
}

fn click_row(h: &mut Harness, name: &str) {
    h.step(&json!({"click":{"handle":format!("timeline:{name}")}})).unwrap();
    h.frames(3);
}

fn items(h: &Harness, input: usize) -> Vec<Sel> {
    h.app.dialog.as_ref().unwrap().inputs[input].items.clone()
}

#[test]
fn a_pattern_takes_the_feature_clicked_in_the_tree() {
    let mut h = part(ModellingApproach::Advanced);
    h.app.start("solid.pattern.rectangular");
    h.frames(4);
    let d = h.app.dialog.as_mut().unwrap();
    // The Features object type: the input takes features.
    d.inputs[0].accept = crate::selection::FEATURES;
    d.inputs[0].items.clear();
    d.active = 0;
    let extrude = id_of(&h, "Extrude1");
    click_row(&mut h, "Extrude1");
    assert_eq!(items(&h, 0), vec![Sel::Feature { id: extrude }], "the click picked the feature");
    assert!(h.app.session.selection.is_empty(), "and did not also select it");
}

#[test]
fn a_sketch_in_the_tree_gives_its_profiles_and_a_datum_gives_its_plane() {
    let mut h = part(ModellingApproach::Advanced);
    h.app.start("solid.extrude");
    h.frames(4);
    let d = h.app.dialog.as_mut().unwrap();
    d.inputs[0].items.clear();
    d.active = 0;
    let base = id_of(&h, "Base");
    click_row(&mut h, "Base");
    assert_eq!(items(&h, 0), vec![Sel::Profile { sketch: base, index: 0 }]);

    // A datum plane into a plane input (an offset plane's base).
    h.app.dialog = None;
    h.app.start("construct.plane.offset");
    h.frames(4);
    h.app.dialog.as_mut().unwrap().active = 0;
    click_row(&mut h, "Datum Top");
    assert_eq!(items(&h, 0), vec![Sel::Plane { name: "Datum Top".into() }]);
}

#[test]
fn a_feature_gives_the_bodies_it_made_to_a_body_input() {
    let mut h = part(ModellingApproach::Advanced);
    h.app.start("solid.mirror");
    h.frames(4);
    let d = h.app.dialog.as_mut().unwrap();
    d.inputs[0].items.clear();
    d.active = 0;
    click_row(&mut h, "Extrude1");
    assert_eq!(items(&h, 0), vec![Sel::Body { name: "Main".into() }]);
}

#[test]
fn a_row_the_input_cannot_use_is_selected_as_usual() {
    let mut h = part(ModellingApproach::Advanced);
    h.app.start("construct.plane.offset");
    h.frames(4);
    h.app.dialog.as_mut().unwrap().active = 0;
    click_row(&mut h, "Base");
    assert!(items(&h, 0).is_empty(), "a sketch is not a plane");
    assert!(h.app.session.selection.iter().any(|s| matches!(s, Sel::Feature { .. })), "the click selected it");
}

#[test]
fn without_the_wider_mapping_only_features_come_from_the_tree() {
    let h = part(ModellingApproach::Classic);
    let base = id_of(&h, "Base");
    assert!(crate::tree_pick::sels_for(&h.app, crate::selection::PROFILES, base, false).is_empty());
    assert_eq!(crate::tree_pick::sels_for(&h.app, crate::selection::FEATURES, base, false), vec![Sel::Feature { id: base }]);
    assert!(!crate::tree_pick::sels_for(&h.app, crate::selection::PROFILES, base, true).is_empty());
}
