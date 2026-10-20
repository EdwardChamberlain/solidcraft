//! The datum coordinate system in the Advanced UI: its dialog, its row in the tree, and the
//! defaults it changes (a new sketch's origin and x axis).
use crate::prefs::ModellingApproach;
use crate::scenario::Harness;
use serde_json::json;
use solidcraft_engine::Sel;
use solidcraft_engine::geom::Vec3;

fn advanced() -> Harness {
    let mut h = Harness::new();
    h.app.preferences.modelling_approach = ModellingApproach::Advanced;
    h
}

/// A coordinate system at (10, 20, 0) whose x axis runs along +Y.
fn turned(h: &mut Harness) -> String {
    let r = h
        .app
        .run("construct.csys.three_points", json!({"origin":{"point":[10,20,0]},"x":{"point":[10,30,0]},"y":{"point":[0,20,0]},"name":"Turned"}))
        .unwrap();
    h.frames(4);
    r["name"].as_str().unwrap().to_string()
}

fn near(a: Vec3, b: Vec3) -> bool {
    a.dist(b) < 1e-9
}

#[test]
fn the_datum_csys_button_opens_one_dialog_that_infers_three_points_or_a_plane() {
    let mut h = advanced();
    h.app.run("solid.box", json!({"length":10,"width":10,"height":10})).unwrap();
    h.frames(6);
    h.step(&json!({"widget":"Datum CSYS"})).unwrap();
    h.frames(4);
    let d = h.app.dialog.as_mut().expect("the Datum CSYS dialog");
    assert!(matches!(d.kind, crate::dialogs::Kind::DatumInferred { class: crate::dialogs_datum::Class::Csys, .. }));
    let v = |x: f64, y: f64| Sel::Vertex { body: "Body1".into(), point: Vec3::new(x, y, 0.0) };
    d.inputs[0].items = vec![v(0.0, 0.0), v(10.0, 0.0), v(0.0, 10.0)];
    let cmds = crate::dialogs::apply_commands(&h.app, h.app.dialog.as_ref().unwrap()).unwrap();
    assert_eq!(cmds[0].0, "construct.csys.three_points", "origin, x point, y point in pick order");
    // OK makes it.
    h.frames(6);
    h.step(&json!({"widget":"OK"})).unwrap();
    h.frames(4);
    assert_eq!(solidcraft_engine::view::coordinate_systems(&h.app.session).len(), 1);
    // One plane picked: a coordinate system on it.
    h.app.start("datum.csys");
    h.frames(4);
    h.app.dialog.as_mut().unwrap().inputs[0].items = vec![Sel::Plane { name: "XZ".into() }];
    let cmds = crate::dialogs::apply_commands(&h.app, h.app.dialog.as_ref().unwrap()).unwrap();
    assert_eq!(cmds[0].0, "construct.csys.on_plane");
}

#[test]
fn a_new_coordinate_system_shows_as_its_triad_and_the_tree_lists_its_parts() {
    let mut h = advanced();
    let name = turned(&mut h);
    let hidden = &h.app.session.visibility.hidden_origin;
    assert!(hidden.contains(&format!("{name}:XY")) && !hidden.contains(&format!("{name}:X")), "planes start hidden, the triad shown");
    assert!(crate::scenario::handle_at(&format!("timeline:{name}")).is_some(), "it is a row of the tree");
    // Its eye hides and shows the whole system.
    let id = h.app.session.doc.find_feature(&name).unwrap().id;
    let eye = crate::shell::part_nav_tree::eye_of(&h.app, id).expect("an eye");
    assert!(crate::shell::part_nav_tree::shown(&h.app, &eye));
    crate::shell::part_nav_tree::toggle(&mut h.app, id);
    assert!(!crate::shell::part_nav_tree::shown(&h.app, &eye));
    assert!(h.app.session.visibility.hidden_origin.contains(&format!("{name}:O")));
    crate::shell::part_nav_tree::toggle(&mut h.app, id);
    assert!(crate::shell::part_nav_tree::shown(&h.app, &eye));
    assert!(h.app.session.visibility.hidden_origin.contains(&format!("{name}:XY")), "showing it leaves its planes as they were");
}

#[test]
fn the_work_coordinate_system_is_the_default_of_a_new_sketch() {
    let mut h = advanced();
    let name = turned(&mut h);
    h.app.start("sketch.create");
    h.frames(4);
    let d = h.app.dialog.as_mut().unwrap();
    d.inputs[0].items = vec![Sel::Plane { name: "XY".into() }];
    h.frames(4);
    h.step(&json!({"widget":"OK"})).unwrap();
    h.frames(4);
    let id = h.app.session.active_sketch.unwrap();
    let plane = h.app.session.world_state().sketch(id).unwrap().plane;
    assert!(near(plane.origin, Vec3::ZERO) && near(plane.x, Vec3::X), "the world's own by default: {plane:?}");
    h.app.run("sketch.finish", json!({})).unwrap();
    // Make it the work coordinate system: the origin and x axis follow it.
    h.app.run("csys.set_work", json!({"csys": name})).unwrap();
    h.app.start("sketch.create");
    h.frames(4);
    h.app.dialog.as_mut().unwrap().inputs[0].items = vec![Sel::Plane { name: "XY".into() }];
    h.frames(4);
    h.step(&json!({"widget":"OK"})).unwrap();
    h.frames(4);
    let id = h.app.session.active_sketch.unwrap();
    let plane = h.app.session.world_state().sketch(id).unwrap().plane;
    assert!(near(plane.origin, Vec3::new(10.0, 20.0, 0.0)) && near(plane.x, Vec3::Y), "{plane:?}");
}

#[test]
fn the_tree_menu_makes_and_releases_the_work_system_and_its_parts_can_be_picked() {
    let mut h = advanced();
    let name = turned(&mut h);
    let id = h.app.session.doc.find_feature(&name).unwrap().id;
    h.app.run("csys.set_work", json!({"csys": id})).unwrap();
    assert_eq!(h.app.session.doc.work_csys, Some(id));
    h.app.run("csys.set_work", json!({})).unwrap();
    assert_eq!(h.app.session.doc.work_csys, None);
    // A sketch on one of its planes, picked from the tree into the dialog.
    h.app.start("sketch.create");
    h.frames(4);
    h.app.ui.show_browser = true;
    h.step(&json!({"click":{"handle":format!("timeline-sketch-fold:{name}")}})).unwrap();
    h.frames(4);
    h.step(&json!({"click":{"handle":"row:XY"}})).unwrap();
    h.frames(3);
    let picked = h.app.dialog.as_ref().and_then(|d| d.inputs.first()).map(|i| i.items.clone()).unwrap_or_default();
    assert_eq!(picked, vec![Sel::Plane { name: format!("{name}:XY") }], "the part row picked its plane into the dialog");
}
