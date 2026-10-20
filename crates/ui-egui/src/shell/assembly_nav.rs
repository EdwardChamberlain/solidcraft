//! The Assembly Navigator (Advanced approach): the components of the part on show, each one a
//! part file linked in (or a component kept inside this file), with where its file is, whether
//! the file is there, how many times it is placed, and what to do with it.

use egui::RichText;
use serde_json::json;

use crate::SolveApp;
use crate::theme::Tokens;

/// One component as the navigator lists it.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub component: u64,
    pub name: String,
    pub depth: usize,
    /// The part file's address as the link stores it, for a linked component.
    pub path: Option<String>,
    /// The part file, resolved against this design's folder.
    pub file: Option<String>,
    /// The part file exists (always true for a component kept inside this file).
    pub found: bool,
    /// Placements of the component.
    pub count: usize,
    pub grounded: bool,
    /// The component's placements in the design itself (their ids), for Fix and Show/Hide.
    pub placements: Vec<u64>,
    /// Every placement of it is hidden.
    pub hidden: bool,
}

/// The components of the design, parents before their children.
pub fn rows(app: &SolveApp) -> Vec<Row> {
    let doc = &app.session.doc;
    // A link's path is relative to this design's folder once the design has a file.
    let folder = app.session.path.as_deref().and_then(|p| std::path::Path::new(p).parent().map(std::path::Path::to_path_buf));
    let linked = |component: u64| -> Option<(String, String, bool)> {
        let link = doc.links.get(&component)?;
        let given = std::path::Path::new(&link.path);
        let file = match &folder {
            Some(dir) if given.is_relative() => dir.join(given),
            _ => given.to_path_buf(),
        };
        let shown = file.display().to_string();
        let found = solidcraft_engine::io::vfs::exists(&shown);
        Some((link.path.clone(), shown, found))
    };
    let doc_hidden = &app.session.visibility.hidden_occurrences;
    // Depth first, each component before its own children, siblings in file order. Bounded, as a
    // design file is hostile input: a parent cycle can't loop it.
    let mut out = Vec::new();
    let mut stack: Vec<(u64, usize)> = doc.components.iter().filter(|c| c.parent == 0).rev().map(|c| (c.id, 0)).collect();
    while let Some((id, depth)) = stack.pop() {
        if out.len() >= 4096 || depth > 32 {
            continue;
        }
        let Some(c) = doc.components.iter().find(|c| c.id == id) else { continue };
        let link = linked(id);
        let placements: Vec<_> = doc.occurrences.iter().filter(|o| o.component == id).collect();
        out.push(Row {
            component: id,
            name: c.name.clone(),
            depth,
            found: link.as_ref().is_none_or(|(_, _, found)| *found),
            path: link.as_ref().map(|(path, _, _)| path.clone()),
            file: link.map(|(_, file, _)| file),
            count: placements.len(),
            grounded: placements.iter().any(|o| o.grounded),
            // Only placements directly in the design have a path of their own to hide.
            hidden: c.parent == 0
                && !placements.is_empty()
                && placements.iter().all(|o| doc_hidden.iter().any(|path| path.len() == 1 && path.first() == Some(&o.id))),
            placements: placements.iter().map(|o| o.id).collect(),
        });
        stack.extend(doc.components.iter().filter(|k| k.parent == id).rev().map(|k| (k.id, depth + 1)));
    }
    out
}

pub fn show(app: &mut SolveApp, ui: &mut egui::Ui) {
    let t = Tokens::get();
    egui::Panel::left("sc_assembly_navigator")
        .exact_size(300.0)
        .resizable(false)
        .frame(egui::Frame::NONE.fill(t.panel).inner_margin(egui::Margin::symmetric(6, 6)))
        .show(ui, |ui| {
            ui.label(RichText::new(app.tr("ui.assembly_nav_heading")).size(11.0).strong().color(t.text_dim));
            ui.horizontal_wrapped(|ui| {
                if ui.button(app.tr("ui.add_component")).clicked() {
                    app.start("component.insert_design");
                }
                if ui.button(app.tr("ui.new_component")).clicked() {
                    app.start("component.new_part");
                }
                let update = ui.add_enabled(!app.session.doc.links.is_empty(), egui::Button::new(app.tr("ui.update_parts")));
                crate::scenario::publish_rect("assembly_nav:update", update.rect);
                if update.clicked() {
                    match crate::documents::with_open(app, |app| app.run("component.update_links", json!({}))) {
                        Ok(v) => {
                            let n = v.get("updated").and_then(|u| u.as_array()).map_or(0, Vec::len);
                            app.set_status(crate::i18n::format("ui.parts_updated", &[("count", &n.to_string())]), false);
                        }
                        Err(e) => app.set_status(e, true),
                    }
                }
                let save = ui.button(app.tr("ui.save_all"));
                crate::scenario::publish_rect("assembly_nav:save_all", save.rect);
                if save.clicked() {
                    let r = crate::documents::save_all(app);
                    let error = !r.failed.is_empty();
                    let text = if error {
                        r.failed.join("; ")
                    } else {
                        crate::i18n::format("ui.saved_all", &[("saved", &r.saved.to_string()), ("untitled", &r.untitled.len().to_string())])
                    };
                    app.set_status(text, error);
                }
            });
            ui.separator();
            let rows = rows(app);
            let design = app.session.doc.name.clone();
            ui.label(RichText::new(design).strong());
            if rows.is_empty() {
                ui.label(RichText::new(app.tr("ui.assembly_nav_empty")).color(t.text_dim));
            }
            egui::ScrollArea::vertical().id_salt("sc_assembly_nav_rows").auto_shrink([false, false]).show(ui, |ui| {
                for row in &rows {
                    one_row(app, ui, row);
                }
            });
        });
}

