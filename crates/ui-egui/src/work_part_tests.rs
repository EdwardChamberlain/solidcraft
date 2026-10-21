//! Editing a part with its assembly around it (the work part).
use crate::prefs::ModellingApproach;
use crate::scenario::Harness;
use crate::work_part::{CONTEXT_PREFIX, context_bodies, make_work_part, return_to_assembly};
use serde_json::json;
use solidcraft_engine::doc::Approach;

/// An assembly (`machine`, saved) of two parts, Bracket and Pin, each modelled in its own tab.
/// Bracket stands 20 mm along X in the assembly; Pin 70 mm. Returns the harness with the assembly's
/// tab on show, and the folder to clean up.
fn assembly(tag: &str) -> (Harness, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("solidcraft-work-part-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut h = Harness::new();
    h.app.preferences.modelling_approach = ModellingApproach::Advanced;
    h.app.session.doc_mut().approach = Approach::Advanced;
    h.frames(2);
    h.app.run("file.save", json!({"path": dir.join("machine.solidcraft")})).unwrap();
    h.app.run("component.new_part", json!({"name": "Bracket", "translate": [20, 0, 0]})).unwrap();
    h.app.run("component.new_part", json!({"name": "Pin", "translate": [70, 0, 0]})).unwrap();
    for (name, boxed) in [
        ("Bracket", json!({"length": 60, "width": 30, "height": 10, "body_name": "Plate"})),
        ("Pin", json!({"length": 10, "width": 10, "height": 40, "body_name": "Pin", "corner": [0, 10, 0]})),
    ] {
        h.app.open_path(&dir.join(format!("{name}.solidcraft")).display().to_string());
        h.frames(2);
        h.app.run("solid.box", boxed).unwrap();
        h.app.run("file.save", json!({})).unwrap();
    }
    crate::documents::switch(&mut h.app, 0);
    h.frames(2);
    assert_eq!(h.app.session.world_state().bodies.len(), 2, "the assembly holds both parts");
    (h, dir)
}

fn component(h: &Harness, name: &str) -> u64 {
    h.app.session.doc.components.iter().find(|c| c.name == name).unwrap().id
}

fn x_range(b: &solidcraft_engine::doc::ModelBody) -> (f64, f64) {
    let bounds = b.mesh().bounds();
    (bounds.min.x, bounds.max.x)
}

#[test]
fn a_work_part_shows_the_rest_of_the_assembly_in_its_own_frame() {
    let (mut h, dir) = assembly("frame");
    let bracket = component(&h, "Bracket");
    assert!(context_bodies(&h.app).is_none(), "the assembly itself has no context");
    crate::shell::advanced::set_pane(&h.ctx, crate::shell::advanced::Pane::Assembly);
    h.frames(2);
    make_work_part(&mut h.app, bracket).unwrap();
    h.frames(3);
    assert!(h.app.session.path.as_deref().is_some_and(|p| p.ends_with("Bracket.solidcraft")), "the part's tab is on show");
    let work = h.app.work.clone().expect("this tab is a work part");
    assert_eq!((work.component, work.name.as_str(), work.show), (bracket, "Bracket", true));
    // The Part Navigator, with the work-part banner, is brought forward once.
    assert_eq!(crate::shell::advanced::pane(&h.ctx), crate::shell::advanced::Pane::Part);
    assert!(crate::scenario::rect_of("part_nav:return").is_some(), "the banner offers the way back");
    assert!(!h.app.work.as_ref().unwrap().fresh);
    // The Pin stands 70 mm along in the assembly and Bracket 20, so 50 mm along in Bracket's frame.
    let context = context_bodies(&h.app).expect("context bodies");
    assert_eq!(context.len(), 1, "the part itself is not part of its context");
    assert!(context[0].name.starts_with(CONTEXT_PREFIX), "{}", context[0].name);
    let (min, max) = x_range(&context[0]);
    assert!((min - 50.0).abs() < 1e-6 && (max - 60.0).abs() < 1e-6, "{min}..{max}");
    // The part's own world has none of it: nothing the part can pick or refer to.
    assert!(h.app.session.world_state().bodies.iter().all(|b| !b.name.starts_with(CONTEXT_PREFIX)));
    assert_eq!(h.app.session.world_state().bodies.len(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_context_can_be_hidden_and_the_way_back_goes_to_the_assembly() {
    let (mut h, dir) = assembly("back");
    let bracket = component(&h, "Bracket");
    make_work_part(&mut h.app, bracket).unwrap();
    let (_, before) = crate::viewport::display_scene_keys(&h.app, &crate::viewport::projection(&h.app, h.app.viewport.rect.unwrap()));
    h.app.work.as_mut().unwrap().show = false;
    assert!(context_bodies(&h.app).is_none());
    let (_, hidden) = crate::viewport::display_scene_keys(&h.app, &crate::viewport::projection(&h.app, h.app.viewport.rect.unwrap()));
    assert_ne!(before, hidden, "the scene is redrawn when the context goes");
    h.app.work.as_mut().unwrap().show = true;
    // Back to the assembly: its tab is on show and is no work part; the part's tab keeps being one.
    assert!(return_to_assembly(&mut h.app));
    assert!(h.app.work.is_none() && h.app.session.doc.name == "machine");
    crate::documents::switch(&mut h.app, 1);
    assert!(h.app.work.is_some(), "the part's tab is still the work part");
    // Asking again from the assembly goes to the open tab, not a second one.
    let tabs = h.app.docs.count();
    crate::documents::switch(&mut h.app, 0);
    make_work_part(&mut h.app, bracket).unwrap();
    assert_eq!(h.app.docs.count(), tabs);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_unsaved_assembly_or_a_missing_part_cannot_be_made_the_work_part() {
    let mut h = Harness::new();
    h.app.preferences.modelling_approach = ModellingApproach::Advanced;
    assert!(make_work_part(&mut h.app, 1).unwrap_err().contains("Save the assembly first"));
    let (mut h, dir) = assembly("missing");
    let bracket = component(&h, "Bracket");
    std::fs::remove_file(dir.join("Bracket.solidcraft")).unwrap();
    // Nothing else is open for it either: close its tab.
    let tab = crate::documents::tab_of(&h.app, &dir.join("Bracket.solidcraft").display().to_string()).unwrap();
    crate::documents::close(&mut h.app, tab, true);
    assert!(make_work_part(&mut h.app, bracket).unwrap_err().contains("not found"));
    assert!(h.app.work.is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn editing_the_work_part_reaches_the_assembly_when_it_is_shown_again() {
    let (mut h, dir) = assembly("edit");
    let bracket = component(&h, "Bracket");
    make_work_part(&mut h.app, bracket).unwrap();
    // A feature made in the part, as in any tab: a hole-sized boss on the plate.
    h.app.run("solid.box", json!({"length": 10, "width": 10, "height": 10, "body_name": "Boss", "corner": [0, 0, 10]})).unwrap();
    assert_eq!(h.app.session.world_state().bodies.len(), 2);
    assert!(context_bodies(&h.app).is_some(), "still around the part after the edit");
    assert!(return_to_assembly(&mut h.app));
    h.frames(2);
    assert_eq!(h.app.session.world_state().bodies.len(), 3, "Plate, Boss and Pin, from the part as it is now (unsaved)");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_selector_can_pick_a_face_of_the_assembly_around_the_work_part() {
    use solidcraft_engine::geom::Vec3;
    let (mut h, dir) = assembly("pick");
    let bracket = component(&h, "Bracket");
    crate::shell::advanced::set_pane(&h.ctx, crate::shell::advanced::Pane::Assembly);
    h.frames(2);
    make_work_part(&mut h.app, bracket).unwrap();
    h.frames(3);
    // The sketch dialog's plane selector, with the Pin (the assembly, 50 mm along) in view from above.
    h.app.start("sketch.create");
    h.frames(3);
    h.call("ui.view", json!({"view": "top"}));
    h.app.cam.target = Vec3::new(55.0, 15.0, 20.0);
    h.app.cam.distance = 150.0;
    h.frames(4);
    h.step(&json!({"click": {"world": [55.0, 15.0, 40.0]}})).unwrap();
    h.frames(3);
    let d = h.app.dialog.as_ref().expect("the sketch dialog");
    let picked = d.inputs.iter().flat_map(|i| i.items.iter()).next().cloned();
    assert!(
        matches!(&picked, Some(solidcraft_engine::Sel::Face { body, .. }) if body.starts_with(CONTEXT_PREFIX)),
        "the Pin's top face was picked: {picked:?}"
    );
    // It makes the sketch on that plane, by value: 40 mm up, in the part's frame.
    h.step(&json!({"widget": "OK"})).unwrap();
    h.frames(4);
    let sid = h.app.session.active_sketch.expect("a sketch on the picked plane");
    let origin = h.app.session.world_state().sketch(sid).unwrap().plane.origin;
    assert!((origin.z - 40.0).abs() < 1e-6, "{origin:?}");
    // Without a selector waiting for it, the assembly is not pickable.
    h.app.dialog = None;
    h.frames(2);
    assert!(crate::work_part::pickable(&h.app).is_none());
    let _ = std::fs::remove_dir_all(&dir);
}
