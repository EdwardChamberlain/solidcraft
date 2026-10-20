//! The Advanced Extrude dialog: Section, Direction (a vector), Limits (Start and End), Offset.
use crate::dialogs::{Kind, apply_commands};
use crate::prefs::ModellingApproach;
use crate::scenario::Harness;
use serde_json::{Value, json};
use solidcraft_engine::Sel;

fn open(approach: ModellingApproach) -> Harness {
    let mut h = Harness::new();
    h.app.preferences.modelling_approach = approach;
    h.app.run("sketch.create", json!({"plane":"XY","name":"Base"})).unwrap();
    h.app.run("sketch.rectangle.two_point", json!({"p0":[0,0],"p1":[20,10]})).unwrap();
    h.app.run("sketch.finish", json!({})).unwrap();
    h.step(&json!({"call":"ui.start","params":{"command":"solid.extrude"}})).unwrap();
    h.frames(6);
    h
}

fn params(h: &Harness) -> Value {
    apply_commands(&h.app, h.app.dialog.as_ref().unwrap()).unwrap().into_iter().last().unwrap().1
}

fn group(name: &str) -> bool {
    crate::scenario::handle_at(&format!("dialog:group:{name}")).is_some()
}

#[test]
fn the_advanced_layout_has_section_direction_limits_offset_draft_and_boolean() {
    let h = open(ModellingApproach::Advanced);
    for g in ["extrude_section", "extrude_direction", "extrude_limits", "extrude_offset", "extrude_draft", "extrude_boolean"] {
        assert!(group(g), "{g}");
    }
    assert!(h.widget("Thin Extrude").is_none(), "no thin checkbox: the wall is an offset");
}

#[test]
fn classic_keeps_its_own_layout() {
    let h = open(ModellingApproach::Classic);
    assert!(h.widget("Thin Extrude").is_some());
    assert!(!group("extrude_direction") && !group("extrude_offset"));
    assert!(h.app.dialog.as_ref().unwrap().specs.is_empty());
}

#[test]
fn start_and_end_are_the_commands_start_distances_and_extent() {
    let mut h = open(ModellingApproach::Advanced);
    let base = params(&h);
    assert_eq!(base["direction"], "positive");
    assert!(base.get("start_offset").is_none() && base.get("through_all").is_none());

    // Value: a start offset.
    if let Some(Kind::Extrude { start, .. }) = h.app.dialog.as_mut().map(|d| &mut d.kind) {
        *start = "-5 mm".into();
    }
    assert_eq!(params(&h)["start_offset"], "-5 mm");

    // Value (Other Side): two sides, the start's distance being the other side's.
    if let Some(Kind::Extrude { start, direction, distance2, .. }) = h.app.dialog.as_mut().map(|d| &mut d.kind) {
        start.clear();
        *direction = crate::dialogs::TWO_SIDES;
        *distance2 = "7 mm".into();
    }
    let p = params(&h);
    assert_eq!(p["distance2"], "7 mm");

    // Through All.
    if let Some(Kind::Extrude { all, direction, .. }) = h.app.dialog.as_mut().map(|d| &mut d.kind) {
        *all = true;
        *direction = 0;
    }
    assert_eq!(params(&h)["through_all"], true);
}

#[test]
fn the_offset_group_is_the_thin_wall() {
    let mut h = open(ModellingApproach::Advanced);
    assert!(params(&h).get("thin").is_none(), "None: a solid");
    if let Some(Kind::Extrude { thin, wall, wall_side, .. }) = h.app.dialog.as_mut().map(|d| &mut d.kind) {
        *thin = true;
        *wall = "3 mm".into();
        *wall_side = 2;
    }
    let p = params(&h);
    assert_eq!((p["thin"].clone(), p["side"].clone(), p["thickness"].clone()), (json!(true), json!("outside"), json!("3 mm")));
    if let Some(Kind::Extrude { wall_side, wall2, .. }) = h.app.dialog.as_mut().map(|d| &mut d.kind) {
        *wall_side = 3;
        *wall2 = "1 mm".into();
    }
    let p = params(&h);
    assert_eq!((p["side"].clone(), p["thickness2"].clone()), (json!("two_sides"), json!("1 mm")));
}

