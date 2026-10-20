//! The Advanced selection bar: type filter, scope and intent.
use crate::prefs::ModellingApproach;
use crate::scenario::Harness;
use crate::selection::{Intent, Scope};
use crate::shell::selection_bar::{FILTERS, current_filter, set_filter};
use crate::viewport::{Hit, candidate};
use serde_json::json;
use solidcraft_engine::Sel;
use solidcraft_engine::geom::Vec3;

fn advanced() -> Harness {
    let mut h = Harness::new();
    h.app.preferences.modelling_approach = ModellingApproach::Advanced;
    h
}

/// A box in the part itself and one in a component.
fn two_places() -> Harness {
    let mut h = advanced();
    h.app.run("solid.box", json!({"length":10,"width":10,"height":10,"body_name":"Own"})).unwrap();
    h.app.run("component.create", json!({"name":"Block"})).unwrap();
    h.app.run("solid.box", json!({"length":10,"width":10,"height":10,"corner":[30,0,0],"body_name":"Inner"})).unwrap();
    h.app.run("component.activate", json!({"component":0})).unwrap();
    h.frames(4);
    h
}

fn face(body: &str) -> Hit {
    Hit::Face { body: body.into(), index: 0, point: Vec3::new(1.0, 1.0, 10.0) }
}

fn edge(body: &str) -> Hit {
    Hit::Edge { body: body.into(), index: 0, mid: Vec3::new(5.0, 0.0, 10.0) }
}

#[test]
fn every_type_filter_is_recognised_and_a_mix_is_custom() {
    let mut h = advanced();
    for (i, (name, ..)) in FILTERS.iter().enumerate() {
        set_filter(&mut h.app.ui, i);
        assert_eq!(current_filter(&h.app.ui), Some(i), "{name}");
    }
    h.app.ui.pick_off = vec!["edges".into()];
    assert_eq!(current_filter(&h.app.ui), None, "a hand-made mix is Custom");
}

#[test]
fn the_type_filter_decides_what_a_click_picks() {
    let mut h = two_places();
    let hits = [edge("Own"), face("Own")];
    let first = |h: &Harness| candidate(&h.app, &hits).map(|(_, s)| s);
    assert!(matches!(first(&h), Some(Sel::Edge { .. })), "all: the nearest hit");
    set_filter(&mut h.app.ui, FILTERS.iter().position(|f| f.0 == "Faces").unwrap());
    assert!(matches!(first(&h), Some(Sel::Face { .. })), "faces only");
    set_filter(&mut h.app.ui, FILTERS.iter().position(|f| f.0 == "Bodies").unwrap());
    assert!(matches!(first(&h), Some(Sel::Body { .. })), "bodies: a face picks its body");
    set_filter(&mut h.app.ui, FILTERS.iter().position(|f| f.0 == "Vertices").unwrap());
    assert!(first(&h).is_none(), "nothing here is a vertex");
}

#[test]
fn the_type_filter_reaches_the_picks_of_a_dialog_in_advanced_only() {
    let mut h = two_places();
    h.app.start("solid.fillet");
    h.frames(4);
    let hits = [face("Own"), edge("Own")];
    set_filter(&mut h.app.ui, FILTERS.iter().position(|f| f.0 == "Edges").unwrap());
    let c = candidate(&h.app, &hits).map(|(_, s)| s);
    assert!(matches!(c, Some(Sel::Edge { .. })), "the Fillet takes faces and edges; the filter leaves the edge: {c:?}");
    h.app.preferences.modelling_approach = ModellingApproach::Classic;
    let c = candidate(&h.app, &hits).map(|(_, s)| s);
    assert!(matches!(c, Some(Sel::Face { .. })), "Classic's filter never reached a dialog: {c:?}");
}

#[test]
fn the_scope_keeps_picks_to_the_work_part_or_lets_its_components_in() {
    let mut h = two_places();
    let hits = [face("Inner"), face("Own")];
    let picked = |h: &Harness| candidate(&h.app, &hits).and_then(|(hit, _)| if let Hit::Face { body, .. } = hit { Some(body) } else { None });
    assert_eq!(picked(&h).as_deref(), Some("Inner"), "the whole design");
    h.app.ui.selection_scope = Scope::WorkPart;
    assert_eq!(picked(&h).as_deref(), Some("Own"), "the work part only: the component is skipped");
    h.app.ui.selection_scope = Scope::WorkPartAndComponents;
    assert_eq!(picked(&h).as_deref(), Some("Inner"), "the work part and its components");
    h.app.preferences.modelling_approach = ModellingApproach::Classic;
    h.app.ui.selection_scope = Scope::WorkPart;
    assert_eq!(picked(&h).as_deref(), Some("Inner"), "Classic has no scope");
}

#[test]
fn the_intent_takes_the_edges_that_continue_the_one_picked() {
    let mut h = advanced();
    h.app.run("sketch.create", json!({"plane":"XY","name":"Base"})).unwrap();
    h.app.run("sketch.rectangle.two_point", json!({"p0":[0,0],"p1":[20,10]})).unwrap();
    h.app.run("sketch.finish", json!({})).unwrap();
    h.app.run("solid.extrude", json!({"sketch":"Base","distance":"10 mm","operation":"new","body_name":"Main"})).unwrap();
    h.app.run("solid.fillet", json!({"edges":[[20,5,10]],"radius":"2 mm"})).unwrap();
    h.frames(2);
    let st = h.app.session.world_state();
    let b = st.body("Main").unwrap();
    let m = b.mesh();
    // An edge of the fillet's rounded run: a tangent chain takes it with its neighbours.
    let start = (0..m.edges.len()).find(|i| m.tangent_chain(*i, 2f64.to_radians()).len() > 1).expect("a tangent run");
    let at = m.edges[start].first().copied().unwrap();
    let sel = Sel::Edge { body: "Main".into(), index: start, point: at };
    crate::selection::set_intent(Intent::Single);
    let single = crate::viewport::grouped_selection(&h.app.session, &sel).len();
    crate::selection::set_intent(Intent::TangentChain);
    let chain = crate::viewport::grouped_selection(&h.app.session, &sel).len();
    crate::selection::set_intent(Intent::Single);
    assert!(chain > single, "the chain takes {chain}, the single pick {single}");
}

