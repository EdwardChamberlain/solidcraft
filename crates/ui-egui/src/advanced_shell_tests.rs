//! Advanced approach: its own ribbon tabs, built from the same commands.
use crate::prefs::ModellingApproach;
use crate::scenario::{Harness, rect_of};
use crate::toolbar_custom::panel_lists;
use crate::workspace::ADVANCED_TABS;
use serde_json::json;

#[test]
fn every_advanced_ribbon_command_exists_and_every_tab_has_panels() {
    for tab in ADVANCED_TABS {
        let lists = panel_lists(tab);
        assert!(!lists.is_empty(), "{tab} has no panels");
        for (panel, cmds, promoted) in lists {
            assert!(!cmds.is_empty(), "{tab}/{panel} lists no commands");
            assert!(promoted >= 1 && promoted <= cmds.len(), "{tab}/{panel}: {promoted} buttons of {}", cmds.len());
        }
    }
    let feature = &panel_lists("HOME")[0].1;
    for id in ["sketch.create", "solid.extrude", "solid.revolve", "solid.fillet", "solid.thread"] {
        assert!(feature.iter().any(|c| c.id == id), "HOME/FEATURE lacks {id}");
    }
    // The datum panel is a plane, an axis and a point; every construct method the engine has is in
    // one of their drop-downs.
    let datum = &panel_lists("HOME")[1].1;
    assert_eq!(datum.iter().map(|c| c.id).collect::<Vec<_>>(), ["datum.plane", "datum.axis", "datum.point"]);
    for c in solidcraft_engine::command_specs().iter().filter(|c| c.id.starts_with("construct.")) {
        assert!(crate::sketch_families::DATUM_FAMILIES.iter().any(|f| f.commands.contains(&c.id)), "{} is in no datum drop-down", c.id);
    }
}

#[test]
fn the_advanced_ribbon_shows_its_tabs_and_classic_keeps_its_own() {
    let mut h = Harness::new();
    h.frames(3);
    assert!(rect_of("toolbar_tab:SOLID").is_some() && rect_of("toolbar_tab:HOME").is_none());
    h.app.preferences.modelling_approach = ModellingApproach::Advanced;
    h.frames(3);
    assert_eq!(h.app.ui.tab, "HOME");
    for tab in ADVANCED_TABS {
        assert!(rect_of(&format!("toolbar_tab:{tab}")).is_some(), "no {tab} tab");
    }
    assert!(rect_of("toolbar_tab:SOLID").is_none());
    // A sketch adds the contextual tab and leaving it returns to Home.
    h.app.run("sketch.create", json!({"plane":"XY"})).unwrap();
    h.frames(3);
    assert_eq!(h.app.ui.tab, "SKETCH");
    h.app.run("sketch.finish", json!({})).unwrap();
    h.frames(3);
    assert_eq!(h.app.ui.tab, "HOME");
    h.app.preferences.modelling_approach = ModellingApproach::Classic;
    h.frames(3);
    assert_eq!(h.app.ui.tab, "SOLID");
}

fn advanced() -> Harness {
    let mut h = Harness::new();
    h.app.preferences.modelling_approach = ModellingApproach::Advanced;
    // A design made in this approach records it (File > New does this for the real app).
    h.app.session.doc_mut().approach = solidcraft_engine::doc::Approach::Advanced;
    h.frames(3);
    h
}

#[test]
fn the_cue_line_follows_the_dialog_and_the_sketch() {
    let mut h = advanced();
    assert!(rect_of("cue_line").is_some() && rect_of("navigator:part").is_some());
    h.app.run("sketch.create", json!({"plane":"XY","name":"Base"})).unwrap();
    h.frames(2);
    assert!(
        crate::shell::advanced::cue_text(&h.app).contains("degrees of freedom")
            || crate::shell::advanced::cue_text(&h.app).contains("fully constrained")
    );
    h.app.run("sketch.rectangle.two_point", json!({"p0":[0,0],"p1":[10,10]})).unwrap();
    h.app.run("sketch.finish", json!({})).unwrap();
    h.app.start("solid.extrude");
    h.frames(2);
    let cue = crate::shell::advanced::cue_text(&h.app);
    assert!(cue.contains("Select") || cue.contains("OK"), "{cue}");
    h.app.dialog = None;
    h.app.start("construct.plane.three_points");
    h.frames(2);
    assert!(crate::shell::advanced::cue_text(&h.app).starts_with("1 of 3"), "{}", crate::shell::advanced::cue_text(&h.app));
}

