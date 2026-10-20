//! Face mates: touch, align, distance and concentric, over the joint machinery.

use serde_json::json;
use solidcraft_geom::Vec3;

use super::tests::{place, run};
use crate::Session;

/// A grounded 40 x 40 x 10 base, and a free 20 x 20 x 5 block lying elsewhere.
fn pair() -> Session {
    let mut s = Session::default();
    run(&mut s, "component.create", json!({"name": "Base"}));
    run(&mut s, "solid.box", json!({"length": 40, "width": 40, "height": 10, "corner": [0, 0, 0]}));
    let base = super::tests::occ(&s, "Base");
    run(&mut s, "occurrence.ground", json!({"occurrence": base, "grounded": true}));
    run(&mut s, "component.activate", json!({"component": "root"}));
    run(&mut s, "component.create", json!({"name": "Block"}));
    run(&mut s, "solid.box", json!({"length": 20, "width": 20, "height": 5, "corner": [100, 100, 100]}));
    s
}

#[test]
fn touch_puts_the_block_on_the_base() {
    let mut s = pair();
    let r = run(
        &mut s,
        "mate.create",
        json!({"type": "touch", "a": {"occurrence": "Base", "face": [20, 20, 10]}, "b": {"occurrence": "Block", "face": [110, 110, 100]}}),
    );
    assert_eq!(r["conflicts"].as_array().map(Vec::len), Some(0), "{r}");
    // The block's bottom (z = 100) now lies on the base's top (z = 10), the block above it.
    assert!((place(&s, "Block", Vec3::new(110.0, 110.0, 100.0)).z - 10.0).abs() < 1e-6);
    assert!((place(&s, "Block", Vec3::new(110.0, 110.0, 105.0)).z - 15.0).abs() < 1e-6);
}

#[test]
fn distance_leaves_a_gap_and_align_flips_the_block() {
    let mut s = pair();
    run(
        &mut s,
        "mate.create",
        json!({"type": "distance", "offset": 3, "a": {"occurrence": "Base", "face": [20, 20, 10]}, "b": {"occurrence": "Block", "face": [110, 110, 100]}}),
    );
    let z = place(&s, "Block", Vec3::new(110.0, 110.0, 100.0)).z;
    assert!((z - 13.0).abs() < 1e-6 || (z - 7.0).abs() < 1e-6, "gap of 3 mm, got z = {z}");
    let mut s = pair();
    run(
        &mut s,
        "mate.create",
        json!({"type": "align", "a": {"occurrence": "Base", "face": [20, 20, 10]}, "b": {"occurrence": "Block", "face": [110, 110, 105]}}),
    );
    // Top to top: flush.
    assert!((place(&s, "Block", Vec3::new(110.0, 110.0, 105.0)).z - 10.0).abs() < 1e-6);
}

#[test]
fn concentric_shares_the_axis() {
    let mut s = Session::default();
    run(&mut s, "component.create", json!({"name": "Plate"}));
    run(&mut s, "solid.box", json!({"length": 30, "width": 20, "height": 6, "corner": [-15, -10, 0]}));
    let plate = super::tests::occ(&s, "Plate");
    run(&mut s, "occurrence.ground", json!({"occurrence": plate, "grounded": true}));
    run(&mut s, "component.activate", json!({"component": "root"}));
    run(&mut s, "component.create", json!({"name": "Knob"}));
    run(&mut s, "solid.cylinder", json!({"base": [50, 50, 6], "axis": [0, 0, 1], "radius": 10, "height": 10, "operation": "new"}));
    run(
        &mut s,
        "mate.create",
        json!({"type": "concentric", "a": {"occurrence": "Plate", "face": [0, 0, 6]}, "b": {"occurrence": "Knob", "circle": [50, 50, 6]}}),
    );
    let c = place(&s, "Knob", Vec3::new(50.0, 50.0, 6.0));
    assert!(c.x.abs() < 1e-6 && c.y.abs() < 1e-6, "{c:?}");
}