#[test]
fn the_bar_has_a_filter_a_scope_and_an_intent_and_they_change_the_state() {
    let mut h = two_places();
    for handle in ["selection:filter", "selection:scope", "selection:intent"] {
        assert!(crate::scenario::handle_at(handle).is_some(), "{handle}");
    }
    h.step(&json!({"click":{"handle":"selection:scope"}})).unwrap();
    h.step(&json!({"click":{"handle":"selection:scope:Work Part Only"}})).unwrap();
    h.frames(2);
    assert_eq!(h.app.ui.selection_scope, Scope::WorkPart);
    h.step(&json!({"click":{"handle":"selection:filter"}})).unwrap();
    h.step(&json!({"click":{"handle":"selection:filter:Edges"}})).unwrap();
    h.frames(2);
    assert_eq!(current_filter(&h.app.ui).map(|i| FILTERS[i].0), Some("Edges"));
    h.step(&json!({"click":{"handle":"selection:intent"}})).unwrap();
    h.step(&json!({"click":{"handle":"selection:intent:Tangent Chain"}})).unwrap();
    h.frames(2);
    assert_eq!(h.app.ui.selection_intent, Intent::TangentChain);
    assert_eq!(crate::selection::intent(), Intent::TangentChain, "the picks that follow use it");
    // The state is visible to scripts and agents.
    assert!(h.app.prefs().contains("\"selection_scope\":\"work_part\""));
}

#[test]
fn face_edges_and_connected_curves_take_the_boundary_and_the_whole_wire_with_an_edge() {
    let mut h = advanced();
    h.app.run("solid.box", json!({"length": 20, "width": 20, "height": 20, "body_name": "Cube"})).unwrap();
    h.frames(4);
    let st = h.app.session.world_state();
    let m = st.body("Cube").unwrap().mesh();
    let at = m.edges[0].first().copied().unwrap();
    let sel = Sel::Edge { body: "Cube".into(), index: 0, point: at };
    let count = |intent: Intent| {
        crate::selection::set_intent(intent);
        let n = crate::viewport::grouped_selection(&h.app.session, &sel).len();
        crate::selection::set_intent(Intent::Single);
        n
    };
    assert_eq!(count(Intent::Single), 1);
    assert_eq!(count(Intent::FaceEdges), 4, "a face of a cube has four edges");
    assert_eq!(count(Intent::Connected), 12, "all the cube's edges meet end to end");
}

#[test]
fn the_snap_points_menu_turns_each_inference_off_and_the_state_reaches_the_picks() {
    let mut h = advanced();
    h.frames(4);
    assert!(crate::scenario::handle_at("selection:snap").is_some());
    assert!(crate::selection::snap_on(1) && crate::selection::snap_on(16), "all on to start");
    h.step(&json!({"click":{"handle":"selection:snap"}})).unwrap();
    h.step(&json!({"click":{"handle":"selection:snap:Midpoint"}})).unwrap();
    h.frames(3);
    assert!(!crate::selection::snap_on(1) && crate::selection::snap_on(2), "the midpoint inference is off, the others on");
    assert_eq!(h.app.ui.snap_kinds & 1, 0);
    // It is part of the saved state scripts and agents see.
    let state: serde_json::Value = serde_json::from_str(&h.app.prefs()).unwrap();
    assert_eq!(state["snap_kinds"], json!(h.app.ui.snap_kinds));
}

#[test]
fn an_advanced_sketch_does_not_snap_to_a_model_vertex_so_nothing_is_projected() {
    let mut h = advanced();
    h.app.run("solid.box", json!({"length": 20, "width": 20, "height": 20})).unwrap();
    h.app.run("sketch.create", json!({"plane": "XY", "name": "S"})).unwrap();
    h.frames(4);
    let st = h.app.session.world_state();
    let ss = st.sketch(h.app.session.active_sketch.unwrap()).unwrap();
    let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 600.0));
    h.app.fit_view();
    let proj = crate::viewport::projection(&h.app, rect);
    let on_vertex = ss.plane.to_local(solidcraft_engine::geom::Vec3::new(20.0, 20.0, 0.0));
    let snapped = crate::sketch_tools::refine_snap(&h.app, &proj, &ss.sketch, &ss.plane, on_vertex);
    assert!(snapped.as_ref().is_none_or(|(_, arg)| arg.as_deref().is_none_or(|a| !a.starts_with("vertex:"))), "{snapped:?}");
    // Classic still does: the control for this test.
    h.app.preferences.modelling_approach = ModellingApproach::Classic;
    let classic = crate::sketch_tools::refine_snap(&h.app, &proj, &ss.sketch, &ss.plane, on_vertex);
    assert!(classic.is_some_and(|(_, arg)| arg.is_some_and(|a| a.starts_with("vertex:"))), "Classic snaps to the vertex");
}