#[test]
fn the_navigator_tabs_switch_the_pane_and_hide_it() {
    use crate::shell::advanced::{Pane, pane};
    let mut h = advanced();
    assert_eq!(pane(&h.ctx), Pane::Part);
    h.step(&json!({"widget":"Assembly Navigator"})).unwrap();
    h.frames(2);
    assert_eq!(pane(&h.ctx), Pane::Assembly);
    h.step(&json!({"widget":"Assembly Navigator"})).unwrap();
    h.frames(2);
    assert!(!h.app.ui.show_browser, "clicking the open tab hides the pane");
}

#[test]
fn the_part_navigator_reports_a_features_parents_children_and_values() {
    use crate::shell::part_nav::facts;
    let mut h = advanced();
    h.app.run("sketch.create", json!({"plane":"XY","name":"Base"})).unwrap();
    h.app.run("sketch.rectangle.two_point", json!({"p0":[0,0],"p1":[10,10]})).unwrap();
    h.app.run("sketch.finish", json!({})).unwrap();
    h.app.run("solid.extrude", json!({"sketch":"Base","distance":"10 mm","operation":"new","name":"Boss"})).unwrap();
    let sketch = h.app.session.doc.find_feature("Base").unwrap().id;
    let boss = h.app.session.doc.find_feature("Boss").unwrap().id;
    let f = facts(&h.app, boss).unwrap();
    assert_eq!((f.name.as_str(), f.parents.as_slice()), ("Boss", &["Base".to_string()][..]));
    assert!(f.inputs.iter().any(|(_, e)| e == "10 mm"), "{:?}", f.inputs);
    let children = facts(&h.app, sketch).unwrap().children;
    assert!(children.contains(&"Boss".to_string()), "{children:?}");
    // The panes draw for a selected feature without a panic.
    h.app.session.selection = vec![solidcraft_engine::Sel::Feature { id: boss }];
    h.frames(3);
    assert!(rect_of("part_nav:edit").is_some());
    h.app.session.selection.clear();
    h.frames(2);
    assert!(rect_of("part_nav:edit").is_none());
}