/// The direction B's face normal (its top, z up in the block) points in the world.
fn up(s: &Session) -> Vec3 {
    let o = place(s, "Block", Vec3::new(110.0, 110.0, 100.0));
    place(s, "Block", Vec3::new(110.0, 110.0, 105.0)) - o
}

fn mate(s: &mut Session, ty: &str, extra: serde_json::Value) {
    let mut p = json!({"type": ty, "a": {"occurrence": "Base", "face": [20, 20, 10]}, "b": {"occurrence": "Block", "face": [110, 110, 100]}});
    if let (Some(o), Some(e)) = (p.as_object_mut(), extra.as_object()) {
        o.extend(e.clone());
    }
    run(s, "mate.create", p);
}

#[test]
fn parallel_keeps_the_faces_parallel_and_perpendicular_square() {
    let mut s = pair();
    mate(&mut s, "parallel", json!({}));
    let u = up(&s);
    assert!((u.z.abs() - 5.0).abs() < 1e-6 && u.x.abs() < 1e-6 && u.y.abs() < 1e-6, "parallel: {u:?}");
    // Free to slide: a drive moves it in z, x and y.
    let j = s.doc.assembly.joints[0].name.clone();
    run(&mut s, "joint.drive", json!({"joint": j, "values": [7, 3, 4, 0]}));
    let at = place(&s, "Block", Vec3::new(110.0, 110.0, 100.0));
    // 4 mm above the face, and 7 and 3 mm across it (the face frame's x and y are its own choice).
    assert!((at.z - 14.0).abs() < 1e-6 && ((at.x - 20.0).hypot(at.y - 20.0) - 58f64.sqrt()).abs() < 1e-6, "{at:?}");
    let mut s = pair();
    mate(&mut s, "perpendicular", json!({}));
    let u = up(&s);
    assert!(u.z.abs() < 1e-6 && (u.x.hypot(u.y) - 5.0).abs() < 1e-6, "perpendicular: {u:?}");
}

#[test]
fn angle_holds_the_faces_at_the_angle() {
    let mut s = pair();
    mate(&mut s, "angle", json!({"angle": "30 deg"}));
    let u = up(&s);
    // 30 degrees from the base's normal (or its opposite), so z is 5·cos 30 either way.
    assert!((u.z.abs() - 5.0 * 30f64.to_radians().cos()).abs() < 1e-6, "{u:?}");
}

#[test]
fn fix_grounds_the_component_of_the_face() {
    let mut s = pair();
    run(&mut s, "mate.create", json!({"type": "fix", "a": {"occurrence": "Block", "face": [110, 110, 100]}}));
    let o = s.doc.occurrences.iter().find(|o| o.component == s.doc.find_component("Block").unwrap_or(0)).map(|o| o.grounded);
    assert_eq!(o, Some(true));
    assert!(s.execute("mate.create", &json!({"type": "fix"})).is_err());
    assert!(s.execute("mate.create", &json!({"type": "fix", "a": "root"})).is_err());
}

#[test]
fn hostile_parameters_are_refused() {
    let mut s = pair();
    let face = |o: &str, z: f64| json!({"occurrence": o, "face": [20, 20, z]});
    for p in [
        json!({}),
        json!({"type": "wobble", "a": face("Base", 10.0), "b": face("Block", 100.0)}),
        json!({"type": "distance", "a": face("Base", 10.0), "b": face("Block", 100.0)}),
        json!({"type": "concentric", "offset": 1, "a": face("Base", 10.0), "b": face("Block", 100.0)}),
        json!({"type": "touch", "a": face("Base", 10.0), "b": face("Base", 0.0)}),
        json!({"type": "touch", "a": 7, "b": null}),
        json!({"type": "distance", "offset": "nan", "a": face("Base", 10.0), "b": face("Block", 100.0)}),
        json!({"type": "angle", "a": face("Base", 10.0), "b": face("Block", 100.0)}),
        json!({"type": "touch", "angle": "10 deg", "a": face("Base", 10.0), "b": face("Block", 100.0)}),
        json!({"type": "angle", "angle": "oops", "a": face("Base", 10.0), "b": face("Block", 100.0)}),
    ] {
        assert!(s.execute("mate.create", &p).is_err(), "{p}");
    }
}
