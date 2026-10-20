//! Specifiers in the Create Sketch dialog (Advanced): the plane, the reference direction and the
//! origin, each with a Type of its own.
use crate::prefs::ModellingApproach;
use crate::scenario::Harness;
use crate::specifier;
use serde_json::json;
use solidcraft_engine::Sel;
use solidcraft_engine::geom::Vec3;

fn advanced() -> Harness {
    let mut h = Harness::new();
    h.app.preferences.modelling_approach = ModellingApproach::Advanced;
    h
}

fn open(h: &mut Harness) {
    h.app.start("sketch.create");
    h.frames(4);
}

fn plane(name: &str) -> Sel {
    Sel::Plane { name: name.into() }
}

fn pick(h: &mut Harness, spec: usize, sel: Sel) {
    let d = h.app.dialog.as_mut().unwrap();
    let at = d.inputs.iter().position(|i| i.owner == spec + 1).expect("the specifier has an input");
    d.inputs[at].items = vec![sel];
}

fn choose(h: &mut Harness, spec: usize, id: &str) {
    let mut d = h.app.dialog.take().unwrap();
    assert!(specifier::choose(&h.app, &mut d, spec, id), "{id}");
    h.app.dialog = Some(d);
}

fn ok(h: &mut Harness) {
    h.frames(6);
    h.step(&json!({"widget":"OK"})).unwrap();
    h.frames(4);
}

fn sketch_plane(h: &Harness) -> solidcraft_engine::geom::Plane {
    let id = h.app.session.active_sketch.unwrap_or_else(|| panic!("a sketch was started: {:?}", h.app.dialog.as_ref().map(|d| d.error.clone())));
    h.app.session.world_state().sketch(id).unwrap().plane
}

fn near(a: Vec3, b: Vec3) -> bool {
    a.dist(b) < 1e-9
}

#[test]
fn advanced_create_sketch_names_the_plane_the_direction_and_the_origin() {
    let mut h = advanced();
    open(&mut h);
    let d = h.app.dialog.as_ref().unwrap();
    assert_eq!(d.specs.len(), 3, "plane, reference direction, origin");
    assert!(h.widget("OK").is_some(), "the dialog waits for OK");
    assert!(crate::scenario::handle_at("dialog:group:sketch_plane").is_some());
    assert!(crate::scenario::handle_at("dialog:group:sketch_direction").is_some());
    assert!(crate::scenario::handle_at("dialog:group:sketch_origin").is_some());
}

#[test]
fn picking_the_plane_waits_for_ok_so_the_direction_and_origin_can_still_be_chosen() {
    let mut h = advanced();
    open(&mut h);
    let d = h.app.dialog.as_ref().unwrap();
    assert_eq!(d.specs[1].id(), "axis", "the direction starts at the x axis");
    assert_eq!(d.specs[2].id(), "origin", "the origin starts at the workpiece origin");
    assert_eq!(d.inputs.len(), 1, "only the plane has anything to pick");
    pick(&mut h, 0, plane("XY"));
    h.frames(8);
    assert!(h.app.session.active_sketch.is_none() && h.app.dialog.is_some(), "the pick alone does not start the sketch");
    ok(&mut h);
    assert!(h.app.session.active_sketch.is_some() && h.app.dialog.is_none());
    let p = sketch_plane(&h);
    assert!(near(p.origin, Vec3::ZERO) && near(p.x, Vec3::X), "{p:?}");
}

#[test]
fn the_defaults_suit_every_plane_the_x_axis_square_to_one_keeps_its_own() {
    let mut h = advanced();
    open(&mut h);
    pick(&mut h, 0, plane("YZ"));
    ok(&mut h);
    let p = sketch_plane(&h);
    assert!(near(p.x, solidcraft_engine::geom::Plane::YZ.x) && near(p.origin, Vec3::ZERO), "{p:?}");
}

#[test]
fn a_direction_chosen_after_the_face_is_picked_is_used() {
    let mut h = advanced();
    open(&mut h);
    pick(&mut h, 0, plane("XY"));
    h.frames(6);
    choose(&mut h, 1, "axis");
    h.app.dialog.as_mut().unwrap().specs[1].values = vec!["Y".into()];
    ok(&mut h);
    assert!(near(sketch_plane(&h).x, Vec3::Y), "{:?}", sketch_plane(&h));
}

