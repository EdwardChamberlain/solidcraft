//! Mirror and Draft in the Advanced approach: the plane is a selector row with its own dialog.
use crate::dialogs::{Kind, apply_commands};
use crate::prefs::ModellingApproach;
use crate::scenario::Harness;
use serde_json::json;
use solidcraft_engine::Sel;

fn part(approach: ModellingApproach) -> Harness {
    let mut h = Harness::new();
    h.app.preferences.modelling_approach = approach;
    h.app.run("solid.box", json!({"length":10,"width":10,"height":10,"body_name":"Block"})).unwrap();
    h.frames(4);
    h
}

fn plane(name: &str) -> Sel {
    Sel::Plane { name: name.into() }
}

#[test]
fn the_mirror_plane_is_a_selector_row_in_groups_and_classic_keeps_its_input() {
    let mut h = part(ModellingApproach::Advanced);
    h.app.start("solid.mirror");
    h.frames(4);
    let d = h.app.dialog.as_ref().unwrap();
    assert_eq!(d.specs.len(), 1);
    assert_eq!(d.inputs[1].owner, 1, "the plane's input is the specifier's, in the same place");
    for g in ["mirror_objects", "mirror_plane", "mirror_settings"] {
        assert!(crate::scenario::handle_at(&format!("dialog:group:{g}")).is_some(), "{g}");
    }
    assert!(crate::scenario::handle_at("dialog:specify:mirror_plane_selector").is_some(), "the selector has its icon");
    let mut c = part(ModellingApproach::Classic);
    c.app.start("solid.mirror");
    c.frames(4);
    assert!(c.app.dialog.as_ref().unwrap().specs.is_empty());
}

#[test]
fn a_picked_plane_mirrors_as_before_and_a_plane_made_in_the_dialog_is_made_first() {
    let mut h = part(ModellingApproach::Advanced);
    h.app.start("solid.mirror");
    h.frames(4);
    {
        let d = h.app.dialog.as_mut().unwrap();
        d.inputs[0].items = vec![Sel::Body { name: "Block".into() }];
        d.inputs[1].items = vec![plane("YZ")];
    }
    let cmds = apply_commands(&h.app, h.app.dialog.as_ref().unwrap()).unwrap();
    assert_eq!((cmds.len(), cmds[0].0.as_str(), cmds[0].1["plane"].clone()), (1, "solid.mirror", json!("YZ")));
    // An offset plane, chosen in the plane's own dialog.
    {
        let mut d = h.app.dialog.take().unwrap();
        assert!(crate::specifier::choose(&h.app, &mut d, 0, "construct.plane.offset"));
        let at = d.inputs.iter().position(|i| i.owner == 1).unwrap();
        d.inputs[at].items = vec![plane("YZ")];
        *d.specs[0].inputs_value_mut().unwrap() = "15 mm".into();
        h.app.dialog = Some(d);
    }
    let cmds = apply_commands(&h.app, h.app.dialog.as_ref().unwrap()).unwrap();
    assert_eq!(cmds.len(), 2, "the plane, then the mirror");
    assert_eq!(cmds[0].0, "construct.plane.offset");
    assert_eq!(cmds[1].1["plane"], cmds[0].1["name"], "the mirror uses the plane by name");
    h.frames(6);
    h.step(&json!({"widget":"OK"})).unwrap();
    h.frames(4);
    assert!(h.app.session.doc.features.iter().any(|f| f.kind.type_name().contains("Mirror")), "the mirror was made");
    assert!(h.app.session.doc.features.iter().any(|f| matches!(f.kind, solidcraft_engine::doc::FeatureKind::ConstructionPlane { .. })));
}

#[test]
fn an_existing_mirror_opens_with_its_plane_and_a_made_plane_cannot_edit_it() {
    let mut h = part(ModellingApproach::Advanced);
    h.app.run("solid.mirror", json!({"bodies":["Block"],"plane":"YZ","combine":false})).unwrap();
    h.frames(2);
    let id = h.app.session.doc.features.iter().find(|f| f.kind.type_name().contains("Mirror")).map(|f| f.id).unwrap();
    h.app.edit_feature(id);
    h.frames(4);
    let d = h.app.dialog.as_ref().expect("the edit dialog");
    assert!(matches!(d.kind, Kind::Mirror) && !d.specs.is_empty());
    let cmds = apply_commands(&h.app, d).unwrap();
    assert_eq!(cmds[0].1["params"]["plane"], "YZ", "{:?}", cmds[0]);
    {
        let mut d = h.app.dialog.take().unwrap();
        crate::specifier::choose(&h.app, &mut d, 0, "construct.plane.offset");
        let at = d.inputs.iter().position(|i| i.owner == 1).unwrap();
        d.inputs[at].items = vec![plane("YZ")];
        h.app.dialog = Some(d);
    }
    let err = apply_commands(&h.app, h.app.dialog.as_ref().unwrap()).unwrap_err();
    assert!(err.contains("make the plane first"), "{err}");
}

#[test]
fn the_draft_neutral_plane_is_a_selector_row_too() {
    let mut h = part(ModellingApproach::Advanced);
    h.app.start("solid.draft");
    h.frames(4);
    let d = h.app.dialog.as_ref().unwrap();
    assert_eq!(d.specs.len(), 1);
    for g in ["draft_faces", "draft_plane", "draft_angle"] {
        assert!(crate::scenario::handle_at(&format!("dialog:group:{g}")).is_some(), "{g}");
    }
    let state = h.app.session.world_state();
    let side = state.body("Block").unwrap().body.faces(0.01).unwrap();
    let face = side.iter().find(|f| f.plane_normal.is_some_and(|n| n.x > 0.9)).expect("a side face");
    let sel = Sel::Face { body: "Block".into(), index: face.index, point: face.centroid };
    let d = h.app.dialog.as_mut().unwrap();
    d.inputs[0].items = vec![sel];
    d.inputs[1].items = vec![plane("XY")];
    let cmds = apply_commands(&h.app, h.app.dialog.as_ref().unwrap()).unwrap();
    assert_eq!((cmds[0].0.as_str(), cmds[0].1["neutral"].clone()), ("solid.draft", json!("XY")));
}

#[test]
fn the_other_advanced_dialogs_have_a_selection_and_a_settings_group_and_classic_none() {
    let groups = |approach: ModellingApproach, command: &str| {
        let mut h = part(approach);
        h.app.start(command);
        h.frames(4);
        ["selection", "settings"].map(|g| crate::scenario::handle_at(&format!("dialog:group:{g}")).is_some())
    };
    for command in [
        "solid.shell",
        "solid.rib",
        "solid.sweep",
        "solid.loft",
        "solid.combine",
        "solid.thread",
        "solid.scale",
        "solid.offset_face",
        "solid.pattern.circular",
    ] {
        assert_eq!(groups(ModellingApproach::Advanced, command), [true, true], "{command}");
        assert_eq!(groups(ModellingApproach::Classic, command), [false, false], "{command}");
    }
}