#[test]
fn the_vector_reverses_the_extrude_and_keeps_in_step_with_the_dialog() {
    let mut h = open(ModellingApproach::Advanced);
    assert_eq!(params(&h)["direction"], "positive");
    h.app.dialog.as_mut().unwrap().specs[0].reverse = true;
    h.frames(3);
    assert_eq!(params(&h)["direction"], "negative", "Reverse on the vector reverses the extrude");
    // The other way round: the dialog's own direction (the picked side, a canvas drag) shows in Reverse.
    if let Some(Kind::Extrude { direction, .. }) = h.app.dialog.as_mut().map(|d| &mut d.kind) {
        *direction = 0;
    }
    h.frames(3);
    assert!(!h.app.dialog.as_ref().unwrap().specs[0].reverse);
}

#[test]
fn a_vector_along_the_profile_normal_is_accepted_and_one_across_it_is_refused() {
    let mut h = open(ModellingApproach::Advanced);
    let set = |h: &mut Harness, axis: &str| {
        let d = h.app.dialog.as_mut().unwrap();
        let at = d.inputs.iter().position(|i| i.owner == 1).unwrap();
        d.inputs[at].items = vec![Sel::Axis { name: axis.into() }];
    };
    set(&mut h, "Z");
    h.frames(3);
    assert_eq!(params(&h)["direction"], "positive");
    set(&mut h, "X");
    h.frames(3);
    let err = apply_commands(&h.app, h.app.dialog.as_ref().unwrap()).unwrap_err();
    assert!(err.contains("not supported yet"), "{err}");
}

#[test]
fn an_empty_vector_does_not_stop_the_middle_click_or_the_next_pick() {
    let h = open(ModellingApproach::Advanced);
    let d = h.app.dialog.as_ref().unwrap();
    let at = d.inputs.iter().position(|i| i.owner == 1).unwrap();
    assert!(crate::specifier::is_optional(d, at));
    assert_ne!(d.active, at, "the active input is the profile, not the optional vector");
}

#[test]
fn the_start_and_end_choices_work_through_the_dialog() {
    let mut h = open(ModellingApproach::Advanced);
    assert!(h.widget("Start").is_some() && h.widget("End").is_some());
    h.step(&json!({"click":{"handle":"dialog:Start"}})).unwrap();
    h.step(&json!({"click":{"handle":"dialog:Start:Value (Other Side)"}})).unwrap();
    h.frames(4);
    assert!(matches!(h.app.dialog.as_ref().unwrap().kind, Kind::Extrude { direction, .. } if direction == crate::dialogs::TWO_SIDES));
    h.step(&json!({"click":{"handle":"dialog:End"}})).unwrap();
    h.step(&json!({"click":{"handle":"dialog:End:Through All"}})).unwrap();
    h.frames(4);
    assert!(matches!(h.app.dialog.as_ref().unwrap().kind, Kind::Extrude { all: true, .. }));
    h.step(&json!({"click":{"handle":"dialog:group:extrude_offset"}})).unwrap();
    h.frames(3);
    h.step(&json!({"click":{"handle":"dialog:Type"}})).unwrap();
    h.step(&json!({"click":{"handle":"dialog:Type:Outside"}})).unwrap();
    h.frames(4);
    assert!(matches!(h.app.dialog.as_ref().unwrap().kind, Kind::Extrude { thin: true, wall_side: 2, .. }));
}

fn extrude(h: &Harness) -> (String, String, Option<(String, String)>) {
    match &h.app.dialog.as_ref().unwrap().kind {
        Kind::Extrude { distance, start, plane_end, .. } => (distance.clone(), start.clone(), plane_end.clone()),
        other => panic!("{other:?}"),
    }
}

