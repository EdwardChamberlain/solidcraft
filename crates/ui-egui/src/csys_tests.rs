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

#[test]
fn the_fixed_coordinate_system_takes_its_origin_and_axes_from_selector_rows() {
    let mut h = advanced();
    assert!(
        crate::dialogs_datum::types(crate::dialogs_datum::Class::Csys).iter().any(|(c, _)| *c == "construct.csys.frame"),
        "it is a Type of the datum"
    );
    h.app.start("construct.csys.frame");
    h.frames(4);
    let d = h.app.dialog.as_ref().expect("a dialog");
    assert_eq!(d.specs.len(), 3, "origin, x axis, y axis");
    for g in ["csys_origin", "csys_x", "csys_y"] {
        assert!(crate::scenario::handle_at(&format!("dialog:group:{g}")).is_some(), "{g}");
    }
    // Nothing picked: the world's origin and axes.
    let c = crate::dialogs::apply_commands(&h.app, d).unwrap();
    assert_eq!(c[0].0, "construct.csys.frame");
    assert_eq!(c[0].1["x_dir"], json!([1.0, 0.0, 0.0]));
    assert_eq!(c[0].1["y_dir"], json!([0.0, 1.0, 0.0]));
    // The Y axis along Z (the x axis picked as Y's own first): turned about X.
    {
        let d = h.app.dialog.as_mut().unwrap();
        let at = d.inputs.iter().position(|i| i.owner == 3).unwrap();
        d.inputs[at].items = vec![Sel::Axis { name: "Z".into() }];
    }
    let c = crate::dialogs::apply_commands(&h.app, h.app.dialog.as_ref().unwrap()).unwrap();
    assert_eq!(c[0].1["y_dir"], json!([0.0, 0.0, 1.0]));
    h.frames(6);
    h.step(&json!({"widget": "OK"})).unwrap();
    h.frames(4);
    let list = solidcraft_engine::view::coordinate_systems(&h.app.session);
    assert_eq!(list.len(), 1, "one coordinate system made");
}

#[test]
fn a_fixed_coordinate_system_reopens_with_its_numbers_and_editing_redefines_it() {
    use solidcraft_engine::doc::FeatureKind;
    use solidcraft_engine::doc::construct::CsysDef;
    let mut h = advanced();
    h.app.run("construct.csys.frame", json!({"origin": [5, 6, 7], "x_dir": [1, 0, 0], "y_dir": [0, 0, 1], "name": "Fixed"})).unwrap();
    let id = h.app.session.doc.find_feature("Fixed").unwrap().id;
    let mut d = crate::dialogs::for_feature(&h.app, id, None).expect("it reopens");
    assert_eq!((d.specs.len(), d.editing.map(|e| e.0)), (3, Some(id)));
    assert_eq!(d.specs[0].values, vec!["5 mm", "6 mm", "7 mm"]);
    assert_eq!(d.specs[2].values, vec!["0", "0", "1"]);
    // Unchanged, it applies as it was.
    let (cmd, p) = crate::dialogs_datum::command(&h.app, &d).unwrap();
    assert_eq!(cmd, "construct.csys.frame");
    assert_eq!((p["origin"].clone(), p["y_dir"].clone()), (json!([5.0, 6.0, 7.0]), json!([0.0, 0.0, 1.0])));
    // Another origin, applied through the edit.
    d.specs[0].values = vec!["10 mm".into(), "0 mm".into(), "0 mm".into()];
    let cmds = crate::dialogs::apply_commands(&h.app, &d).unwrap();
    assert_eq!(cmds[0].0, "timeline.redefine");
    h.app.run(&cmds[0].0, cmds[0].1.clone()).unwrap();
    let Some(FeatureKind::ConstructionCsys { def: CsysDef::Frame { origin, .. } }) = h.app.session.doc.feature(id).map(|f| f.kind.clone()) else {
        panic!("still a fixed coordinate system")
    };
    assert!((origin.x - 10.0).abs() < 1e-9 && origin.y.abs() < 1e-9, "{origin:?}");
    // A dialog that is opened afresh and typed over the same way works too: components can be typed.
    h.app.start("construct.csys.frame");
    h.frames(3);
    let mut d = h.app.dialog.take().unwrap();
    d.specs[1] = crate::specifier::Specifier::at_components(solidcraft_engine::geom::Vec3::new(0.0, 1.0, 0.0));
    d.specs[2] = crate::specifier::Specifier::at_components(solidcraft_engine::geom::Vec3::new(-1.0, 0.0, 0.0));
    let (_, p) = crate::dialogs_datum::command(&h.app, &d).unwrap();
    assert_eq!((p["x_dir"].clone(), p["y_dir"].clone()), (json!([0.0, 1.0, 0.0]), json!([-1.0, 0.0, 0.0])));
}