#[test]
fn the_assembly_navigator_lists_linked_parts_with_their_state_and_opens_them() {
    use crate::shell::advanced::{Pane, set_pane};
    use crate::shell::assembly_nav::rows;
    let dir = std::env::temp_dir().join(format!("solidcraft-asm-nav-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut h = advanced();
    h.app.run("file.save", json!({"path": dir.join("case.solidcraft")})).unwrap();
    h.app.run("component.new_part", json!({"name": "Lid"})).unwrap();
    h.app.run("component.new_part", json!({"name": "Base"})).unwrap();
    let list = rows(&h.app);
    assert_eq!(list.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(), ["Lid", "Base"]);
    assert!(list.iter().all(|r| r.found && r.count == 1 && r.path.as_deref().is_some_and(|p| p.ends_with(".solidcraft"))), "{list:?}");
    set_pane(&h.ctx, Pane::Assembly);
    h.frames(3);
    assert!(rect_of("assembly_nav:row:Lid").is_some() && rect_of("assembly_nav:update").is_some());
    // A part file that has gone shows as missing.
    std::fs::remove_file(dir.join("Base.solidcraft")).unwrap();
    let list = rows(&h.app);
    assert!(list[0].found && !list[1].found, "{list:?}");
    h.frames(2);
    assert!(rect_of("assembly_nav:open:Lid").is_some() && rect_of("assembly_nav:open:Base").is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_new_design_records_its_approach_and_opening_one_switches_to_it() {
    let dir = std::env::temp_dir().join(format!("solidcraft-adopt-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("part.solidcraft");
    let mut h = advanced();
    h.app.run("file.new", json!({})).unwrap();
    assert_eq!(h.app.session.doc.approach, solidcraft_engine::doc::Approach::Advanced);
    h.app.run("file.save", json!({"path": file})).unwrap();
    h.app.preferences.modelling_approach = ModellingApproach::Classic;
    h.app.run("file.new", json!({})).unwrap();
    assert_eq!(h.app.session.doc.approach, solidcraft_engine::doc::Approach::Classic);
    h.app.run("doc.open", json!({"path": file})).unwrap();
    assert_eq!(h.app.preferences.modelling_approach, ModellingApproach::Advanced, "opening an Advanced design switches the window to it");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn save_all_writes_every_open_design_and_brings_assemblies_up_to_date() {
    let dir = std::env::temp_dir().join(format!("solidcraft-save-all-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut h = advanced();
    // An assembly with a part, saved; the part is then opened in its own tab and changed.
    h.app.run("file.save", json!({"path": dir.join("case.solidcraft")})).unwrap();
    h.app.run("component.new_part", json!({"name": "Lid"})).unwrap();
    h.app.run("file.save", json!({})).unwrap();
    let volume = |h: &mut Harness| h.app.session.execute("inspect.measure", &json!({})).unwrap()["total"]["volume_mm3"].as_f64().unwrap();
    assert!(h.app.session.world_state().bodies.is_empty(), "a new part has nothing in it yet");
    h.app.open_path(&dir.join("Lid.solidcraft").display().to_string());
    h.frames(2);
    assert_eq!(h.app.docs.count(), 2);
    h.app.run("solid.box", json!({"length": 10, "width": 10, "height": 10, "body_name": "Extra", "corner": [50, 0, 0]})).unwrap();
    let r = crate::documents::save_all(&mut h.app);
    assert_eq!((r.saved, r.untitled.len(), r.failed.len()), (1, 0, 0), "{r:?}");
    assert_eq!(r.updated, 1, "the assembly took the part's new body");
    crate::documents::control(&mut h.app, &json!({"action":"switch","index":0}));
    h.frames(2);
    assert!((volume(&mut h) - 1000.0).abs() < 1e-6, "{}", volume(&mut h));
    // The update changed the assembly, so the next Save All writes it; then nothing is left.
    assert_eq!(crate::documents::save_all(&mut h.app).saved, 1);
    assert_eq!(crate::documents::save_all(&mut h.app).saved, 0);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn autosave_keeps_every_open_design_with_unsaved_changes_not_only_the_one_shown() {
    let dir = std::env::temp_dir().join(format!("solidcraft-tabs-autosave-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut h = advanced();
    crate::recovery_ui::start(&mut h.app, &dir.join("recovery"), std::time::Duration::ZERO);
    h.app.run("solid.box", json!({"length": 10, "width": 10, "height": 10})).unwrap();
    crate::documents::new_tab(&mut h.app);
    h.frames(2);
    h.app.run("solid.box", json!({"length": 5, "width": 5, "height": 5})).unwrap();
    assert_eq!(h.app.docs.count(), 2);
    crate::recovery_ui::tick(&mut h.app);
    let entries = std::fs::read_dir(dir.join("recovery")).unwrap().flatten().filter(|e| e.path().extension().is_some_and(|x| x == "json")).count();
    assert_eq!(entries, 2, "both tabs are autosaved");
    // Closing the app with both unsaved keeps both for the next launch.
    crate::recovery_ui::close(&mut h.app);
    assert_eq!(solidcraft_engine::recovery::orphans(&dir.join("recovery")).len(), 2);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn switching_to_an_assembly_shows_the_part_open_in_another_tab_as_it_is_now() {
    let dir = std::env::temp_dir().join(format!("solidcraft-live-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut h = advanced();
    h.app.run("file.save", json!({"path": dir.join("case.solidcraft")})).unwrap();
    h.app.run("component.new_part", json!({"name": "Lid"})).unwrap();
    h.app.run("file.save", json!({})).unwrap();
    let volume = |h: &mut Harness| h.app.session.execute("inspect.measure", &json!({})).unwrap()["total"]["volume_mm3"].as_f64().unwrap();
    assert!(h.app.session.world_state().bodies.is_empty());
    // The part in its own tab gets a body; it is NOT saved.
    h.app.open_path(&dir.join("Lid.solidcraft").display().to_string());
    h.frames(2);
    h.app.run("solid.box", json!({"length": 10, "width": 10, "height": 10, "body_name": "Extra", "corner": [50, 0, 0]})).unwrap();
    assert!(h.app.session.is_dirty());
    // Back to the assembly: it already has the part's new body.
    crate::documents::switch(&mut h.app, 0);
    h.frames(2);
    assert_eq!(h.app.session.doc.name, "case");
    assert!((volume(&mut h) - 1000.0).abs() < 1e-6, "{}", volume(&mut h));
    // The part's file is untouched.
    let mut on_disk = solidcraft_engine::Session::default();
    on_disk.execute("doc.open", &json!({"path": dir.join("Lid.solidcraft")})).unwrap();
    assert!(on_disk.doc.features.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn classic_keeps_a_link_until_update_links_is_asked() {
    let dir = std::env::temp_dir().join(format!("solidcraft-live-classic-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut h = Harness::new();
    h.app.run("file.save", json!({"path": dir.join("case.solidcraft")})).unwrap();
    h.app.run("component.new_part", json!({"name": "Lid"})).unwrap();
    h.app.run("file.save", json!({})).unwrap();
    h.app.open_path(&dir.join("Lid.solidcraft").display().to_string());
    h.frames(2);
    h.app.run("solid.box", json!({"length": 10, "width": 10, "height": 10, "body_name": "Extra", "corner": [50, 0, 0]})).unwrap();
    crate::documents::switch(&mut h.app, 0);
    h.frames(2);
    assert!(h.app.session.world_state().bodies.is_empty(), "Classic did not update on its own");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_part_navigator_starts_with_the_origin_and_picking_a_plane_there_fills_the_datum_dialog() {
    let mut h = advanced();
    assert!(crate::scenario::handle_at("row:Origin").is_some(), "the Origin row heads the tree");
    assert!(crate::scenario::handle_at("row:XY").is_none(), "folded until opened");
    h.app.tree.expanded.insert("origin".into());
    h.frames(3);
    assert!(crate::scenario::handle_at("row:XY").is_some(), "the origin planes are listed");
    h.app.start("construct.plane.offset");
    h.frames(3);
    // Clicking the row goes through the same path a pick in the viewport takes.
    h.step(&json!({"click": {"handle": "row:XY"}})).unwrap();
    h.frames(3);
    let picked = h.app.dialog.as_ref().map(|d| d.inputs[0].items.clone()).unwrap_or_default();
    assert_eq!(picked, vec![solidcraft_engine::Sel::Plane { name: "XY".into() }]);
}

#[test]
fn the_resource_bar_has_a_tab_for_each_navigator_and_a_context_menu_fixes_a_component() {
    let dir = std::env::temp_dir().join(format!("solidcraft-resource-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut h = advanced();
    for tab in ["part", "assembly"] {
        assert!(rect_of(&format!("navigator:{tab}")).is_some(), "no {tab} tab on the resource bar");
    }
    h.app.run("file.save", json!({"path": dir.join("case.solidcraft")})).unwrap();
    h.app.run("component.new_part", json!({"name": "Lid"})).unwrap();
    let rows = crate::shell::assembly_nav::rows(&h.app);
    assert!(!rows[0].grounded && !rows[0].hidden && rows[0].placements.len() == 1);
    // What the menu runs.
    let occurrence = rows[0].placements[0];
    h.app.run("occurrence.ground", json!({"occurrence": occurrence, "grounded": true})).unwrap();
    h.app.run("browser.visibility", json!({"visible": false, "paths": [[occurrence]]})).unwrap();
    let rows = crate::shell::assembly_nav::rows(&h.app);
    assert!(rows[0].grounded && rows[0].hidden, "{rows:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn feature_types_read_as_words_and_every_datum_dialog_is_headed_datum() {
    use crate::shell::part_nav::friendly_kind;
    assert_eq!(friendly_kind("ExtrudeFeature"), "Extrude");
    assert_eq!(friendly_kind("SurfaceExtrudeFeature"), "Surface Extrude");
    assert_eq!(friendly_kind("Sketch"), "Sketch");
    let mut h = advanced();
    for (cmd, title) in [("construct.plane.offset", "DATUM PLANE"), ("construct.axis.edge", "DATUM AXIS"), ("construct.point.vertex", "DATUM POINT")]
    {
        h.app.dialog = None;
        h.app.start(cmd);
        h.frames(2);
        let kind = &h.app.dialog.as_ref().unwrap().kind;
        assert_eq!(crate::dialogs_datum::class_title(kind), Some(title), "{cmd}");
    }
}

#[test]
fn a_new_part_asks_for_the_design_to_be_saved_first() {
    let mut h = advanced();
    h.app.start("component.new_part");
    h.frames(2);
    assert!(h.app.dialog.is_none(), "no dialog while there is nowhere to put the part");
    assert!(h.app.status.as_ref().is_some_and(|(text, _, error)| *error && text.contains("Save this design first")), "{:?}", h.app.status);
}

#[test]
fn the_sketch_ribbon_shows_sketch_dimension_as_a_button_in_both_approaches() {
    for approach in [ModellingApproach::Advanced, ModellingApproach::Classic] {
        let mut h = Harness::new();
        h.app.preferences.modelling_approach = approach;
        h.app.run("sketch.create", json!({"plane":"XY"})).unwrap();
        h.frames(6);
        assert!(h.widget("Sketch Dimension").is_some(), "{approach:?}: the dimension tool is on the Sketch ribbon");
        h.step(&json!({"widget":"Sketch Dimension"})).unwrap();
        h.frames(3);
        assert_eq!(h.app.tool.as_ref().map(|t| t.cmd), Some("sketch.dimension"), "{approach:?}: the button starts the dimension tool");
    }
}

#[test]
fn the_pattern_panel_is_pattern_and_mirror() {
    let mut h = advanced();
    h.frames(6);
    for button in ["Pattern", "Mirror"] {
        assert!(h.widget(button).is_some(), "{button} is a button on the Home ribbon");
    }
    for other in ["Pattern Feature", "Pattern Geometry", "Rectangular Pattern", "Circular Pattern", "Pattern on Path"] {
        assert!(h.widget(other).is_none(), "{other} is not on the ribbon");
    }
    let lists = panel_lists("HOME");
    let pattern = lists.iter().find(|(p, ..)| *p == "PATTERN").expect("the Pattern panel");
    assert_eq!(pattern.1.iter().map(|c| c.id).collect::<Vec<_>>(), ["solid.pattern", "solid.mirror"], "nothing else is on offer");
}

#[test]
fn pattern_chooses_its_layout_and_what_it_repeats_in_the_dialog() {
    use crate::dialogs::Kind;
    use crate::selection::{BODIES, FEATURES};
    let mut h = advanced();
    h.app.start("solid.pattern");
    h.frames(4);
    assert!(h.widget("Layout").is_some() && h.widget("Object Type").is_some(), "both are rows of the dialog");
    let d = h.app.dialog.as_ref().unwrap();
    assert!(matches!(d.kind, Kind::PatternRect { .. }) && d.inputs[0].accept == BODIES, "Linear bodies to start with");
    h.step(&json!({"click":{"handle":"dialog:Object Type"}})).unwrap();
    h.step(&json!({"click":{"handle":"dialog:Object Type:Features"}})).unwrap();
    h.frames(3);
    assert_eq!(h.app.dialog.as_ref().unwrap().inputs[0].accept, FEATURES);
    h.step(&json!({"click":{"handle":"dialog:Layout"}})).unwrap();
    h.step(&json!({"click":{"handle":"dialog:Layout:Circular"}})).unwrap();
    h.frames(4);
    let d = h.app.dialog.as_ref().unwrap();
    assert!(matches!(d.kind, Kind::PatternCirc { .. }));
    assert_eq!(d.inputs[0].accept, FEATURES, "still features");
}

#[test]
fn finish_sketch_is_first_in_advanced_with_reattach_beside_it_and_last_in_classic() {
    for approach in [ModellingApproach::Advanced, ModellingApproach::Classic] {
        let mut h = Harness::new();
        h.app.preferences.modelling_approach = approach;
        h.app.run("sketch.create", json!({"plane":"XY"})).unwrap();
        h.frames(6);
        let panel = |name: &str| rect_of(&format!("panel:SKETCH:{name}")).unwrap_or_else(|| panic!("{approach:?}: no {name} panel"));
        let (finish, create) = (panel("FINISH SKETCH"), panel("CREATE"));
        match approach {
            ModellingApproach::Advanced => {
                assert!(finish.right() <= create.left() + 1.0, "Finish Sketch is at the left: {finish:?} {create:?}");
                assert!(finish.width() > 60.0, "two buttons: Finish Sketch and Reattach Sketch: {finish:?}");
            }
            _ => {
                assert!(finish.left() >= create.right() - 1.0, "Classic keeps it at the right: {finish:?} {create:?}");
                assert!(h.widget("Reattach Sketch").is_none());
            }
        }
    }
}

#[test]
fn the_sketch_ribbon_has_one_finish_button_and_the_others_are_in_its_drop_down() {
    let mut h = advanced();
    h.app.run("sketch.create", json!({"plane":"XY"})).unwrap();
    h.frames(6);
    let lists = panel_lists("SKETCH");
    let finish = lists.iter().find(|(p, ..)| *p == "FINISH SKETCH").expect("the Finish Sketch panel");
    assert!(finish.1.iter().any(|c| c.id == "sketch.finish_auto_constrain"), "finishing with auto constrain is in the panel");
    // Shown on the toolbar: Finish Sketch and Reattach Sketch only.
    assert!(h.widget("Finish with AutoConstrain").is_none(), "not a button");
    assert!(rect_of("panel:SKETCH:FINISH SKETCH").is_some_and(|r| r.width() < 130.0), "two buttons wide");
}

#[test]
fn the_datum_panel_has_a_button_each_for_plane_axis_and_point_with_the_ways_in_their_drop_downs() {
    let mut h = advanced();
    h.frames(6);
    for label in ["Datum Plane", "Datum Axis", "Datum Point"] {
        assert!(h.widget(label).is_some(), "{label} is a button");
    }
    for family in ["Plane", "Axis", "Point"] {
        assert!(crate::scenario::handle_at(&format!("family:{family}")).is_some(), "{family} has its drop-down arrow");
    }
    assert!(h.widget("Offset Plane").is_none() && h.widget("Midplane").is_none(), "the ways are not on the ribbon");
    // The button opens the one dialog, at Inferred; its Type lists the ways.
    h.step(&json!({"widget":"Datum Point"})).unwrap();
    h.frames(4);
    let d = h.app.dialog.as_ref().expect("the Datum Point dialog");
    assert!(matches!(d.kind, crate::dialogs::Kind::DatumInferred { .. }));
    assert_eq!(d.origin.as_deref(), Some("datum.point"));
}

#[test]
fn a_datum_drop_down_starts_the_dialog_at_that_way() {
    let mut h = advanced();
    h.frames(6);
    h.step(&json!({"click":{"handle":"family:Plane"}})).unwrap();
    h.frames(3);
    assert!(h.widget("Midplane").is_some(), "the Plane drop-down lists its ways");
    h.step(&json!({"widget":"Midplane"})).unwrap();
    h.frames(4);
    let d = h.app.dialog.as_ref().expect("a dialog");
    assert!(matches!(d.kind, crate::dialogs::Kind::Midplane), "{:?}", d.kind);
}