#[test]
fn the_end_value_is_measured_from_the_sketch_plane_and_value_from_start_from_the_start() {
    let mut h = open(ModellingApproach::Advanced);
    // Value, measured from the sketch plane: 10 mm there, with the start 4 mm up, is 6 from the start.
    h.step(&json!({"click":{"handle":"dialog:Start:Value"}})).ok();
    if let Some(Kind::Extrude { start, .. }) = h.app.dialog.as_mut().map(|d| &mut d.kind) {
        *start = "4 mm".into();
    }
    h.frames(4);
    let (distance, start, plane_end) = extrude(&h);
    assert_eq!((distance.as_str(), start.as_str()), ("6 mm", "4 mm"));
    assert_eq!(plane_end.map(|p| p.0), Some("10 mm".to_string()), "what is shown is still the value from the plane");
    let p = params(&h);
    assert_eq!((p["distance"].clone(), p["start_offset"].clone()), (json!("6 mm"), json!("4 mm")));

    // Value from Start: the number is the length from the start, as the command's distance.
    h.step(&json!({"click":{"handle":"dialog:End"}})).unwrap();
    h.step(&json!({"click":{"handle":"dialog:End:Value from Start"}})).unwrap();
    h.frames(4);
    let (distance, _, plane_end) = extrude(&h);
    assert_eq!((distance.as_str(), plane_end), ("6 mm", None));
    if let Some(Kind::Extrude { distance, .. }) = h.app.dialog.as_mut().map(|d| &mut d.kind) {
        *distance = "20 mm".into();
    }
    h.frames(3);
    assert_eq!(params(&h)["distance"], "20 mm");

    // Back to Value: the length from the start becomes the end from the plane.
    h.step(&json!({"click":{"handle":"dialog:End"}})).unwrap();
    h.step(&json!({"click":{"handle":"dialog:End:Value"}})).unwrap();
    h.frames(4);
    let (distance, _, plane_end) = extrude(&h);
    assert_eq!((distance.as_str(), plane_end.map(|p| p.0)), ("20 mm", Some("24 mm".to_string())));
}

#[test]
fn a_distance_changed_elsewhere_is_read_back_into_the_end_value() {
    let mut h = open(ModellingApproach::Advanced);
    if let Some(Kind::Extrude { start, .. }) = h.app.dialog.as_mut().map(|d| &mut d.kind) {
        *start = "2 mm".into();
    }
    h.frames(3);
    // A drag on the canvas changes the distance itself.
    if let Some(Kind::Extrude { distance, .. }) = h.app.dialog.as_mut().map(|d| &mut d.kind) {
        *distance = "13 mm".into();
    }
    h.frames(3);
    let (distance, _, plane_end) = extrude(&h);
    assert_eq!((distance.as_str(), plane_end.map(|p| p.0)), ("13 mm", Some("15 mm".to_string())));
}

#[test]
fn symmetric_is_an_end_not_a_start() {
    let mut h = open(ModellingApproach::Advanced);
    h.step(&json!({"click":{"handle":"dialog:Start"}})).unwrap();
    assert!(crate::scenario::handle_at("dialog:Start:Symmetric Value").is_none(), "the Start has no Symmetric Value");
    assert!(crate::scenario::handle_at("dialog:Start:Value (Other Side)").is_some());
    h.step(&json!({"click":{"handle":"dialog:Start:Value"}})).unwrap();
    h.step(&json!({"click":{"handle":"dialog:End"}})).unwrap();
    h.step(&json!({"click":{"handle":"dialog:End:Symmetric Value"}})).unwrap();
    h.frames(4);
    let p = params(&h);
    assert_eq!(p["direction"], "symmetric");
    assert!(p.get("start_offset").is_none(), "a symmetric extrude has no start offset");
    assert!(crate::scenario::handle_at("dialog:Start").is_none(), "and no Start to choose");
}
