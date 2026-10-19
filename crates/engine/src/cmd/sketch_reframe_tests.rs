//! `sketch.create` `reframe`: a sketch's own origin and x axis on any plane, the sketch staying
//! attached to the plane it was made on.

use serde_json::{Value, json};
use solidcraft_geom::Vec3;

use crate::Session;

fn run(s: &mut Session, id: &str, p: Value) -> Value {
    match s.execute(id, &p) {
        Ok(v) => v,
        Err(e) => panic!("{id} {p}: {e}"),
    }
}

fn plane_of(s: &Session) -> solidcraft_geom::Plane {
    let id = s.active_sketch.unwrap();
    s.world_state().sketch(id).unwrap().plane
}

fn near(a: Vec3, b: Vec3) -> bool {
    a.dist(b) < 1e-9
}

#[test]
fn reframe_sets_the_origin_and_the_x_axis_in_the_plane() {
    let mut s = Session::default();
    run(&mut s, "sketch.create", json!({"plane":"XY","reframe":{"origin":[5.0,7.0,3.0],"x_dir":[0.0,1.0,0.0]}}));
    let p = plane_of(&s);
    assert!(near(p.origin, Vec3::new(5.0, 7.0, 0.0)), "the origin is moved onto the plane: {:?}", p.origin);
    assert!(near(p.x, Vec3::Y), "x follows the direction: {:?}", p.x);
    assert!(near(p.normal(), Vec3::Z), "the plane's side is kept: {:?}", p.normal());
    assert!(near(p.y, -Vec3::X), "y completes a right-handed frame: {:?}", p.y);
}

#[test]
fn a_direction_off_the_plane_is_taken_along_it_and_one_square_to_it_is_refused() {
    let mut s = Session::default();
    run(&mut s, "sketch.create", json!({"plane":"XY","reframe":{"x_dir":[1.0,0.0,5.0]}}));
    assert!(near(plane_of(&s).x, Vec3::X));
    let mut s = Session::default();
    let err = s.execute("sketch.create", &json!({"plane":"XY","reframe":{"x_dir":[0.0,0.0,1.0]}})).unwrap_err().to_string();
    assert!(err.contains("along the plane"), "{err}");
    assert!(s.doc.features.is_empty(), "nothing was made");
}

#[test]
fn hostile_reframes_are_refused_not_panicked_on() {
    for bad in [json!({}), json!({"origin":[1,2]}), json!({"origin":"x"}), json!({"x_dir":[0,0,0]}), json!({"x_dir":[1,"a",0]})] {
        let mut s = Session::default();
        assert!(s.execute("sketch.create", &json!({"plane":"XY","reframe":bad})).is_err(), "{bad}");
    }
}

#[test]
fn a_reframed_sketch_stays_attached_and_keeps_its_frame_through_save_and_reload() {
    let mut s = Session::default();
    run(&mut s, "sketch.create", json!({"plane":"XY","name":"Base"}));
    run(&mut s, "sketch.rectangle.two_point", json!({"p0":[0,0],"p1":[10,10]}));
    run(&mut s, "sketch.finish", json!({}));
    run(&mut s, "solid.extrude", json!({"sketch":"Base","distance":"10 mm","operation":"new","body_name":"Main"}));
    run(&mut s, "sketch.create", json!({"plane":{"face":[5.0,5.0,10.0]},"reframe":{"origin":[2.0,2.0,10.0],"x_dir":[0.0,1.0,0.0]},"name":"Top"}));
    let p = plane_of(&s);
    assert!(near(p.origin, Vec3::new(2.0, 2.0, 10.0)) && near(p.x, Vec3::Y), "{p:?}");
    run(&mut s, "sketch.finish", json!({}));
    run(&mut s, "timeline.edit", json!({"feature":"Extrude1","set":{"extent":{"distance":"20 mm"}}}));
    let id = s.doc.find_feature("Top").unwrap().id;
    let p = s.world_state().sketch(id).unwrap().plane;
    assert!(near(p.origin, Vec3::new(2.0, 2.0, 20.0)) && near(p.x, Vec3::Y), "the sketch follows its face: {p:?}");
    let json = serde_json::to_string(&s.doc).unwrap();
    let doc: solidcraft_doc::Document = serde_json::from_str(&json).unwrap();
    assert_eq!(serde_json::to_string(&doc).unwrap(), json, "the reframed plane survives a round trip");
}
