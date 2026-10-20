//! A datum's parents are the features it was built from.

use serde_json::json;

use crate::shell::part_nav_tree::{names_of, related};

fn id(app: &crate::SolveApp, name: &str) -> u64 {
    app.session.doc.find_feature(name).map(|f| f.id).unwrap_or(0)
}

#[test]
fn a_datum_depends_on_the_features_it_was_built_from() {
    let mut s = solidcraft_engine::Session::default();
    s.execute("solid.box", &json!({"length": 40, "width": 30, "height": 20, "body_name": "Box"})).unwrap();
    s.execute("solid.cylinder", &json!({"base": [100, 0, 0], "radius": 5, "height": 10, "body_name": "Cyl", "operation": "new"})).unwrap();
    let mut app = crate::SolveApp::new(s, crate::Services::default());
    let (box_id, cyl_id) = (app.session.doc.features[0].id, app.session.doc.features[1].id);
    app.run("construct.plane.offset", json!({"base": {"face": [20, 15, 20]}, "offset": 5, "name": "P"})).unwrap();
    app.run("construct.point.vertex", json!({"point": {"vertex": [100, 0, 0]}, "name": "V"})).unwrap();
    app.run("construct.axis.edge", json!({"edge": {"edge": [20, 0, 0]}, "name": "A"})).unwrap();
    let parents = |app: &crate::SolveApp, name: &str| related(app, id(app, name)).parents;
    assert!(parents(&app, "P").contains(&box_id), "P: {:?}", parents(&app, "P"));
    assert!(!parents(&app, "P").contains(&cyl_id));
    assert!(parents(&app, "V").contains(&cyl_id), "V: {:?}", parents(&app, "V"));
    assert!(parents(&app, "A").contains(&box_id), "A: {:?}", parents(&app, "A"));
    let (_, children) = names_of(&app, box_id);
    assert!(children.iter().any(|c| c == "P"), "{children:?}");
    // A sketch placed on a face depends on the feature that made it.
    app.run("sketch.create", json!({"plane": {"face": [20, 15, 20]}, "name": "S"})).unwrap();
    app.run("sketch.circle.center", json!({"center": [10, 10], "radius": 3})).unwrap();
    app.run("sketch.finish", json!({})).unwrap();
    assert!(parents(&app, "S").contains(&box_id), "S: {:?}", parents(&app, "S"));
    // A datum made from a datum depends on that datum.
    app.run("construct.point.between", json!({"a": {"construct_point": "V"}, "b": {"vertex": [100, 0, 10]}, "name": "M"})).unwrap();
    assert!(parents(&app, "M").contains(&id(&app, "V")), "{:?}", parents(&app, "M"));
    assert!(parents(&app, "M").contains(&cyl_id), "{:?}", parents(&app, "M"));
}
