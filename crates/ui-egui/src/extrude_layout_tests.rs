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
