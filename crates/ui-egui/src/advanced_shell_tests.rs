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
    // The datum panel offers every construct method the engine has.
    let datum = &panel_lists("HOME")[1].1;
    let engine = solidcraft_engine::command_specs().iter().filter(|c| c.id.starts_with("construct.")).count();
    assert_eq!(datum.len(), engine, "HOME/DATUM and the engine's construct.* commands differ");
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
    use crate::shell::part_nav::{children_of, facts};
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
    let (deleted, _) = children_of(&h.app, sketch);
    assert!(deleted.contains(&"Boss".to_string()), "{deleted:?}");
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
    assert!((volume(&mut h) - 1000.0).abs() < 1e-6, "the placeholder box");
    h.app.open_path(&dir.join("Lid.solidcraft").display().to_string());
    h.frames(2);
    assert_eq!(h.app.docs.count(), 2);
    h.app.run("solid.box", json!({"length": 10, "width": 10, "height": 10, "body_name": "Extra", "corner": [50, 0, 0]})).unwrap();
    let r = crate::documents::save_all(&mut h.app);
    assert_eq!((r.saved, r.untitled.len(), r.failed.len()), (1, 0, 0), "{r:?}");
    assert_eq!(r.updated, 1, "the assembly took the part's new body");
    crate::documents::control(&mut h.app, &json!({"action":"switch","index":0}));
    h.frames(2);
    assert!((volume(&mut h) - 2000.0).abs() < 1e-6, "{}", volume(&mut h));
    // The update changed the assembly, so the next Save All writes it; then nothing is left.
    assert_eq!(crate::documents::save_all(&mut h.app).saved, 1);
    assert_eq!(crate::documents::save_all(&mut h.app).saved, 0);
    let _ = std::fs::remove_dir_all(&dir);
}