fn one_row(app: &mut SolveApp, ui: &mut egui::Ui, row: &Row) {
    let t = Tokens::get();
    ui.horizontal_wrapped(|ui| {
        ui.add_space(8.0 + row.depth as f32 * 14.0);
        let status = match (&row.path, row.found) {
            (None, _) => "ui.component_inside",
            (Some(_), true) => "ui.component_linked",
            (Some(_), false) => "ui.component_missing",
        };
        let colour = if !row.found {
            t.error
        } else if row.hidden {
            t.text_dim
        } else {
            t.text
        };
        let label = ui.add(egui::Label::new(RichText::new(&row.name).color(colour)).sense(egui::Sense::click()));
        crate::scenario::publish_rect(&format!("assembly_nav:row:{}", row.name), label.rect);
        let mut hover = app.tr(status).to_string();
        if let Some(path) = &row.path {
            hover = format!("{hover}: {path}");
        }
        let label = label.on_hover_text(hover);
        ui.label(RichText::new(format!("×{}", row.count)).size(11.0).color(t.text_dim));
        if row.grounded {
            ui.label(RichText::new(app.tr("ui.component_fixed")).size(11.0).color(t.text_dim));
        }
        if row.hidden {
            ui.label(RichText::new(app.tr("ui.component_hidden")).size(11.0).color(t.text_dim));
        }
        let mut open = false;
        if row.file.is_some() && row.found {
            let button = ui.small_button(app.tr("ui.open_part"));
            crate::scenario::publish_rect(&format!("assembly_nav:open:{}", row.name), button.rect);
            open |= button.clicked();
            // The same part, with the rest of the assembly around it.
            let work = ui.small_button(app.tr("ui.make_work_part"));
            crate::scenario::publish_rect(&format!("assembly_nav:work:{}", row.name), work.rect);
            if work.clicked() {
                work_part_of(app, row.component);
            }
        }
        // The same and the rest, on a right click.
        label.context_menu(|ui| {
            if row.file.is_some() && row.found && ui.button(app.tr("ui.make_work_part")).clicked() {
                work_part_of(app, row.component);
                ui.close();
            }
            if let Some(file) = &row.file
                && row.found
                && ui.button(app.tr("ui.open_part_displayed")).clicked()
            {
                app.open_path(file);
                ui.close();
            }
            let (show_hide, to_visible) = if row.hidden { ("ui.show_component", true) } else { ("ui.hide_component", false) };
            if !row.placements.is_empty() && ui.button(app.tr(show_hide)).clicked() {
                let paths: Vec<Vec<u64>> = row.placements.iter().map(|id| vec![*id]).collect();
                if let Err(e) = app.run("browser.visibility", json!({"visible": to_visible, "paths": paths})) {
                    app.set_status(e, true);
                }
                ui.close();
            }
            let fix = if row.grounded { "ui.free_component" } else { "ui.fix_component" };
            if !row.placements.is_empty() && ui.button(app.tr(fix)).clicked() {
                for id in &row.placements {
                    if let Err(e) = app.run("occurrence.ground", json!({"occurrence": id, "grounded": !row.grounded})) {
                        app.set_status(e, true);
                        break;
                    }
                }
                ui.close();
            }
            if let Some(first) = row.placements.first().copied() {
                if ui.button("Pattern Component…").clicked() {
                    app.start("occurrence.pattern");
                    // The component is picked: its first body stands for it.
                    let body = app.session.occurrence_bodies(&[first], true).ok().and_then(|b| b.into_iter().next());
                    if let (Some(body), Some(d)) = (body, app.dialog.as_mut())
                        && let Some(input) = d.inputs.first_mut()
                    {
                        input.items = vec![solidcraft_engine::Sel::Body { name: body }];
                        d.advance();
                    }
                    ui.close();
                }
                // Another component in its place, keeping where it is, whether it is fixed and its joints.
                let others: Vec<(u64, String)> = app
                    .session
                    .doc
                    .components
                    .iter()
                    .filter(|c| c.id != row.component)
                    .filter(|c| {
                        row.placements.iter().all(|id| {
                            app.session
                                .doc
                                .occurrences
                                .iter()
                                .find(|o| o.id == *id)
                                .is_some_and(|o| !app.session.doc.component_within(o.parent, c.id))
                        })
                    })
                    .map(|c| (c.id, c.name.clone()))
                    .collect();
                if !others.is_empty() {
                    let r = ui.menu_button("Replace With", |ui| {
                        for (id, name) in &others {
                            let r = ui.button(name);
                            crate::scenario::publish_handle(&format!("assembly_nav:replace:{name}"), r.rect.center());
                            if r.clicked() {
                                if let Err(e) = app.run("component.replace", json!({"component": row.component, "with": id})) {
                                    app.set_status(e, true);
                                }
                                ui.close();
                            }
                        }
                    });
                    crate::scenario::publish_handle(&format!("assembly_nav:replace:{}", row.name), r.response.rect.center());
                }
            }
            if row.path.is_some() && ui.button(app.tr("ui.break_link")).clicked() {
                if let Err(e) = app.run("component.break_link", json!({"component": row.component})) {
                    app.set_status(e, true);
                }
                ui.close();
            }
        });
        if open && let Some(file) = &row.file {
            app.open_path(file);
        }
    });
    ui.add_space(2.0);
}

/// Edit a component's part with the assembly around it; say why when that can't be done.
fn work_part_of(app: &mut SolveApp, component: u64) {
    if let Err(e) = crate::work_part::make_work_part(app, component) {
        app.set_status(e, true);
    }
}
