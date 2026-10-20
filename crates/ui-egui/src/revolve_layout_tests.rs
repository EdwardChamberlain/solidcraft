//! The Advanced Revolve dialog: Section, Axis (a vector), Limits (Start and End), Boolean.
use crate::dialogs::{Kind, apply_commands};
use crate::prefs::ModellingApproach;
use crate::scenario::Harness;
use serde_json::{Value, json};
use solidcraft_engine::Sel;

fn open(approach: ModellingApproach) -> Harness {
    let mut h = Harness::new();
    h.app.preferences.modelling_approach = approach;
    h.app.run("sketch.create", json!({"plane":"XY","name":"Base"})).unwrap();
    h.app.run("sketch.rectangle.two_point", json!({"p0":[10,0],"p1":[20,10]})).unwrap();
    h.app.run("sketch.finish", json!({})).unwrap();
    h.step(&json!({"call":"ui.start","params":{"command":"solid.revolve"}})).unwrap();
    h.frames(6);
    h
}

fn params(h: &Harness) -> Value {
    apply_commands(&h.app, h.app.dialog.as_ref().unwrap()).unwrap().into_iter().last().unwrap().1
}

fn group(name: &str) -> bool {
    crate::scenario::handle_at(&format!("dialog:group:{name}")).is_some()
}

fn pick_axis(h: &mut Harness, sel: Sel) {
    let d = h.app.dialog.as_mut().unwrap();
    let at = d.inputs.iter().position(|i| i.owner == 1).unwrap();
    d.inputs[at].items = vec![sel];
}

#[test]
fn the_advanced_layout_has_section_axis_limits_and_boolean() {
    let h = open(ModellingApproach::Advanced);
    for g in ["revolve_section", "revolve_axis", "revolve_limits", "revolve_boolean"] {
        assert!(group(g), "{g}");
    }
    assert!(h.widget("Start").is_some() && h.widget("End").is_some());
    assert_eq!(h.app.dialog.as_ref().unwrap().inputs.len(), 2, "the profile and the axis, as in the command");
}

#[test]
fn classic_keeps_its_own_layout() {
    let h = open(ModellingApproach::Classic);
    assert!(h.app.dialog.as_ref().unwrap().specs.is_empty());
    assert!(!group("revolve_axis"));
}

#[test]
fn the_axis_is_a_name_a_line_or_a_reversed_or_computed_line() {
    let mut h = open(ModellingApproach::Advanced);
    pick_axis(&mut h, Sel::Axis { name: "Y".into() });
    h.frames(3);
    assert_eq!(params(&h)["axis"], "Y", "a picked axis goes as it is, so the revolve follows it");
    h.app.dialog.as_mut().unwrap().specs[0].reverse = true;
    let a = params(&h)["axis"].clone();
    assert_eq!(a["dir"], json!([0.0, -1.0, 0.0]), "reversed: the line the other way: {a}");
    h.app.dialog.as_mut().unwrap().specs[0].reverse = false;
    // A coordinate axis chosen by Type.
    {
        let mut d = h.app.dialog.take().unwrap();
        assert!(crate::specifier::choose(&h.app, &mut d, 0, "axis"));
        d.specs[0].values = vec!["X".into()];
        h.app.dialog = Some(d);
    }
    let a = params(&h)["axis"].clone();
    assert_eq!(a["dir"], json!([1.0, 0.0, 0.0]), "{a}");
    assert_eq!(a["origin"], json!([0.0, 0.0, 0.0]));
}

#[test]
fn start_and_end_are_the_commands_sides_symmetric_and_to_object() {
    let mut h = open(ModellingApproach::Advanced);
    pick_axis(&mut h, Sel::Axis { name: "Y".into() });
    h.frames(3);
    h.step(&json!({"click":{"handle":"dialog:Start"}})).unwrap();
    h.step(&json!({"click":{"handle":"dialog:Start:Value (Other Side)"}})).unwrap();
    h.frames(3);
    assert!(params(&h).get("angle2").is_some(), "two sides: {}", params(&h));
    h.step(&json!({"click":{"handle":"dialog:Start"}})).unwrap();
    h.step(&json!({"click":{"handle":"dialog:Start:Symmetric Value"}})).unwrap();
    h.frames(3);
    let p = params(&h);
    assert!(p["direction"] == "symmetric" && p.get("angle2").is_none(), "{p}");
    h.step(&json!({"click":{"handle":"dialog:End"}})).unwrap();
    h.step(&json!({"click":{"handle":"dialog:End:Until Selected"}})).unwrap();
    h.frames(3);
    let d = h.app.dialog.as_ref().unwrap();
    assert!(d.inputs.iter().any(|i| i.label == "To Object"), "Until Selected asks for the object");
    assert!(d.extra.get("direction").is_none(), "no sides when it turns to an object");
}

#[test]
fn an_existing_revolve_opens_with_its_axis_and_reapplies_the_same() {
    let mut h = Harness::new();
    h.app.preferences.modelling_approach = ModellingApproach::Advanced;
    h.app.run("sketch.create", json!({"plane":"XY","name":"Base"})).unwrap();
    h.app.run("sketch.rectangle.two_point", json!({"p0":[10,0],"p1":[20,10]})).unwrap();
    h.app.run("sketch.finish", json!({})).unwrap();
    h.app.run("solid.revolve", json!({"sketch":"Base","axis":"Y","angle":"90 deg","operation":"new","body_name":"Ring"})).unwrap();
    h.frames(2);
    let id = h.app.session.doc.features.iter().find(|f| f.kind.type_name().contains("Revolve")).map(|f| f.id).unwrap();
    h.app.edit_feature(id);
    h.frames(4);
    let p = params(&h)["params"].clone();
    assert_eq!((p["axis"].clone(), p["angle"].clone()), (json!("Y"), json!("90 deg")), "{p}");
    assert!(matches!(h.app.dialog.as_ref().unwrap().kind, Kind::Revolve { .. }));
}