#[test]
fn the_default_origin_of_a_sketch_on_a_face_is_the_workpiece_origin_projected_onto_it() {
    let mut h = advanced();
    h.app.run("sketch.create", json!({"plane":"XY","name":"Base"})).unwrap();
    h.app.run("sketch.rectangle.two_point", json!({"p0":[5,5],"p1":[25,15]})).unwrap();
    h.app.run("sketch.finish", json!({})).unwrap();
    h.app.run("solid.extrude", json!({"sketch":"Base","distance":"10 mm","operation":"new","body_name":"Main"})).unwrap();
    h.frames(2);
    open(&mut h);
    let face = crate::dialogs::face_sel(&h.app.session, Vec3::new(15.0, 10.0, 10.0)).expect("the top face");
    pick(&mut h, 0, face);
    ok(&mut h);
    let p = sketch_plane(&h);
    assert!(near(p.origin, Vec3::new(0.0, 0.0, 10.0)) && near(p.x, Vec3::X), "{p:?}");
}

#[test]
fn classic_create_sketch_still_starts_on_the_pick() {
    let mut h = Harness::new();
    open(&mut h);
    assert!(h.app.dialog.as_ref().unwrap().specs.is_empty());
    let mut d = h.app.dialog.take().unwrap();
    d.inputs[0].items = vec![plane("XY")];
    h.app.dialog = Some(d);
    h.frames(4);
    assert!(h.app.session.active_sketch.is_some());
}

#[test]
fn an_offset_plane_is_made_and_sketched_on() {
    let mut h = advanced();
    open(&mut h);
    choose(&mut h, 0, "construct.plane.offset");
    pick(&mut h, 0, plane("XY"));
    {
        let d = h.app.dialog.as_mut().unwrap();
        let inner = d.specs[0].inputs_value_mut().expect("the offset has a distance");
        *inner = "20 mm".into();
    }
    ok(&mut h);
    let made = h.app.session.doc.features.iter().filter(|f| matches!(f.kind, solidcraft_engine::doc::FeatureKind::ConstructionPlane { .. })).count();
    assert_eq!(made, 1, "the plane is a feature of its own");
    let p = sketch_plane(&h);
    assert!(near(p.origin, Vec3::new(0.0, 0.0, 20.0)), "{p:?}");
}

#[test]
fn inferred_makes_a_bisector_from_two_planes_and_the_plane_itself_from_one() {
    let mut h = advanced();
    open(&mut h);
    let d = h.app.dialog.as_mut().unwrap();
    d.inputs[0].items = vec![plane("XZ"), plane("YZ")];
    let out = specifier::plane(&h.app, h.app.dialog.as_ref().unwrap(), 0).unwrap();
    assert_eq!(out.prefix.first().map(|c| c.0.as_str()), Some("construct.plane.midplane"));
    let d = h.app.dialog.as_mut().unwrap();
    d.inputs[0].items = vec![plane("XY")];
    let out = specifier::plane(&h.app, h.app.dialog.as_ref().unwrap(), 0).unwrap();
    assert!(out.prefix.is_empty() && out.plane == json!("XY"), "one plane is the plane: {out:?}");
}

#[test]
fn the_reference_direction_sets_the_sketch_x_axis_and_reverse_inverts_it() {
    for (reverse, want) in [(false, Vec3::Y), (true, -Vec3::Y)] {
        let mut h = advanced();
        open(&mut h);
        pick(&mut h, 0, plane("XY"));
        choose(&mut h, 1, "axis");
        {
            let d = h.app.dialog.as_mut().unwrap();
            d.specs[1].values = vec!["Y".into()];
            d.specs[1].reverse = reverse;
        }
        ok(&mut h);
        let p = sketch_plane(&h);
        assert!(near(p.x, want) && near(p.normal(), Vec3::Z), "reverse {reverse}: {p:?}");
    }
}

#[test]
fn a_direction_from_an_edge_or_a_face_normal_works_too() {
    let mut h = advanced();
    h.app.run("sketch.create", json!({"plane":"XY","name":"Base"})).unwrap();
    h.app.run("sketch.rectangle.two_point", json!({"p0":[0,0],"p1":[20,10]})).unwrap();
    h.app.run("sketch.finish", json!({})).unwrap();
    h.app.run("solid.extrude", json!({"sketch":"Base","distance":"10 mm","operation":"new","body_name":"Main"})).unwrap();
    h.frames(2);
    open(&mut h);
    pick(&mut h, 0, plane("XZ"));
    choose(&mut h, 1, "normal");
    pick(&mut h, 1, plane("YZ"));
    let d = h.app.dialog.as_ref().unwrap();
    let v = specifier::vector(&h.app, d, 1).unwrap().unwrap();
    assert!(near(v, Vec3::X) || near(v, -Vec3::X), "the normal of YZ: {v:?}");
}

