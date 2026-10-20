//! Pattern Component and Replace Component.

use serde_json::{Value, json};
use solidcraft_geom::Vec3;

use crate::Session;

fn run(s: &mut Session, cmd: &str, p: Value) -> Value {
    s.execute(cmd, &p).unwrap_or_else(|e| panic!("{cmd} {p}: {e}"))
}

/// A part with a 10 mm cube at the origin of its own component, and a second one with a 20 mm cube.
fn two_parts() -> Session {
    let mut s = Session::default();
    run(&mut s, "component.create", json!({"name": "Small"}));
    run(&mut s, "solid.box", json!({"length": 10, "width": 10, "height": 10}));
    run(&mut s, "component.activate", json!({"component": "root"}));
    run(&mut s, "component.create", json!({"name": "Big"}));
    run(&mut s, "solid.box", json!({"length": 20, "width": 20, "height": 20}));
    run(&mut s, "component.activate", json!({"component": "root"}));
    s
}

fn placed(s: &Session, id: u64) -> Vec3 {
    let o = s.doc.occurrences.iter().find(|o| o.id == id).unwrap();
    Vec3::new(o.transform[3][0], o.transform[3][1], o.transform[3][2])
}

#[test]
fn a_linear_pattern_places_instances_at_the_spacing() {
    let mut s = two_parts();
    let r = run(&mut s, "occurrence.pattern", json!({"component": "Small", "direction": [1, 0, 0], "count": 4, "spacing": 25}));
    let ids: Vec<u64> = r["occurrences"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap()).collect();
    assert_eq!(ids.len(), 3, "the original and three copies make four");
    for (k, id) in ids.iter().enumerate() {
        assert!((placed(&s, *id).x - 25.0 * (k + 1) as f64).abs() < 1e-9);
    }
    let comp = s.doc.find_component("Small").unwrap();
    assert_eq!(s.doc.occurrences.iter().filter(|o| o.component == comp).count(), 4);
}

#[test]
fn a_circular_pattern_spreads_the_copies_round_the_axis() {
    let mut s = two_parts();
    let first = s.doc.occurrences.iter().find(|o| o.component == s.doc.find_component("Small").unwrap()).unwrap().id;
    let r = run(&mut s, "occurrence.pattern", json!({"occurrence": first, "layout": "circular", "count": 4, "origin": [100, 0, 0]}));
    let ids: Vec<u64> = r["occurrences"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap()).collect();
    assert_eq!(ids.len(), 3);
    // The original is at the origin, 100 mm from the axis: the copies are a quarter turn apart.
    let p = placed(&s, ids[0]);
    assert!((p.x - 100.0).abs() < 1e-6 && (p.y + 100.0).abs() < 1e-6, "{p:?}");
    let q = placed(&s, ids[1]);
    assert!((q.x - 200.0).abs() < 1e-6 && q.y.abs() < 1e-6, "{q:?}");
    // A part turn: the last copy is at the end.
    let r =
        run(&mut s, "occurrence.pattern", json!({"occurrence": first, "layout": "circular", "count": 3, "origin": [100, 0, 0], "angle": "90 deg"}));
    let ids: Vec<u64> = r["occurrences"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap()).collect();
    let end = placed(&s, ids[1]);
    assert!((end.x - 100.0).abs() < 1e-6 && (end.y + 100.0).abs() < 1e-6, "{end:?}");
}

#[test]
fn replace_swaps_the_component_and_keeps_the_place_and_the_fixed_state() {
    let mut s = two_parts();
    let small = s.doc.find_component("Small").unwrap();
    let occ = s.doc.occurrences.iter().find(|o| o.component == small).unwrap().id;
    run(&mut s, "occurrence.move", json!({"occurrence": occ, "translate": [50, 0, 0]}));
    run(&mut s, "occurrence.ground", json!({"occurrence": occ, "grounded": true}));
    run(&mut s, "component.replace", json!({"component": "Small", "with": "Big"}));
    let o = s.doc.occurrences.iter().find(|o| o.id == occ).unwrap();
    assert_eq!(o.component, s.doc.find_component("Big").unwrap());
    assert!(o.grounded);
    assert!((placed(&s, occ).x - 50.0).abs() < 1e-9);
    let v = run(&mut s, "occurrence.properties", json!({"occurrence": occ}))["total"]["volume_mm3"].as_f64().unwrap();
    assert!((v - 8000.0).abs() < 1e-6, "now the big cube: {v}");
}

#[test]
fn pattern_and_replace_refuse_what_cannot_be_done() {
    let mut s = two_parts();
    for (cmd, p) in [
        ("occurrence.pattern", json!({})),
        ("occurrence.pattern", json!({"component": "Small", "count": 1, "direction": [1, 0, 0], "spacing": 5})),
        ("occurrence.pattern", json!({"component": "Small", "count": 501, "direction": [1, 0, 0], "spacing": 5})),
        ("occurrence.pattern", json!({"component": "Small", "count": 3, "direction": [0, 0, 0], "spacing": 5})),
        ("occurrence.pattern", json!({"component": "Small", "count": 3, "direction": [1, 0, 0]})),
        ("occurrence.pattern", json!({"component": "Small", "count": 3, "direction": [1, 0, 0], "spacing": "oops"})),
        ("occurrence.pattern", json!({"component": "Small", "count": 3, "direction": [1, 0, 0], "spacing": 1e12})),
        ("occurrence.pattern", json!({"component": "Small", "count": 3, "layout": "circular", "axis": [0, 0, 0]})),
        ("occurrence.pattern", json!({"component": "Small", "count": 3, "layout": "spiral"})),
        ("component.replace", json!({"component": "Small"})),
        ("component.replace", json!({"component": "Small", "with": "nope"})),
        ("component.replace", json!({"component": "Small", "with": "root"})),
        ("component.replace", json!({"component": "root", "with": "Big"})),
    ] {
        assert!(s.execute(cmd, &p).is_err(), "{cmd} {p}");
    }
    // A component cannot replace one inside itself's own parent chain.
    run(&mut s, "component.create", json!({"name": "Outer"}));
    run(&mut s, "component.create", json!({"name": "Inner"}));
    run(&mut s, "solid.box", json!({"length": 5, "width": 5, "height": 5}));
    run(&mut s, "component.activate", json!({"component": "root"}));
    assert!(s.execute("component.replace", &json!({"component": "Inner", "with": "Outer"})).is_err(), "Outer contains the place Inner sits in");
}