#[test]
fn the_origin_can_be_the_workpiece_origin() {
    let mut h = advanced();
    open(&mut h);
    choose(&mut h, 0, "construct.plane.offset");
    pick(&mut h, 0, plane("XY"));
    *h.app.dialog.as_mut().unwrap().specs[0].inputs_value_mut().unwrap() = "20 mm".into();
    choose(&mut h, 2, "origin");
    ok(&mut h);
    let p = sketch_plane(&h);
    assert!(near(p.origin, Vec3::new(0.0, 0.0, 20.0)), "the workpiece origin, onto the plane: {p:?}");
}

#[test]
fn the_origin_can_be_a_typed_point() {
    let mut h = advanced();
    open(&mut h);
    pick(&mut h, 0, plane("XY"));
    choose(&mut h, 2, "coords");
    h.app.dialog.as_mut().unwrap().specs[2].values = vec!["5 mm".into(), "6 mm".into(), "7 mm".into()];
    ok(&mut h);
    assert!(near(sketch_plane(&h).origin, Vec3::new(5.0, 6.0, 0.0)), "{:?}", sketch_plane(&h));
}

#[test]
fn the_origin_can_be_a_picked_point() {
    let mut h = advanced();
    open(&mut h);
    pick(&mut h, 0, plane("XY"));
    choose(&mut h, 2, "existing");
    pick(&mut h, 2, Sel::Vertex { body: "Main".into(), point: Vec3::new(3.0, 4.0, 9.0) });
    ok(&mut h);
    assert!(near(sketch_plane(&h).origin, Vec3::new(3.0, 4.0, 0.0)), "{:?}", sketch_plane(&h));
}

#[test]
fn changing_the_type_keeps_the_picks_the_new_type_can_use() {
    let mut h = advanced();
    open(&mut h);
    pick(&mut h, 0, plane("XY"));
    choose(&mut h, 0, "plane");
    let d = h.app.dialog.as_ref().unwrap();
    assert_eq!(d.inputs[0].items, vec![plane("XY")], "the plane is still picked");
    assert_eq!(d.specs[0].count, 1);
    assert_eq!(d.inputs.len(), 1, "only the plane has an input: the direction and origin defaults need no pick");
}

#[test]
fn nothing_picked_refuses_ok_with_a_reason_instead_of_a_sketch() {
    let mut h = advanced();
    open(&mut h);
    let d = h.app.dialog.as_ref().unwrap();
    let err = crate::dialogs::sketch_commands(&h.app, d).unwrap_err();
    assert!(err.contains("select a plane"), "{err}");
}

#[test]
fn reattach_sketch_changes_the_plane_of_the_sketch_being_edited() {
    let mut h = advanced();
    h.app.run("sketch.create", json!({"plane":"XY","name":"Base"})).unwrap();
    h.app.run("sketch.rectangle.two_point", json!({"p0":[0,0],"p1":[10,5]})).unwrap();
    h.frames(4);
    let curves = h.app.session.doc.sketch(h.app.session.active_sketch.unwrap()).unwrap().curves.len();
    h.app.start("sketch.reattach");
    h.frames(4);
    let d = h.app.dialog.as_ref().expect("the Reattach dialog opens");
    assert_eq!(d.specs.len(), 3, "plane, reference direction and origin, like Create Sketch");
    assert!(d.specs[1].id() == "inferred" && d.specs[2].id() == "inferred", "nothing is changed unless asked for");
    pick(&mut h, 0, plane("XZ"));
    h.frames(6);
    assert!(h.app.dialog.is_some(), "a pick alone does not reattach: OK does");
    ok(&mut h);
    assert!(h.app.dialog.is_none());
    let p = sketch_plane(&h);
    assert!(near(p.normal(), solidcraft_engine::geom::Plane::XZ.normal()), "{p:?}");
    let id = h.app.session.active_sketch.expect("still editing the sketch");
    assert_eq!(h.app.session.doc.sketch(id).unwrap().curves.len(), curves, "the geometry stays");
}

#[test]
fn reattach_sketch_takes_a_datum_plane_and_nothing_without_a_sketch() {
    let mut h = advanced();
    h.app.start("sketch.reattach");
    h.frames(3);
    assert!(h.app.dialog.is_none(), "no sketch, no dialog");
    h.app.run("construct.plane.offset", json!({"base":"XY","offset":"15 mm","name":"Datum A"})).unwrap();
    h.app.run("sketch.create", json!({"plane":"XY","name":"Base"})).unwrap();
    h.frames(3);
    h.app.start("sketch.reattach");
    h.frames(4);
    pick(&mut h, 0, plane("Datum A"));
    ok(&mut h);
    assert!(near(sketch_plane(&h).origin, Vec3::new(0.0, 0.0, 15.0)), "{:?}", sketch_plane(&h));
}
